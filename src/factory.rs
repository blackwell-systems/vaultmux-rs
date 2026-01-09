//! Backend factory and registration system.

use crate::{Backend, Config, Result, VaultmuxError};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Factory function type for creating backends.
pub type BackendFactory = fn(Config) -> Result<Box<dyn Backend>>;

static BACKEND_REGISTRY: OnceLock<RwLock<HashMap<String, BackendFactory>>> = OnceLock::new();

fn registry() -> &'static RwLock<HashMap<String, BackendFactory>> {
    BACKEND_REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Registers a backend factory function.
///
/// This is typically called from backend modules' `register()` functions
/// during library initialization.
///
/// # Example
///
/// ```no_run
/// use vaultmux::factory::register_backend;
/// use vaultmux::{Backend, Config, Result};
///
/// fn my_backend_factory(config: Config) -> Result<Box<dyn Backend>> {
///     // Create and return backend instance
///     # unimplemented!()
/// }
///
/// pub fn register() {
///     register_backend("mybackend", my_backend_factory);
/// }
/// ```
pub fn register_backend(backend_type: &str, factory: BackendFactory) {
    let mut reg = registry().write().unwrap();
    reg.insert(backend_type.to_string(), factory);
}

/// Creates a new backend from configuration.
///
/// The appropriate backend factory is looked up based on `config.backend`.
/// If the backend is not registered, an error is returned with a hint to
/// check feature flags.
///
/// # Errors
///
/// Returns an error if:
/// - Backend type is not registered (missing feature flag or `register()` call)
/// - Backend factory returns an error during initialization
///
/// # Example
///
/// ```no_run
/// use vaultmux::{Config, BackendType, factory};
///
/// #[tokio::main]
/// async fn main() -> vaultmux::Result<()> {
///     let config = Config::new(BackendType::Pass);
///     let backend = factory::new_backend(config)?;
///     Ok(())
/// }
/// ```
pub fn new_backend(config: Config) -> Result<Box<dyn Backend>> {
    let backend_name = config.backend.to_string();

    let reg = registry().read().unwrap();
    let factory = reg.get(&backend_name).ok_or_else(|| {
        VaultmuxError::Other(anyhow::anyhow!(
            "unknown backend: {} (did you enable the '{}' feature flag?)",
            backend_name,
            backend_name
        ))
    })?;

    factory(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BackendType;

    fn mock_factory(_cfg: Config) -> Result<Box<dyn Backend>> {
        Err(VaultmuxError::Other(anyhow::anyhow!("mock factory")))
    }

    #[test]
    fn test_backend_registration() {
        register_backend("test-backend", mock_factory);

        let reg = registry().read().unwrap();
        assert!(reg.contains_key("test-backend"));
    }

    #[test]
    #[cfg(not(feature = "bitwarden"))]
    fn test_unknown_backend_error() {
        let config = Config::new(BackendType::Bitwarden);
        let result = new_backend(config);

        assert!(result.is_err());
        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.contains("unknown backend"));
            assert!(err_msg.contains("feature flag"));
        }
    }
}
