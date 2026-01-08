//! pass (Unix password manager) backend.
//!
//! This backend integrates with the `pass` command-line tool, which stores
//! passwords in GPG-encrypted files organized in a directory tree.
//!
//! # Requirements
//!
//! - `pass` command-line tool
//! - `gpg` (GnuPG) for encryption
//! - Initialized password store (`pass init <gpg-key-id>`)
//!
//! # Features
//!
//! - No session tokens (authentication handled by GPG agent)
//! - Directory-based organization (folders map to directories)
//! - Git integration (optional)
//! - Fully offline capable
//!
//! # Example
//!
//! ```no_run
//! use vaultmux::{Config, BackendType, factory, Backend};
//!
//! #[tokio::main]
//! async fn main() -> vaultmux::Result<()> {
//!     let config = Config::new(BackendType::Pass)
//!         .with_store_path("/home/user/.password-store")
//!         .with_prefix("myapp");
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

pub use backend::PassBackend;
pub use session::PassSession;

/// Registers the pass backend with the factory.
pub fn register() {
    crate::factory::register_backend("pass", |cfg| {
        Ok(Box::new(PassBackend::new(cfg)))
    });
}
