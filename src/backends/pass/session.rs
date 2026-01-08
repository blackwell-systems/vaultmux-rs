//! pass backend session implementation.

use crate::{Result, Session};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// pass session (stateless - GPG agent handles authentication).
///
/// Unlike Bitwarden or 1Password, pass doesn't use session tokens.
/// Authentication is handled by the GPG agent, which prompts for the
/// GPG key passphrase when needed.
#[derive(Debug, Clone)]
pub struct PassSession;

impl PassSession {
    /// Creates a new pass session.
    pub fn new() -> Self {
        Self
    }
}

impl Default for PassSession {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Session for PassSession {
    fn token(&self) -> &str {
        ""
    }

    async fn is_valid(&self) -> bool {
        true
    }

    async fn refresh(&mut self) -> Result<()> {
        Ok(())
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        None
    }
}
