//! Common utilities for CLI-based backends.
//!
//! This module provides shared infrastructure for backends that integrate
//! via command-line tools (Bitwarden, 1Password, pass).

use crate::{Result, VaultmuxError};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Executes a command and returns stdout as a string.
///
/// This is the primary way CLI backends should execute commands.
///
/// # Arguments
///
/// - `program`: Command to execute (e.g., "bw", "op", "pass")
/// - `args`: Command arguments
/// - `env`: Optional environment variables (e.g., session tokens)
///
/// # Errors
///
/// Returns [`VaultmuxError::CommandFailed`] if:
/// - Command not found
/// - Exit code is non-zero
/// - Output is not valid UTF-8
pub async fn run_command(
    program: &str,
    args: &[&str],
    env: &[(&str, &str)],
) -> Result<String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    for (key, value) in env {
        cmd.env(key, value);
    }

    let output = cmd.output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            VaultmuxError::BackendNotInstalled(format!("{} command not found", program))
        } else {
            VaultmuxError::Io(e)
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(VaultmuxError::CommandFailed(format!(
            "{} failed with exit code {}: {}",
            program,
            output.status.code().unwrap_or(-1),
            stderr
        )));
    }

    String::from_utf8(output.stdout).map_err(|e| {
        VaultmuxError::Other(anyhow::anyhow!("Invalid UTF-8 in command output: {}", e))
    })
}

/// Executes a command with stdin input.
///
/// Used for interactive operations like authentication.
pub async fn run_command_with_stdin(
    program: &str,
    args: &[&str],
    env: &[(&str, &str)],
    stdin_data: &str,
) -> Result<String> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    for (key, value) in env {
        cmd.env(key, value);
    }

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            VaultmuxError::BackendNotInstalled(format!("{} command not found", program))
        } else {
            VaultmuxError::Io(e)
        }
    })?;

    // Write to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_data.as_bytes())
            .await
            .map_err(VaultmuxError::Io)?;
        stdin.flush().await.map_err(VaultmuxError::Io)?;
    }

    let output = child.wait_with_output().await.map_err(VaultmuxError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(VaultmuxError::CommandFailed(format!(
            "{} failed with exit code {}: {}",
            program,
            output.status.code().unwrap_or(-1),
            stderr
        )));
    }

    String::from_utf8(output.stdout).map_err(|e| {
        VaultmuxError::Other(anyhow::anyhow!("Invalid UTF-8 in command output: {}", e))
    })
}

/// Checks if a command-line tool is available in PATH.
///
/// # Example
///
/// ```no_run
/// use vaultmux::cli::check_command_exists;
///
/// #[tokio::main]
/// async fn main() -> vaultmux::Result<()> {
///     if !check_command_exists("bw").await? {
///         println!("Bitwarden CLI is not installed");
///     }
///     Ok(())
/// }
/// ```
pub async fn check_command_exists(program: &str) -> Result<bool> {
    let output = Command::new("which")
        .arg(program)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map_err(VaultmuxError::Io)?;

    Ok(output.success())
}

/// Status cache with time-to-live for authentication checks.
///
/// CLI backends often need to check authentication status, which requires
/// shelling out to the CLI tool. This can be slow (50-200ms per check).
///
/// StatusCache caches the authentication status for a configurable TTL
/// (default 5 seconds) to dramatically reduce overhead.
///
/// # Thread Safety
///
/// This struct is not thread-safe. Wrap in `Arc<Mutex<StatusCache>>` for
/// concurrent access.
///
/// # Example
///
/// ```
/// use vaultmux::cli::StatusCache;
/// use std::time::Duration;
///
/// let mut cache = StatusCache::new(Duration::from_secs(5));
///
/// // First check - cache miss
/// if cache.get().is_none() {
///     // Perform expensive authentication check
///     let is_authenticated = true; // ... actual check
///     cache.set(is_authenticated);
/// }
///
/// // Second check within TTL - cache hit (no expensive check)
/// assert_eq!(cache.get(), Some(true));
/// ```
#[derive(Debug)]
pub struct StatusCache {
    authenticated: bool,
    timestamp: Option<Instant>,
    ttl: Duration,
}

impl StatusCache {
    /// Creates a new status cache with the specified TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            authenticated: false,
            timestamp: None,
            ttl,
        }
    }

    /// Gets the cached authentication status if still valid.
    ///
    /// Returns `None` if the cache is invalid or expired.
    pub fn get(&self) -> Option<bool> {
        if let Some(ts) = self.timestamp {
            if ts.elapsed() < self.ttl {
                return Some(self.authenticated);
            }
        }
        None
    }

    /// Sets the authentication status and updates the timestamp.
    pub fn set(&mut self, authenticated: bool) {
        self.authenticated = authenticated;
        self.timestamp = Some(Instant::now());
    }

    /// Invalidates the cache.
    pub fn invalidate(&mut self) {
        self.timestamp = None;
    }
}

impl Default for StatusCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command_success() {
        let output = run_command("echo", &["hello"], &[]).await.unwrap();
        assert_eq!(output.trim(), "hello");
    }

    #[tokio::test]
    async fn test_run_command_not_found() {
        let result = run_command("nonexistent-command-12345", &[], &[]).await;
        assert!(result.is_err());
        // Command should fail (either not found or permission denied for non-executable file)
        // The exact error depends on the system
    }

    #[tokio::test]
    async fn test_run_command_with_env() {
        let output = run_command("printenv", &["TEST_VAR"], &[("TEST_VAR", "test-value")])
            .await
            .unwrap();
        assert_eq!(output.trim(), "test-value");
    }

    #[tokio::test]
    async fn test_run_command_with_stdin() {
        let output = run_command_with_stdin("cat", &[], &[], "hello from stdin")
            .await
            .unwrap();
        assert_eq!(output.trim(), "hello from stdin");
    }

    #[tokio::test]
    async fn test_check_command_exists() {
        assert!(check_command_exists("echo").await.unwrap());
        assert!(!check_command_exists("nonexistent-command-12345")
            .await
            .unwrap());
    }

    #[test]
    fn test_status_cache() {
        let mut cache = StatusCache::new(Duration::from_millis(100));

        // Initially empty
        assert_eq!(cache.get(), None);

        // Set value
        cache.set(true);
        assert_eq!(cache.get(), Some(true));

        // Still valid
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(cache.get(), Some(true));

        // Expired
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(cache.get(), None);
    }

    #[test]
    fn test_status_cache_invalidate() {
        let mut cache = StatusCache::new(Duration::from_secs(10));
        cache.set(true);
        assert_eq!(cache.get(), Some(true));

        cache.invalidate();
        assert_eq!(cache.get(), None);
    }
}
