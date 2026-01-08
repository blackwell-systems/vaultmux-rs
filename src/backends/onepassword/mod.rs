//! 1Password CLI backend.
//!
//! This backend integrates with 1Password via the `op` command-line tool.
//! It requires the 1Password CLI to be installed and configured.
//!
//! # Authentication
//!
//! The backend uses session tokens stored in `OP_SESSION_<account>` environment
//! variables. Sessions expire after 30 minutes of inactivity.
//!
//! # Configuration
//!
//! - `account`: 1Password account shorthand (default: determined by `op account list`)
//! - `vault`: Vault name to use (default: "Private")
//! - `prefix`: Item name prefix for namespacing
//!
//! # Example
//!
//! ```
//! use vaultmux::{Config, BackendType};
//!
//! let config = Config::new(BackendType::OnePassword)
//!     .with_option("account", "my-account")
//!     .with_option("vault", "Development");
//! ```

mod backend;
mod session;

pub use backend::OnePasswordBackend;
pub use session::OnePasswordSession;

use crate::factory;

/// Registers the 1Password backend with the factory.
pub fn register() {
    factory::register_backend("onepassword", |config| {
        Ok(Box::new(OnePasswordBackend::new(config)))
    });
    factory::register_backend("op", |config| Ok(Box::new(OnePasswordBackend::new(config))));
}
