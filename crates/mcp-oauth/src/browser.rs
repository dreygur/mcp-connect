use crate::{OAuthError, Result};
use std::process::Command;
use tracing::{debug, info, warn};

/// Cross-platform browser launcher for OAuth authorization flows
///
/// This module handles opening the user's default web browser to initiate
/// the OAuth authorization flow.
pub struct BrowserLauncher;

impl BrowserLauncher {
    /// Launch the default browser with the given URL
    ///
    /// # Arguments
    /// * `url` - The authorization URL to open in the browser
    ///
    /// # Returns
    /// Ok(()) if the browser was launched successfully, Err otherwise
    pub async fn launch(url: &str) -> Result<()> {
        info!("Launching browser for OAuth authorization: {}", url);

        let result = if cfg!(target_os = "windows") {
            Self::launch_windows(url).await
        } else if cfg!(target_os = "macos") {
            Self::launch_macos(url).await
        } else {
            Self::launch_linux(url).await
        };

        match result {
            Ok(()) => {
                info!("Browser launched successfully");
                Ok(())
            }
            Err(e) => {
                warn!("Failed to launch browser: {}", e);
                // Print URL to console as fallback
                println!("\n🔐 Please open the following URL in your browser to authorize the application:");
                println!("   {}", url);
                println!("   After authorization, return to this application.\n");
                Ok(())
            }
        }
    }

    /// Launch browser on Windows
    async fn launch_windows(url: &str) -> Result<()> {
        debug!("Launching browser on Windows");

        let output = Command::new("cmd")
            .args(&["/c", "start", url])
            .output()
            .map_err(|e| OAuthError::BrowserLaunch(format!("Windows browser launch failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OAuthError::BrowserLaunch(
                format!("Windows browser launch failed with status {}: {}",
                       output.status, stderr)
            ));
        }

        Ok(())
    }

    /// Launch browser on macOS
    async fn launch_macos(url: &str) -> Result<()> {
        debug!("Launching browser on macOS");

        let output = Command::new("open")
            .arg(url)
            .output()
            .map_err(|e| OAuthError::BrowserLaunch(format!("macOS browser launch failed: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(OAuthError::BrowserLaunch(
                format!("macOS browser launch failed with status {}: {}",
                       output.status, stderr)
            ));
        }

        Ok(())
    }

    /// Launch browser on Linux and other Unix-like systems
    async fn launch_linux(url: &str) -> Result<()> {
        debug!("Launching browser on Linux/Unix");

        // Try common browser launchers in order of preference
        let launchers = ["xdg-open", "gnome-open", "kde-open", "firefox", "chromium", "chrome"];

        for launcher in &launchers {
            debug!("Trying browser launcher: {}", launcher);

            match Command::new(launcher).arg(url).output() {
                Ok(output) if output.status.success() => {
                    debug!("Successfully launched browser with: {}", launcher);
                    return Ok(());
                }
                Ok(output) => {
                    debug!("Browser launcher {} failed with status: {}", launcher, output.status);
                    continue;
                }
                Err(e) => {
                    debug!("Browser launcher {} not found: {}", launcher, e);
                    continue;
                }
            }
        }

        Err(OAuthError::BrowserLaunch(
            "No suitable browser launcher found on this system".to_string()
        ))
    }

    /// Check if a browser launcher is available on this system
    ///
    /// This can be used to determine whether automatic browser launching
    /// will work before attempting the OAuth flow.
    pub fn is_available() -> bool {
        if cfg!(target_os = "windows") {
            // cmd.exe should always be available on Windows
            true
        } else if cfg!(target_os = "macos") {
            // open command should always be available on macOS
            Command::new("open").arg("--help").output().is_ok()
        } else {
            // Check for common Linux browser launchers
            let launchers = ["xdg-open", "gnome-open", "kde-open"];
            launchers.iter().any(|launcher| {
                Command::new(launcher).arg("--help").output().is_ok()
            })
        }
    }

    /// Get the name of the browser launcher that would be used
    ///
    /// This is mainly for debugging and informational purposes.
    pub fn get_launcher_name() -> String {
        if cfg!(target_os = "windows") {
            "cmd /c start".to_string()
        } else if cfg!(target_os = "macos") {
            "open".to_string()
        } else {
            // Find the first available launcher on Linux
            let launchers = ["xdg-open", "gnome-open", "kde-open", "firefox", "chromium", "chrome"];

            for launcher in &launchers {
                if Command::new(launcher).arg("--help").output().is_ok() {
                    return launcher.to_string();
                }
            }

            "none available".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_launcher_name() {
        let launcher = BrowserLauncher::get_launcher_name();

        if cfg!(target_os = "windows") {
            assert_eq!(launcher, "cmd /c start");
        } else if cfg!(target_os = "macos") {
            assert_eq!(launcher, "open");
        } else {
            // On Linux, should return one of the known launchers or "none available"
            assert!(!launcher.is_empty());
        }
    }

    #[test]
    fn test_is_available() {
        // This test will vary by platform, but should not panic
        let available = BrowserLauncher::is_available();
        // Just check that the function runs without error
        println!("Browser launcher available: {}", available);
    }
}
