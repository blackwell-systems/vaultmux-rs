//! Azure session management.

use crate::{Result, Session};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Azure Key Vault session.
///
/// Azure uses DefaultAzureCredential, so there's no explicit session token.
/// This is a placeholder to satisfy the Session trait.
pub struct AzureSession {
    vault_url: String,
}

impl AzureSession {
    /// Creates a new Azure session.
    pub fn new(vault_url: String) -> Self {
        Self { vault_url }
    }

    /// Gets the vault URL.
    pub fn vault_url(&self) -> &str {
        &self.vault_url
    }
}

#[async_trait]
impl Session for AzureSession {
    fn token(&self) -> &str {
        // No explicit token for Azure - DefaultAzureCredential handles authentication
        ""
    }

    async fn is_valid(&self) -> bool {
        // Azure credentials are managed automatically
        true
    }

    async fn refresh(&mut self) -> Result<()> {
        // Azure handles refresh automatically
        Ok(())
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        // Azure manages expiry
        None
    }
}

impl Clone for AzureSession {
    fn clone(&self) -> Self {
        Self {
            vault_url: self.vault_url.clone(),
        }
    }
}
