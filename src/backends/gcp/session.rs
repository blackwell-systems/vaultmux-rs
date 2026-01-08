//! GCP session management.

use crate::{Result, Session};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// GCP Secret Manager session.
///
/// GCP uses Application Default Credentials (ADC), so there's no explicit
/// session token. This is a placeholder to satisfy the Session trait.
pub struct GCPSession {
    project_id: String,
}

impl GCPSession {
    /// Creates a new GCP session.
    pub fn new(project_id: String) -> Self {
        Self { project_id }
    }

    /// Gets the project ID.
    pub fn project_id(&self) -> &str {
        &self.project_id
    }
}

#[async_trait]
impl Session for GCPSession {
    fn token(&self) -> &str {
        // No explicit token for GCP - ADC handles authentication
        ""
    }

    async fn is_valid(&self) -> bool {
        // ADC tokens are managed automatically
        true
    }

    async fn refresh(&mut self) -> Result<()> {
        // ADC handles refresh automatically
        Ok(())
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        // ADC manages expiry
        None
    }
}

impl Clone for GCPSession {
    fn clone(&self) -> Self {
        Self {
            project_id: self.project_id.clone(),
        }
    }
}
