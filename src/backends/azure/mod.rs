//! Azure Key Vault backend.
//!
//! This backend integrates with Azure Key Vault using the official Microsoft
//! Azure SDK for Rust. It requires Azure credentials to be configured.
//!
//! # Authentication
//!
//! The backend uses DefaultAzureCredential, which tries multiple authentication methods:
//! - Environment variables (AZURE_TENANT_ID, AZURE_CLIENT_ID, AZURE_CLIENT_SECRET)
//! - Managed Identity (when running in Azure)
//! - Azure CLI credentials
//! - Azure PowerShell credentials
//!
//! # Configuration
//!
//! - `vault_url`: Azure Key Vault URL (required, e.g., "<https://myvault.vault.azure.net>")
//! - `prefix`: Secret name prefix for namespacing
//!
//! # Example
//!
//! ```
//! use vaultmux::{Config, BackendType};
//!
//! let config = Config::new(BackendType::AzureKeyVault)
//!     .with_option("vault_url", "https://myvault.vault.azure.net")
//!     .with_option("prefix", "app-");
//! ```

mod backend;
mod session;

pub use backend::AzureBackend;
pub use session::AzureSession;

use crate::factory;

/// Registers the Azure Key Vault backend with the factory.
pub fn register() {
    factory::register_backend("azurekeyvault", |config| {
        Ok(Box::new(AzureBackend::new(config)))
    });
    factory::register_backend("azure", |config| Ok(Box::new(AzureBackend::new(config))));
}
