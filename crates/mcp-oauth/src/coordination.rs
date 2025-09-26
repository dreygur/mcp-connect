use crate::{OAuthError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Lock file data for coordination between multiple instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockfileData {
    pub pid: u32,
    pub port: u16,
    pub timestamp: u64,
    pub server_url_hash: String,
}

/// Coordination manager for multi-instance OAuth flows
pub struct CoordinationManager {
    auth_dir: PathBuf,
    server_url_hash: String,
}

impl CoordinationManager {
    pub fn new(auth_dir: PathBuf, server_url_hash: String) -> Self {
        Self {
            auth_dir,
            server_url_hash,
        }
    }

    /// Get the path to the lock file for this server
    fn lock_file_path(&self) -> PathBuf {
        self.auth_dir.join(format!("{}_lock.json", self.server_url_hash))
    }

    /// Check if a lock file exists and is valid
    pub async fn check_lockfile(&self) -> Result<Option<LockfileData>> {
        let lock_path = self.lock_file_path();

        if !lock_path.exists() {
            debug!("No lock file found at {:?}", lock_path);
            return Ok(None);
        }

        let content = fs::read_to_string(&lock_path)
            .map_err(|e| OAuthError::TokenStorage(format!("Failed to read lock file: {}", e)))?;

        let lock_data: LockfileData = serde_json::from_str(&content)
            .map_err(|e| OAuthError::TokenStorage(format!("Invalid lock file format: {}", e)))?;

        // Check if lock is still valid
        if self.is_lock_valid(&lock_data).await? {
            Ok(Some(lock_data))
        } else {
            info!("Lock file exists but is invalid, removing it");
            self.delete_lockfile().await?;
            Ok(None)
        }
    }

    /// Check if a lock file is valid (process running and not too old)
    async fn is_lock_valid(&self, lock_data: &LockfileData) -> Result<bool> {
        const MAX_LOCK_AGE_SECS: u64 = 30 * 60; // 30 minutes

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check if lock is too old
        if now - lock_data.timestamp > MAX_LOCK_AGE_SECS {
            debug!("Lock file is too old: {} seconds", now - lock_data.timestamp);
            return Ok(false);
        }

        // Check if process is still running
        if !is_pid_running(lock_data.pid) {
            debug!("Process {} from lock file is not running", lock_data.pid);
            return Ok(false);
        }

        // TODO: Could add endpoint accessibility check here like geelen does
        // For now, we'll rely on PID check
        debug!("Lock file is valid");
        Ok(true)
    }

    /// Create a lock file for this instance
    pub async fn create_lockfile(&self, port: u16) -> Result<()> {
        // Ensure auth directory exists
        if let Some(parent) = self.auth_dir.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| OAuthError::TokenStorage(format!("Failed to create auth directory: {}", e)))?;
        }
        fs::create_dir_all(&self.auth_dir)
            .map_err(|e| OAuthError::TokenStorage(format!("Failed to create auth directory: {}", e)))?;

        let lock_data = LockfileData {
            pid: process::id(),
            port,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            server_url_hash: self.server_url_hash.clone(),
        };

        let content = serde_json::to_string_pretty(&lock_data)
            .map_err(|e| OAuthError::TokenStorage(format!("Failed to serialize lock data: {}", e)))?;

        fs::write(self.lock_file_path(), content)
            .map_err(|e| OAuthError::TokenStorage(format!("Failed to write lock file: {}", e)))?;

        info!("Created lock file for PID {} on port {}", process::id(), port);
        Ok(())
    }

    /// Delete the lock file
    pub async fn delete_lockfile(&self) -> Result<()> {
        let lock_path = self.lock_file_path();

        if lock_path.exists() {
            fs::remove_file(&lock_path)
                .map_err(|e| OAuthError::TokenStorage(format!("Failed to delete lock file: {}", e)))?;
            debug!("Deleted lock file: {:?}", lock_path);
        }

        Ok(())
    }

    /// Wait for authentication to complete on another instance
    pub async fn wait_for_authentication(&self, port: u16) -> Result<bool> {
        const POLL_INTERVAL_SECS: u64 = 2;
        const MAX_WAIT_SECS: u64 = 300; // 5 minutes
        const MAX_POLLS: u64 = MAX_WAIT_SECS / POLL_INTERVAL_SECS;

        info!("Waiting for authentication to complete on port {}", port);

        for poll_count in 0..MAX_POLLS {
            tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

            // Check if the lock file still exists and is valid
            match self.check_lockfile().await? {
                Some(lock_data) if lock_data.port == port => {
                    // Lock still exists, continue waiting
                    debug!("Poll {}/{}: Authentication still in progress", poll_count + 1, MAX_POLLS);
                    continue;
                }
                Some(_) => {
                    // Lock exists but for different port, something's wrong
                    warn!("Lock file exists but for different port, giving up coordination");
                    return Ok(false);
                }
                None => {
                    // Lock file gone, authentication should be complete
                    info!("Lock file disappeared, authentication may be complete");
                    return Ok(true);
                }
            }
        }

        warn!("Timed out waiting for authentication to complete, proceeding with own auth");
        Ok(false)
    }

    /// Wait for authentication and clean up on completion or timeout
    pub async fn wait_and_cleanup(&self, port: u16) -> Result<bool> {
        let result = self.wait_for_authentication(port).await;

        // Always try to clean up our lock file
        if let Err(e) = self.delete_lockfile().await {
            warn!("Failed to clean up lock file: {}", e);
        }

        result
    }
}

/// Check if a process ID is running
fn is_pid_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use std::process::{Command, Stdio};

        // On Unix systems, use kill -0 to check if process exists
        match Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }

    #[cfg(windows)]
    {
        use std::process::{Command, Stdio};

        // On Windows, use tasklist to check if process exists
        match Command::new("tasklist")
            .arg("/FI")
            .arg(format!("PID eq {}", pid))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
        {
            Ok(status) => status.success(),
            Err(_) => false,
        }
    }
}

/// Generate a hash for the server URL for use in file names
pub fn hash_server_url(url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
