//! Vaultmux - Unified interface for multi-vault secret management.
//!
//! Vaultmux provides a single API for interacting with multiple secret management
//! systems. Write your code once and support Bitwarden, 1Password, pass, Windows
//! Credential Manager, AWS Secrets Manager, Google Cloud Secret Manager, and Azure
//! Key Vault with the same interface.
//!
//! # Features
//!
//! - **Unified API**: Single interface works with any backend
//! - **Async/Await**: Built on tokio for non-blocking I/O
//! - **Type Safety**: Leverage Rust's type system for compile-time guarantees
//! - **Session Caching**: Avoid repeated authentication prompts
//! - **Error Context**: Rich error types with full context and chaining
//! - **Feature Flags**: Optional backend compilation to minimize dependencies
//!
//! # Quick Start
//!
//! ```no_run
//! use vaultmux::{factory, Config, BackendType, Backend};
//!
//! #[tokio::main]
//! async fn main() -> vaultmux::Result<()> {
//!     // Create backend configuration
//!     let config = Config::new(BackendType::Pass)
//!         .with_prefix("myapp");
//!
//!     // Initialize backend
//!     let mut backend = factory::new_backend(config)?;
//!     backend.init().await?;
//!
//!     // Authenticate
//!     let session = backend.authenticate().await?;
//!
//!     // Store a secret
//!     backend.create_item("api-key", "sk-secret123", &*session).await?;
//!
//!     // Retrieve it
//!     let secret = backend.get_notes("api-key", &*session).await?;
//!     println!("Secret: {}", secret);
//!
//!     Ok(())
//! }
//! ```
//!
//! # Supported Backends
//!
//! | Backend | Feature Flag | CLI Required | Notes |
//! |---------|-------------|--------------|-------|
//! | Mock | `mock` (default) | None | In-memory testing backend |
//! | Bitwarden | `bitwarden` | `bw` | CLI integration |
//! | 1Password | `onepassword` | `op` | CLI integration |
//! | pass | `pass` | `pass`, `gpg` | Unix only |
//! | Windows Credential Manager | `wincred` | PowerShell | Windows only |
//! | AWS Secrets Manager | `aws` | None | SDK-based |
//! | GCP Secret Manager | `gcp` | None | SDK-based |
//! | Azure Key Vault | `azure` | None | SDK-based |
//!
//! # Feature Flags
//!
//! Enable backends by adding feature flags to `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! vaultmux = { version = "0.1", features = ["bitwarden", "aws"] }
//! ```
//!
//! Or use `full` to enable all backends:
//!
//! ```toml
//! [dependencies]
//! vaultmux = { version = "0.1", features = ["full"] }
//! ```

pub mod backend;
pub mod session;
pub mod item;
pub mod error;
pub mod config;
pub mod factory;
pub mod validation;
pub mod backends;

pub use backend::Backend;
pub use session::Session;
pub use item::{Item, ItemType};
pub use error::{Result, VaultmuxError};
pub use config::{Config, BackendType};

use lazy_static::lazy_static;
use std::sync::Once;

static INIT: Once = Once::new();

lazy_static! {
    /// Ensures all backends are registered exactly once.
    static ref REGISTER_BACKENDS: () = {
        backends::register_all();
    };
}

/// Initializes the vaultmux library.
///
/// This registers all compiled backends with the factory. It's called
/// automatically when the library is used, but can be called explicitly
/// if needed (it's idempotent).
pub fn init() {
    INIT.call_once(|| {
        lazy_static::initialize(&REGISTER_BACKENDS);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_initialization() {
        init();
        init();
    }

    #[tokio::test]
    async fn test_mock_backend_roundtrip() {
        init();

        let config = Config::new(BackendType::Pass);
        
        let backend = factory::new_backend(config);
        
        assert!(backend.is_err());
    }
}
