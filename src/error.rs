//! Error types for vaultmux operations.

use thiserror::Error;

/// Result type alias using [`VaultmuxError`].
pub type Result<T> = std::result::Result<T, VaultmuxError>;

/// Errors that can occur during vault operations.
///
/// All errors implement `std::error::Error` and can be chained with `source()`.
#[derive(Debug, Error)]
pub enum VaultmuxError {
    /// Item was not found in the vault.
    #[error("item not found: {0}")]
    NotFound(String),

    /// Item already exists (cannot create duplicate).
    #[error("item already exists: {0}")]
    AlreadyExists(String),

    /// Not authenticated with the backend.
    #[error("not authenticated")]
    NotAuthenticated,

    /// Session has expired and must be refreshed.
    #[error("session expired")]
    SessionExpired,

    /// Required CLI tool is not installed.
    #[error("backend CLI not installed: {0}")]
    BackendNotInstalled(String),

    /// Vault is locked and requires unlock.
    #[error("vault is locked")]
    BackendLocked,

    /// Permission denied for the operation.
    #[error("permission denied: {0}")]
    PermissionDenied(String),

    /// Operation is not supported by this backend.
    #[error("operation not supported by backend: {0}")]
    NotSupported(String),

    /// Item name contains invalid characters.
    #[error("invalid item name: {0}")]
    InvalidItemName(String),

    /// Backend operation failed with context.
    #[error("{backend}: {operation} {item}: {source}")]
    BackendOperation {
        /// Backend name
        backend: String,
        /// Operation name (get, create, delete, etc.)
        operation: String,
        /// Item name
        item: String,
        /// Underlying error
        #[source]
        source: Box<VaultmuxError>,
    },

    /// I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Command execution failed.
    #[error("command execution failed: {0}")]
    CommandFailed(String),

    /// Other error (catch-all).
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl VaultmuxError {
    /// Creates a backend operation error with context.
    ///
    /// This wraps an underlying error with information about which backend,
    /// operation, and item caused the failure.
    ///
    /// # Example
    ///
    /// ```
    /// use vaultmux::VaultmuxError;
    ///
    /// let err = VaultmuxError::NotFound("api-key".to_string());
    /// let wrapped = VaultmuxError::backend_op(
    ///     "bitwarden",
    ///     "get",
    ///     "api-key",
    ///     err
    /// );
    ///
    /// assert_eq!(
    ///     wrapped.to_string(),
    ///     "bitwarden: get api-key: item not found: api-key"
    /// );
    /// ```
    pub fn backend_op(
        backend: impl Into<String>,
        operation: impl Into<String>,
        item: impl Into<String>,
        err: VaultmuxError,
    ) -> Self {
        Self::BackendOperation {
            backend: backend.into(),
            operation: operation.into(),
            item: item.into(),
            source: Box::new(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_error_display() {
        let err = VaultmuxError::NotFound("test-item".to_string());
        assert_eq!(err.to_string(), "item not found: test-item");
    }

    #[test]
    fn test_backend_operation_error() {
        let inner = VaultmuxError::NotFound("api-key".to_string());
        let err = VaultmuxError::backend_op("bitwarden", "get", "api-key", inner);
        
        let error_string = err.to_string();
        assert!(error_string.contains("bitwarden"));
        assert!(error_string.contains("get"));
        assert!(error_string.contains("api-key"));
    }

    #[test]
    fn test_error_source_chain() {
        let inner = VaultmuxError::NotFound("test".to_string());
        let outer = VaultmuxError::backend_op("pass", "get", "test", inner);

        assert!(outer.source().is_some());
    }
}
