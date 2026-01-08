//! 1Password session management.

use crate::{Result, Session};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};

/// 1Password session.
///
/// Stores the session token and account information.
/// Sessions in 1Password expire after 30 minutes of inactivity.
pub struct OnePasswordSession {
    token: String,
    account: String,
    expires: DateTime<Utc>,
}

impl OnePasswordSession {
    /// Creates a new 1Password session.
    pub fn new(token: String, account: String) -> Self {
        Self {
            token,
            account,
            expires: Utc::now() + Duration::minutes(30),
        }
    }

    /// Gets the account shorthand.
    pub fn account(&self) -> &str {
        &self.account
    }

    /// Gets the environment variable name for this session.
    pub fn env_var_name(&self) -> String {
        format!("OP_SESSION_{}", self.account)
    }
}

#[async_trait]
impl Session for OnePasswordSession {
    fn token(&self) -> &str {
        &self.token
    }

    async fn is_valid(&self) -> bool {
        Utc::now() < self.expires
    }

    async fn refresh(&mut self) -> Result<()> {
        self.expires = Utc::now() + Duration::minutes(30);
        Ok(())
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        Some(self.expires)
    }
}

impl Clone for OnePasswordSession {
    fn clone(&self) -> Self {
        Self {
            token: self.token.clone(),
            account: self.account.clone(),
            expires: self.expires,
        }
    }
}
