//! Bitwarden backend.
//!
//! This backend integrates with the Bitwarden CLI (`bw`) to manage secrets
//! in Bitwarden vaults.
//!
//! # Requirements
//!
//! - Bitwarden CLI (`bw`) installed
//! - Bitwarden account
//! - Logged in (`bw login`)
//!
//! # Features
//!
//! - Session token caching
//! - JSON-based communication
//! - Folder organization
//! - Sync support
//! - Full CRUD operations
//!
//! # Example
//!
//! ```no_run
//! use vaultmux::{Config, BackendType, factory, Backend};
//!
//! #[tokio::main]
//! async fn main() -> vaultmux::Result<()> {
//!     let config = Config::new(BackendType::Bitwarden)
//!         .with_prefix("myapp")
//!         .with_session_file("/tmp/.bw-session");
//!
//!     let mut backend = factory::new_backend(config)?;
//!     backend.init().await?;
//!
//!     // Unlock vault (prompts for password)
//!     let session = backend.authenticate().await?;
//!
//!     // Create item
//!     backend.create_item("api-key", "secret-value", &*session).await?;
//!
//!     Ok(())
//! }
//! ```

mod backend;
mod session;

pub use backend::BitwardenBackend;
pub use session::BitwardenSession;

/// Registers the Bitwarden backend with the factory.
pub fn register() {
    crate::factory::register_backend("bitwarden", |cfg| Ok(Box::new(BitwardenBackend::new(cfg))));
}
