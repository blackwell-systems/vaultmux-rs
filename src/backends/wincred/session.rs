//! Windows Credential Manager session management.

use crate::{Result, Session};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Windows Credential Manager session.
///
/// Since Windows Credential Manager uses OS-level authentication, there's no
/// explicit session token. This is a placeholder to satisfy the Session trait.
pub struct WincredSession {}

impl WincredSession {
    /// Creates a new Windows Credential Manager session.
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for WincredSession {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Session for WincredSession {
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

impl Clone for WincredSession {
    fn clone(&self) -> Self {
        Self {}
    }
}
