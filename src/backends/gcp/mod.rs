//! Google Cloud Secret Manager backend.
//!
//! This backend integrates with Google Cloud Secret Manager using the official
//! Google API client library. It requires Application Default Credentials (ADC)
//! to be configured.
//!
//! # Authentication
//!
//! The backend uses Google's Application Default Credentials, which can be:
//! - Service account key file (GOOGLE_APPLICATION_CREDENTIALS)
//! - gcloud auth application-default login
//! - GCE/GKE metadata server (when running in Google Cloud)
//!
//! # Configuration
//!
//! - `project_id`: GCP project ID (required)
//! - `prefix`: Secret name prefix for namespacing
//!
//! # Example
//!
//! ```
//! use vaultmux::{Config, BackendType};
//!
//! let config = Config::new(BackendType::GCPSecretManager)
//!     .with_option("project_id", "my-project-123")
//!     .with_option("prefix", "app-");
//! ```

mod backend;
mod session;

pub use backend::GCPBackend;
pub use session::GCPSession;

use crate::factory;

/// Registers the GCP Secret Manager backend with the factory.
pub fn register() {
    factory::register_backend("gcpsecrets", |config| Ok(Box::new(GCPBackend::new(config))));
    factory::register_backend("gcp", |config| Ok(Box::new(GCPBackend::new(config))));
}
