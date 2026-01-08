//! Session management for authenticated vault access.
//!
//! This module provides the [`Session`] trait and supporting infrastructure
//! for managing authenticated sessions with secret backends, including disk-based
//! caching and automatic refresh.

use crate::{Result, VaultmuxError};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Session represents an authenticated session with a backend.
///
/// Sessions encapsulate authentication state and provide methods to check
/// validity and refresh expired sessions.
///
/// # Thread Safety
///
/// All session implementations must be `Send + Sync` to support concurrent
/// access across async tasks.
#[async_trait]
pub trait Session: Send + Sync {
    /// Returns the session token.
    ///
    /// For CLI backends (Bitwarden, 1Password), this is the session token
    /// string used in environment variables.
    ///
    /// For stateless backends (pass, cloud SDKs), this may return an empty
    /// string or a placeholder identifier.
    fn token(&self) -> &str;

    /// Checks if the session is still valid.
    ///
    /// This may involve:
    /// - Checking expiration time against current time
    /// - Calling backend APIs to verify token validity
    /// - Always returning true for non-expiring sessions
    async fn is_valid(&self) -> bool;

    /// Attempts to refresh an expired session.
    ///
    /// Not all backends support refresh. Cloud SDK backends typically
    /// auto-refresh credentials transparently.
    ///
    /// # Errors
    ///
    /// Returns [`VaultmuxError::SessionExpired`] if refresh fails and
    /// re-authentication is required.
    async fn refresh(&mut self) -> Result<()>;

    /// Returns when the session expires, if applicable.
    ///
    /// Returns `None` for non-expiring sessions (pass, some cloud backends).
    fn expires_at(&self) -> Option<DateTime<Utc>>;
}

/// Cached session data stored on disk.
///
/// Session tokens are persisted to disk with restricted permissions (0600 on Unix)
/// to avoid repeated authentication prompts while maintaining security.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSession {
    /// The session token
    pub token: String,
    /// When this session was created
    pub created: DateTime<Utc>,
    /// When this session expires
    pub expires: DateTime<Utc>,
    /// Backend name (for debugging)
    pub backend: String,
}

/// Session cache handles persistence of session tokens to disk.
///
/// # Security
///
/// - Cache files are created with mode 0600 (owner read/write only) on Unix
/// - Parent directories are created with mode 0700 (owner access only)
/// - Invalid or expired sessions are automatically deleted
/// - Tokens are never logged or exposed in errors
///
/// # Example
///
/// ```no_run
/// use vaultmux::session::SessionCache;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> vaultmux::Result<()> {
///     let cache = SessionCache::new(
///         "/tmp/.vaultmux-session",
///         Duration::from_secs(1800)
///     ).await?;
///
///     // Save a session
///     cache.save("session-token-here", "bitwarden").await?;
///
///     // Load it back later
///     if let Some(cached) = cache.load().await? {
///         println!("Loaded session from cache");
///     }
///
///     Ok(())
/// }
/// ```
pub struct SessionCache {
    path: PathBuf,
    ttl: std::time::Duration,
}

impl SessionCache {
    /// Creates a new session cache.
    ///
    /// The parent directory is created with restricted permissions (0700 on Unix)
    /// to protect session tokens.
    ///
    /// # Arguments
    ///
    /// - `path`: File path where session will be cached
    /// - `ttl`: How long the session should be considered valid
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation fails.
    pub async fn new(path: impl AsRef<Path>, ttl: std::time::Duration) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(parent).await?.permissions();
                perms.set_mode(0o700);
                fs::set_permissions(parent, perms).await?;
            }
        }

        Ok(Self { path, ttl })
    }

    /// Loads a cached session from disk.
    ///
    /// Returns `Ok(None)` if:
    /// - File does not exist
    /// - Session is expired
    /// - File contains invalid JSON
    ///
    /// Invalid or expired sessions are automatically deleted.
    ///
    /// # Errors
    ///
    /// Returns an error only for unexpected I/O failures (not missing files).
    pub async fn load(&self) -> Result<Option<CachedSession>> {
        let data = match fs::read(&self.path).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let session: CachedSession = match serde_json::from_slice(&data) {
            Ok(s) => s,
            Err(_) => {
                let _ = fs::remove_file(&self.path).await;
                return Ok(None);
            }
        };

        if Utc::now() > session.expires {
            let _ = fs::remove_file(&self.path).await;
            return Ok(None);
        }

        Ok(Some(session))
    }

    /// Saves a session to disk.
    ///
    /// The session file is created with mode 0600 on Unix systems to prevent
    /// unauthorized access to the session token.
    ///
    /// # Arguments
    ///
    /// - `token`: Session token to cache
    /// - `backend`: Backend name (for debugging)
    ///
    /// # Errors
    ///
    /// Returns an error if writing fails or if the TTL cannot be converted
    /// to a chrono Duration.
    pub async fn save(&self, token: impl Into<String>, backend: impl Into<String>) -> Result<()> {
        let now = Utc::now();
        let ttl_duration = Duration::from_std(self.ttl)
            .map_err(|e| VaultmuxError::Other(e.into()))?;

        let session = CachedSession {
            token: token.into(),
            created: now,
            expires: now + ttl_duration,
            backend: backend.into(),
        };

        let json = serde_json::to_vec_pretty(&session)?;

        let mut file = fs::File::create(&self.path).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata().await?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.path, perms).await?;
        }

        file.write_all(&json).await?;
        file.flush().await?;

        Ok(())
    }

    /// Clears the cached session.
    ///
    /// This is idempotent - calling it multiple times or on a non-existent
    /// cache file is not an error.
    pub async fn clear(&self) -> Result<()> {
        match fs::remove_file(&self.path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_session_cache_save_and_load() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join("session.json");
        
        let cache = SessionCache::new(&cache_path, std::time::Duration::from_secs(3600))
            .await
            .unwrap();

        cache.save("test-token", "test-backend").await.unwrap();

        let loaded = cache.load().await.unwrap();
        assert!(loaded.is_some());
        
        let session = loaded.unwrap();
        assert_eq!(session.token, "test-token");
        assert_eq!(session.backend, "test-backend");
    }

    #[tokio::test]
    async fn test_session_cache_expiry() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join("session.json");
        
        let cache = SessionCache::new(&cache_path, std::time::Duration::from_millis(100))
            .await
            .unwrap();

        cache.save("test-token", "test-backend").await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(200)).await;

        let loaded = cache.load().await.unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_session_cache_clear() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join("session.json");
        
        let cache = SessionCache::new(&cache_path, std::time::Duration::from_secs(3600))
            .await
            .unwrap();

        cache.save("test-token", "test-backend").await.unwrap();
        cache.clear().await.unwrap();

        let loaded = cache.load().await.unwrap();
        assert!(loaded.is_none());
    }
}
