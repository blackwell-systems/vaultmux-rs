//! Windows Credential Manager backend.
//!
//! This backend integrates with Windows Credential Manager using PowerShell commands.
//! It requires PowerShell to be available on the system.
//!
//! # Platform Support
//!
//! This backend is only available on Windows. On non-Windows platforms, attempting to
//! create this backend will return an error.
//!
//! # Authentication
//!
//! Windows Credential Manager uses OS-level authentication, so no explicit credentials
//! are required. The current Windows user's credentials are used automatically.
//!
//! # Configuration
//!
//! - `prefix`: Credential name prefix for namespacing (default: "vaultmux")
//!
//! # Example
//!
//! ```no_run
//! use vaultmux::{Config, BackendType};
//!
//! let config = Config::new(BackendType::WindowsCredentialManager)
//!     .with_prefix("myapp");
//! ```

#[cfg(target_os = "windows")]
mod backend;
#[cfg(target_os = "windows")]
mod session;

#[cfg(target_os = "windows")]
pub use backend::WincredBackend;
#[cfg(target_os = "windows")]
pub use session::WincredSession;

#[cfg(not(target_os = "windows"))]
mod backend_stub;
#[cfg(not(target_os = "windows"))]
pub use backend_stub::WincredBackend;

use crate::factory;

/// Registers the Windows Credential Manager backend with the factory.
pub fn register() {
    factory::register_backend("wincred", |config| {
        Ok(Box::new(WincredBackend::new(config)))
    });
    factory::register_backend("windowscredentialmanager", |config| {
        Ok(Box::new(WincredBackend::new(config)))
    });
}
