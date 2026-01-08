//! AWS Secrets Manager backend.
//!
//! This backend integrates with AWS Secrets Manager using the official AWS SDK.
//!
//! # Requirements
//!
//! - AWS credentials configured via:
//!   - Environment variables (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`)
//!   - Shared credentials file (`~/.aws/credentials`)
//!   - IAM instance role (for EC2/ECS)
//!
//! # Features
//!
//! - Native SDK integration (no CLI)
//! - Automatic credential refresh
//! - Versioning support
//! - Tag-based organization
//! - Prefix-based namespacing
//!
//! # Example
//!
//! ```no_run
//! use vaultmux::{Config, BackendType, factory, Backend};
//!
//! #[tokio::main]
//! async fn main() -> vaultmux::Result<()> {
//!     let config = Config::new(BackendType::AWSSecretsManager)
//!         .with_option("region", "us-west-2")
//!         .with_prefix("myapp/");
//!
//!     let mut backend = factory::new_backend(config)?;
//!     backend.init().await?;
//!
//!     let session = backend.authenticate().await?;
//!     backend.create_item("api-key", "secret-value", &*session).await?;
//!
//!     Ok(())
//! }
//! ```

mod backend;
mod session;

pub use backend::AWSBackend;
pub use session::AWSSession;

/// Registers the AWS Secrets Manager backend with the factory.
pub fn register() {
    crate::factory::register_backend("awssecrets", |cfg| Ok(Box::new(AWSBackend::new(cfg))));
}
