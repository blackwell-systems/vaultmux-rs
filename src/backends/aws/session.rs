//! AWS Secrets Manager session implementation.

use crate::{Result, Session};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// AWS Secrets Manager session.
///
/// AWS credentials are managed by the SDK and refreshed automatically.
/// No explicit session token is needed.
#[derive(Debug, Clone)]
pub struct AWSSession {
    region: String,
}

impl AWSSession {
    pub fn new(region: String) -> Self {
        Self { region }
    }

    pub fn region(&self) -> &str {
        &self.region
    }
}

#[async_trait]
impl Session for AWSSession {
    fn token(&self) -> &str {
        // AWS SDK handles credentials internally
        ""
    }

    async fn is_valid(&self) -> bool {
        // AWS SDK auto-refreshes credentials
        true
    }

    async fn refresh(&mut self) -> Result<()> {
        // AWS SDK handles refresh automatically
        Ok(())
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        // AWS SDK manages expiry internally
        None
    }
}
