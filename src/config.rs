//! Configuration types for backend initialization.

use std::collections::HashMap;
use std::time::Duration;

/// Backend type identifier.
///
/// Each variant corresponds to a specific vault backend implementation.
/// Backends must be enabled via Cargo feature flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendType {
    /// Bitwarden CLI backend (requires `bw` command)
    Bitwarden,
    /// 1Password CLI backend (requires `op` command)
    OnePassword,
    /// pass (Unix password manager) backend (requires `pass` command)
    Pass,
    /// Windows Credential Manager (Windows only)
    WindowsCredentialManager,
    /// AWS Secrets Manager SDK backend
    AWSSecretsManager,
    /// Google Cloud Secret Manager SDK backend
    GCPSecretManager,
    /// Azure Key Vault SDK backend
    AzureKeyVault,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bitwarden => write!(f, "bitwarden"),
            Self::OnePassword => write!(f, "1password"),
            Self::Pass => write!(f, "pass"),
            Self::WindowsCredentialManager => write!(f, "wincred"),
            Self::AWSSecretsManager => write!(f, "awssecrets"),
            Self::GCPSecretManager => write!(f, "gcpsecrets"),
            Self::AzureKeyVault => write!(f, "azurekeyvault"),
        }
    }
}

/// Configuration for creating a backend.
///
/// Use the builder pattern for ergonomic configuration:
///
/// ```no_run
/// use vaultmux::{Config, BackendType};
///
/// let config = Config::new(BackendType::Pass)
///     .with_prefix("myapp")
///     .with_session_file("/tmp/.myapp-session")
///     .with_option("region", "us-west-2");
/// ```
#[derive(Debug, Clone)]
pub struct Config {
    /// Backend type
    pub backend: BackendType,

    /// Pass-specific: password store path (default: ~/.password-store)
    pub store_path: Option<String>,

    /// Prefix for item names (default: "dotfiles")
    pub prefix: String,

    /// Session cache file location
    pub session_file: Option<String>,

    /// Session TTL (default: 30 minutes)
    pub session_ttl: Duration,

    /// Backend-specific options
    pub options: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend: BackendType::Pass,
            store_path: None,
            prefix: "dotfiles".to_string(),
            session_file: None,
            session_ttl: Duration::from_secs(1800), // 30 minutes
            options: HashMap::new(),
        }
    }
}

impl Config {
    /// Creates a new configuration for the specified backend.
    ///
    /// # Example
    ///
    /// ```
    /// use vaultmux::{Config, BackendType};
    ///
    /// let config = Config::new(BackendType::Bitwarden);
    /// assert_eq!(config.backend, BackendType::Bitwarden);
    /// ```
    pub fn new(backend: BackendType) -> Self {
        Self {
            backend,
            ..Default::default()
        }
    }

    /// Sets the item name prefix.
    ///
    /// This prefix is prepended to all item names to provide namespacing.
    /// For example, with prefix "myapp", an item named "api-key" becomes
    /// "myapp/api-key" (exact format depends on backend).
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Sets the password store path (pass backend only).
    pub fn with_store_path(mut self, path: impl Into<String>) -> Self {
        self.store_path = Some(path.into());
        self
    }

    /// Sets the session cache file location.
    ///
    /// If not set, a default location will be used (typically in the user's
    /// home directory or temp directory).
    pub fn with_session_file(mut self, path: impl Into<String>) -> Self {
        self.session_file = Some(path.into());
        self
    }

    /// Sets the session time-to-live.
    ///
    /// This determines how long cached sessions remain valid before requiring
    /// re-authentication.
    pub fn with_session_ttl(mut self, ttl: Duration) -> Self {
        self.session_ttl = ttl;
        self
    }

    /// Adds a backend-specific option.
    ///
    /// Common options:
    ///
    /// **AWS Secrets Manager:**
    /// - `region`: AWS region (e.g., "us-west-2")
    /// - `prefix`: Secret name prefix (e.g., "myapp/")
    /// - `endpoint`: Custom endpoint URL (for LocalStack testing)
    ///
    /// **GCP Secret Manager:**
    /// - `project_id`: GCP project ID (required)
    /// - `prefix`: Secret name prefix (e.g., "myapp-")
    ///
    /// **Azure Key Vault:**
    /// - `vault_url`: Key Vault URL (e.g., "https://myvault.vault.azure.net/")
    /// - `prefix`: Secret name prefix (e.g., "myapp-")
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }

    /// Gets a backend-specific option value.
    pub fn get_option(&self, key: &str) -> Option<&String> {
        self.options.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = Config::new(BackendType::AWSSecretsManager)
            .with_prefix("myapp")
            .with_option("region", "us-west-2")
            .with_session_ttl(Duration::from_secs(3600));

        assert_eq!(config.backend, BackendType::AWSSecretsManager);
        assert_eq!(config.prefix, "myapp");
        assert_eq!(config.get_option("region"), Some(&"us-west-2".to_string()));
        assert_eq!(config.session_ttl, Duration::from_secs(3600));
    }

    #[test]
    fn test_backend_type_display() {
        assert_eq!(BackendType::Bitwarden.to_string(), "bitwarden");
        assert_eq!(BackendType::Pass.to_string(), "pass");
        assert_eq!(BackendType::AWSSecretsManager.to_string(), "awssecrets");
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.backend, BackendType::Pass);
        assert_eq!(config.prefix, "dotfiles");
        assert_eq!(config.session_ttl, Duration::from_secs(1800));
    }
}
