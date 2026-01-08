//! Backend trait definition for vault integrations.
//!
//! This module defines the core [`Backend`] trait that all vault implementations
//! must satisfy. The trait provides a unified interface for authentication,
//! item management, and location (folder/vault) operations.

use crate::{Item, Result, Session};
use async_trait::async_trait;
use std::sync::Arc;

/// Backend represents a secret storage backend.
///
/// All implementations must be `Send + Sync` to support concurrent access
/// across async tasks.
///
/// # Implementations
///
/// - **CLI-based**: Bitwarden (`bw`), 1Password (`op`), pass (`pass`)
/// - **OS-native**: Windows Credential Manager (PowerShell)
/// - **SDK-based**: AWS Secrets Manager, GCP Secret Manager, Azure Key Vault
/// - **Testing**: Mock backend with error injection
///
/// # Example
///
/// ```no_run
/// use vaultmux::{Backend, Config, BackendType};
///
/// #[tokio::main]
/// async fn main() -> vaultmux::Result<()> {
///     let config = Config::new(BackendType::Pass);
///     let mut backend = vaultmux::factory::new_backend(config)?;
///     
///     backend.init().await?;
///     let session = backend.authenticate().await?;
///     
///     backend.create_item("api-key", "secret-value", &*session).await?;
///     let notes = backend.get_notes("api-key", &*session).await?;
///     
///     println!("Retrieved: {}", notes);
///     Ok(())
/// }
/// ```
#[async_trait]
pub trait Backend: Send + Sync {
    // ========================================================================
    // Metadata
    // ========================================================================

    /// Returns the backend name (e.g., "bitwarden", "pass", "awssecrets").
    fn name(&self) -> &str;

    // ========================================================================
    // Lifecycle
    // ========================================================================

    /// Initializes the backend.
    ///
    /// For CLI backends, this checks if the command-line tool is installed.
    /// For SDK backends, this validates configuration and sets up clients.
    ///
    /// # Errors
    ///
    /// Returns [`VaultmuxError::BackendNotInstalled`](crate::VaultmuxError::BackendNotInstalled)
    /// if the required CLI tool or SDK is not available.
    async fn init(&mut self) -> Result<()>;

    /// Closes the backend and releases resources.
    ///
    /// For most backends this is a no-op, but SDK backends may need to
    /// close connections or flush buffers.
    async fn close(&mut self) -> Result<()>;

    // ========================================================================
    // Authentication
    // ========================================================================

    /// Checks if the backend is currently authenticated.
    ///
    /// For CLI backends, this may shell out to check lock status.
    /// For SDK backends, this checks credential validity.
    ///
    /// Note: This operation may be cached for performance (typically 5 seconds).
    async fn is_authenticated(&self) -> bool;

    /// Authenticates with the backend and returns a session.
    ///
    /// For CLI backends (Bitwarden, 1Password), this prompts for a password
    /// or biometric unlock if the vault is locked.
    ///
    /// For SDK backends (AWS, GCP, Azure), this uses existing credentials
    /// from environment variables or SDK credential chains.
    ///
    /// For pass, this always succeeds immediately (authentication is handled
    /// by the GPG agent).
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::NotAuthenticated`](crate::VaultmuxError::NotAuthenticated):
    ///   Authentication failed (wrong password, no credentials, etc.)
    /// - [`VaultmuxError::BackendLocked`](crate::VaultmuxError::BackendLocked):
    ///   Vault is locked and cannot be unlocked automatically
    async fn authenticate(&mut self) -> Result<Arc<dyn Session>>;

    /// Synchronizes with the remote server.
    ///
    /// - Bitwarden: Runs `bw sync`
    /// - 1Password: No-op (automatically synced)
    /// - pass: Can run `pass git pull` (depends on implementation)
    /// - Cloud backends: No-op (always synchronized)
    ///
    /// # Errors
    ///
    /// Returns an error if synchronization fails (network error, conflicts, etc.).
    async fn sync(&mut self, session: &dyn Session) -> Result<()>;

    // ========================================================================
    // Item Operations (CRUD)
    // ========================================================================

    /// Retrieves a complete item by name.
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::NotFound`](crate::VaultmuxError::NotFound):
    ///   Item does not exist
    /// - [`VaultmuxError::SessionExpired`](crate::VaultmuxError::SessionExpired):
    ///   Session is no longer valid
    async fn get_item(&self, name: &str, session: &dyn Session) -> Result<Item>;

    /// Retrieves only the notes field from an item.
    ///
    /// This is more efficient than `get_item()` when you only need the content.
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::NotFound`](crate::VaultmuxError::NotFound):
    ///   Item does not exist
    /// - [`VaultmuxError::SessionExpired`](crate::VaultmuxError::SessionExpired):
    ///   Session is no longer valid
    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String>;

    /// Checks if an item exists.
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::SessionExpired`](crate::VaultmuxError::SessionExpired):
    ///   Session is no longer valid
    async fn item_exists(&self, name: &str, session: &dyn Session) -> Result<bool>;

    /// Lists all items in the vault.
    ///
    /// Note: For large vaults, this may be slow. Consider using location-based
    /// filtering with `list_items_in_location()` if available.
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::SessionExpired`](crate::VaultmuxError::SessionExpired):
    ///   Session is no longer valid
    async fn list_items(&self, session: &dyn Session) -> Result<Vec<Item>>;

    // ========================================================================
    // Mutations
    // ========================================================================

    /// Creates a new item.
    ///
    /// The item type is always [`ItemType::SecureNote`](crate::ItemType::SecureNote)
    /// for simplicity. The `content` is stored in the notes field.
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::AlreadyExists`](crate::VaultmuxError::AlreadyExists):
    ///   An item with this name already exists
    /// - [`VaultmuxError::SessionExpired`](crate::VaultmuxError::SessionExpired):
    ///   Session is no longer valid
    /// - [`VaultmuxError::InvalidItemName`](crate::VaultmuxError::InvalidItemName):
    ///   Item name contains invalid characters
    async fn create_item(&mut self, name: &str, content: &str, session: &dyn Session)
        -> Result<()>;

    /// Updates an existing item's content.
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::NotFound`](crate::VaultmuxError::NotFound):
    ///   Item does not exist
    /// - [`VaultmuxError::SessionExpired`](crate::VaultmuxError::SessionExpired):
    ///   Session is no longer valid
    async fn update_item(&mut self, name: &str, content: &str, session: &dyn Session)
        -> Result<()>;

    /// Deletes an item.
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::NotFound`](crate::VaultmuxError::NotFound):
    ///   Item does not exist
    /// - [`VaultmuxError::SessionExpired`](crate::VaultmuxError::SessionExpired):
    ///   Session is no longer valid
    async fn delete_item(&mut self, name: &str, session: &dyn Session) -> Result<()>;

    // ========================================================================
    // Location Management (Optional)
    // ========================================================================

    /// Lists all locations (folders, vaults, directories).
    ///
    /// - Bitwarden: Folders
    /// - 1Password: Vaults
    /// - pass: Directories
    /// - Other backends: May return [`VaultmuxError::NotSupported`](crate::VaultmuxError::NotSupported)
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::NotSupported`](crate::VaultmuxError::NotSupported):
    ///   Backend does not support locations
    /// - [`VaultmuxError::SessionExpired`](crate::VaultmuxError::SessionExpired):
    ///   Session is no longer valid
    async fn list_locations(&self, session: &dyn Session) -> Result<Vec<String>>;

    /// Checks if a location exists.
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::NotSupported`](crate::VaultmuxError::NotSupported):
    ///   Backend does not support locations
    async fn location_exists(&self, name: &str, session: &dyn Session) -> Result<bool>;

    /// Creates a new location.
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::AlreadyExists`](crate::VaultmuxError::AlreadyExists):
    ///   Location already exists
    /// - [`VaultmuxError::NotSupported`](crate::VaultmuxError::NotSupported):
    ///   Backend does not support locations
    async fn create_location(&mut self, name: &str, session: &dyn Session) -> Result<()>;

    /// Lists items in a specific location.
    ///
    /// # Arguments
    ///
    /// - `loc_type`: Location type ("folder", "vault", "directory")
    /// - `loc_value`: Location name or ID
    /// - `session`: Valid session
    ///
    /// # Errors
    ///
    /// - [`VaultmuxError::NotFound`](crate::VaultmuxError::NotFound):
    ///   Location does not exist
    /// - [`VaultmuxError::NotSupported`](crate::VaultmuxError::NotSupported):
    ///   Backend does not support locations
    async fn list_items_in_location(
        &self,
        loc_type: &str,
        loc_value: &str,
        session: &dyn Session,
    ) -> Result<Vec<Item>>;
}
