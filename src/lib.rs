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
pub mod backends;
pub mod cli;
pub mod config;
pub mod error;
pub mod factory;
pub mod item;
pub mod session;
pub mod validation;

pub use backend::Backend;
pub use config::{BackendType, Config};
pub use error::{Result, VaultmuxError};
pub use item::{Item, ItemType};
pub use session::Session;

use std::sync::Once;

static INIT: Once = Once::new();

/// Initializes the vaultmux library.
///
/// This registers all compiled backends with the factory. It's called
/// automatically when the library is used, but can be called explicitly
/// if needed (it's idempotent).
pub fn init() {
    INIT.call_once(|| {
        backends::register_all();
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

    #[cfg(feature = "mock")]
    #[tokio::test]
    async fn test_mock_backend_creation() {
        init();

        // Mock backend should be registered
        let config = Config {
            backend: BackendType::Pass,
            store_path: None,
            prefix: "test".to_string(),
            session_file: None,
            session_ttl: std::time::Duration::from_secs(1800),
            options: std::collections::HashMap::new(),
        };

        // Pass backend should be available now with the pass feature
        #[cfg(feature = "pass")]
        {
            let backend = factory::new_backend(config);
            assert!(backend.is_ok());
        }

        // Without pass feature, it should error
        #[cfg(not(feature = "pass"))]
        {
            let backend = factory::new_backend(config);
            assert!(backend.is_err());
        }
    }
}
