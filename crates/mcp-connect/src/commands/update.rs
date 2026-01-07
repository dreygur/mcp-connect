//! Self-update functionality for mcp-connect CLI.

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env::consts::{ARCH, OS};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const GITHUB_RELEASES_API: &str = "https://api.github.com/repos/dreygur/mcp-connect/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const CHECK_INTERVAL_SECS: u64 = 86400; // 24 hours

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateState {
    last_check: u64,
    auto_update: bool,
}

impl Default for UpdateState {
    fn default() -> Self {
        Self {
            last_check: 0,
            auto_update: true, // enabled by default
        }
    }
}

impl UpdateState {
    fn load() -> Self {
        Self::state_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self) -> Result<()> {
        let path = Self::state_path().context("Cannot determine state path")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    fn state_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".mcp-connect").join("state.json"))
    }

    fn should_check(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now.saturating_sub(self.last_check) >= CHECK_INTERVAL_SECS
    }

    fn mark_checked(&mut self) {
        self.last_check = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }
}

/// Get platform-specific artifact name
fn get_artifact_name() -> Result<String> {
    let (os, ext) = match OS {
        "linux" => ("linux", "tar.gz"),
        "macos" => ("darwin", "tar.gz"),
        "windows" => ("windows", "zip"),
        _ => anyhow::bail!("Unsupported OS: {}", OS),
    };

    let arch = match ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => anyhow::bail!("Unsupported architecture: {}", ARCH),
    };

    Ok(format!("mcp-connect-{}-{}.{}", os, arch, ext))
}

/// Compare versions (simple semver comparison)
fn version_gt(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.').filter_map(|s| s.parse().ok()).collect()
    };
    parse(a) > parse(b)
}

/// Check for available updates (returns latest version if newer)
async fn fetch_latest_version(client: &Client) -> Result<Option<(String, Release)>> {
    let release: Release = client
        .get(GITHUB_RELEASES_API)
        .send()
        .await?
        .json()
        .await
        .context("Failed to parse release info")?;

    let latest = release.tag_name.trim_start_matches('v').to_string();
    if version_gt(&latest, CURRENT_VERSION) {
        Ok(Some((latest, release)))
    } else {
        Ok(None)
    }
}

/// Download and verify checksum
async fn download_with_checksum(client: &Client, url: &str, checksum_url: &str) -> Result<Vec<u8>> {
    let checksum_content = client.get(checksum_url).send().await?.text().await?;
    let expected_hash = checksum_content
        .split_whitespace()
        .next()
        .context("Invalid checksum format")?;

    let data = client.get(url).send().await?.bytes().await?.to_vec();

    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let actual_hash = format!("{:x}", hasher.finalize());

    if actual_hash != expected_hash {
        anyhow::bail!("Checksum mismatch");
    }
    Ok(data)
}

/// Extract binary from archive
fn extract_binary(data: &[u8], artifact_name: &str) -> Result<Vec<u8>> {
    if artifact_name.ends_with(".tar.gz") {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let mut archive = Archive::new(GzDecoder::new(data));
        for entry in archive.entries()? {
            let mut entry = entry?;
            if entry.path()?.file_name().map(|n| n == "mcp-connect").unwrap_or(false) {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                return Ok(buf);
            }
        }
        anyhow::bail!("Binary not found in archive")
    } else if artifact_name.ends_with(".zip") {
        use std::io::Cursor;
        use zip::ZipArchive;

        let mut archive = ZipArchive::new(Cursor::new(data))?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name().contains("mcp-connect") {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                return Ok(buf);
            }
        }
        anyhow::bail!("Binary not found in archive")
    } else {
        anyhow::bail!("Unknown archive format")
    }
}

/// Replace current binary with new one
fn replace_binary(new_binary: &[u8]) -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let backup_path = current_exe.with_extension("old");
    let new_path = current_exe.with_extension("new");

    let mut file = File::create(&new_path)?;
    file.write_all(new_binary)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&new_path, fs::Permissions::from_mode(0o755))?;
    }

    if current_exe.exists() {
        fs::rename(&current_exe, &backup_path)?;
    }
    fs::rename(&new_path, &current_exe)?;
    let _ = fs::remove_file(&backup_path);

    Ok(())
}

/// Perform the actual update
async fn perform_update(client: &Client, release: &Release, version: &str) -> Result<()> {
    let artifact_name = get_artifact_name()?;
    let checksum_name = format!("{}.sha256", artifact_name);

    let binary_url = release
        .assets
        .iter()
        .find(|a| a.name == artifact_name)
        .map(|a| &a.browser_download_url)
        .context(format!("No release for {}", artifact_name))?;

    let checksum_url = release
        .assets
        .iter()
        .find(|a| a.name == checksum_name)
        .map(|a| &a.browser_download_url)
        .context("Checksum not found")?;

    eprintln!("Downloading v{}...", version);
    let archive_data = download_with_checksum(client, binary_url, checksum_url).await?;

    eprintln!("Installing...");
    let binary_data = extract_binary(&archive_data, &artifact_name)?;
    replace_binary(&binary_data)?;

    eprintln!("Updated to v{}", version);
    Ok(())
}

/// Check for updates on startup (called from main)
/// Returns true if update was performed (caller should exit)
pub async fn check_and_auto_update() -> bool {
    let mut state = UpdateState::load();

    if !state.auto_update || !state.should_check() {
        return false;
    }

    state.mark_checked();
    let _ = state.save();

    let client = match Client::builder().user_agent("mcp-connect").build() {
        Ok(c) => c,
        Err(_) => return false,
    };

    match fetch_latest_version(&client).await {
        Ok(Some((version, release))) => {
            eprintln!("[mcp-connect] Update available: v{} -> v{}", CURRENT_VERSION, version);
            if perform_update(&client, &release, &version).await.is_ok() {
                eprintln!("[mcp-connect] Restart to use new version");
                return true;
            }
        }
        Ok(None) => {}
        Err(_) => {}
    }
    false
}

/// Run the update process (manual)
pub async fn run_update(force: bool) -> Result<()> {
    println!("Checking for updates...");
    println!("Current version: v{}", CURRENT_VERSION);

    let client = Client::builder().user_agent("mcp-connect").build()?;

    // Update state
    let mut state = UpdateState::load();
    state.mark_checked();
    let _ = state.save();

    match fetch_latest_version(&client).await? {
        Some((version, release)) if force || version_gt(&version, CURRENT_VERSION) => {
            println!("New version available: v{}", version);
            perform_update(&client, &release, &version).await?;
            println!("Successfully updated to v{}", version);
        }
        _ if force => {
            println!("Already on latest. Use --force to reinstall.");
        }
        _ => {
            println!("Already up to date (v{})", CURRENT_VERSION);
        }
    }
    Ok(())
}

/// Print version and check for updates
pub async fn version_check() -> Result<()> {
    println!("mcp-connect v{}", CURRENT_VERSION);

    let state = UpdateState::load();
    println!("Auto-update: {}", if state.auto_update { "enabled" } else { "disabled" });

    let client = Client::builder().user_agent("mcp-connect").build()?;
    if let Ok(Some((version, _))) = fetch_latest_version(&client).await {
        println!("\nUpdate available: v{}", version);
        println!("Run 'mcp-connect update' to install");
    }
    Ok(())
}

/// Toggle auto-update setting
pub fn set_auto_update(enabled: bool) -> Result<()> {
    let mut state = UpdateState::load();
    state.auto_update = enabled;
    state.save()?;
    println!("Auto-update {}", if enabled { "enabled" } else { "disabled" });
    Ok(())
}
