//! Bitwarden session implementation.

use crate::{Result, Session, VaultmuxError};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

/// Bitwarden session with token caching.
///
/// Bitwarden sessions are created by unlocking the vault with a password.
/// The session token is valid until the vault is locked again.
pub struct BitwardenSession {
    token: String,
    expires: DateTime<Utc>,
}

impl BitwardenSession {
    /// Creates a new Bitwarden session.
    ///
    /// # Arguments
    ///
    /// - `token`: Session token from `bw unlock`
    /// - `ttl`: Time-to-live for the session
    pub fn new(token: String, ttl: std::time::Duration) -> Self {
        let now = Utc::now();
        let duration = Duration::from_std(ttl).unwrap_or(Duration::seconds(1800));

        Self {
            token,
            expires: now + duration,
        }
    }

    /// Creates a session from a token and explicit expiry time.
    pub fn from_token_and_expiry(token: String, expires: DateTime<Utc>) -> Self {
        Self { token, expires }
    }
}

#[async_trait]
impl Session for BitwardenSession {
    fn token(&self) -> &str {
        &self.token
    }

    async fn is_valid(&self) -> bool {
        Utc::now() < self.expires
    }

    async fn refresh(&mut self) -> Result<()> {
        // Bitwarden doesn't support token refresh
        // User must re-authenticate
        Err(VaultmuxError::SessionExpired)
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        Some(self.expires)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bitwarden_session_creation() {
        let session = BitwardenSession::new(
            "test-token".to_string(),
            std::time::Duration::from_secs(3600),
        );

        assert_eq!(session.token(), "test-token");
        assert!(session.is_valid().await);
    }

    #[tokio::test]
    async fn test_bitwarden_session_expiry() {
        let session = BitwardenSession::new(
            "test-token".to_string(),
            std::time::Duration::from_millis(1),
        );

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(!session.is_valid().await);
    }
}
