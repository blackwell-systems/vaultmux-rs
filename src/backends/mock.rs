//! Mock backend for testing.
//!
//! This backend provides a complete in-memory implementation with error
//! injection capabilities for testing code that uses vaultmux.

use crate::*;
use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Mock backend for testing.
///
/// Stores all data in memory with support for error injection to simulate
/// failure conditions.
///
/// # Example
///
/// ```
/// use vaultmux::backends::mock::MockBackend;
/// use vaultmux::{Backend, VaultmuxError};
///
/// #[tokio::main]
/// async fn main() -> vaultmux::Result<()> {
///     let mut backend = MockBackend::new();
///     backend.init().await?;
///
///     // Pre-populate with test data
///     backend.set_item("test-key", "test-value").await;
///
///     // Test error conditions
///     backend.get_error = Some(VaultmuxError::PermissionDenied("test".to_string()));
///
///     let session = backend.authenticate().await?;
///     let result = backend.get_notes("test-key", &*session).await;
///     assert!(result.is_err());
///
///     Ok(())
/// }
/// ```
pub struct MockBackend {
    items: Arc<RwLock<HashMap<String, Item>>>,
    locations: Arc<RwLock<HashMap<String, bool>>>,

    /// Error to return from `authenticate()`
    pub auth_error: Option<VaultmuxError>,
    /// Error to return from `get_item()` and `get_notes()`
    pub get_error: Option<VaultmuxError>,
    /// Error to return from `create_item()`
    pub create_error: Option<VaultmuxError>,
    /// Error to return from `update_item()`
    pub update_error: Option<VaultmuxError>,
    /// Error to return from `delete_item()`
    pub delete_error: Option<VaultmuxError>,
}

impl MockBackend {
    /// Creates a new mock backend with empty storage.
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(HashMap::new())),
            locations: Arc::new(RwLock::new(HashMap::new())),
            auth_error: None,
            get_error: None,
            create_error: None,
            update_error: None,
            delete_error: None,
        }
    }

    /// Pre-populates the backend with an item.
    ///
    /// Useful for setting up test fixtures.
    pub async fn set_item(&self, name: impl Into<String>, content: impl Into<String>) {
        let item = Item::new_secure_note(name, content);
        let mut items = self.items.write().await;
        items.insert(item.name.clone(), item);
    }

    /// Pre-populates the backend with a location.
    pub async fn set_location(&self, name: impl Into<String>) {
        let mut locations = self.locations.write().await;
        locations.insert(name.into(), true);
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock session that never expires.
pub struct MockSession {
    token: String,
}

impl MockSession {
    fn new() -> Self {
        Self {
            token: "mock-session-token".to_string(),
        }
    }
}

#[async_trait]
impl Session for MockSession {
    fn token(&self) -> &str {
        &self.token
    }

    async fn is_valid(&self) -> bool {
        true
    }

    async fn refresh(&mut self) -> Result<()> {
        Ok(())
    }

    fn expires_at(&self) -> Option<chrono::DateTime<Utc>> {
        None
    }
}

#[async_trait]
impl Backend for MockBackend {
    fn name(&self) -> &str {
        "mock"
    }

    async fn init(&mut self) -> Result<()> {
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        self.auth_error.is_none()
    }

    async fn authenticate(&mut self) -> Result<Arc<dyn Session>> {
        if let Some(ref err) = self.auth_error {
            return Err(VaultmuxError::Other(anyhow::anyhow!("{}", err)));
        }
        Ok(Arc::new(MockSession::new()))
    }

    async fn sync(&mut self, _session: &dyn Session) -> Result<()> {
        Ok(())
    }

    async fn get_item(&self, name: &str, _session: &dyn Session) -> Result<Item> {
        if let Some(ref err) = self.get_error {
            return Err(VaultmuxError::Other(anyhow::anyhow!("{}", err)));
        }

        let items = self.items.read().await;
        items
            .get(name)
            .cloned()
            .ok_or_else(|| VaultmuxError::NotFound(name.to_string()))
    }

    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String> {
        let item = self.get_item(name, session).await?;
        item.notes
            .ok_or_else(|| VaultmuxError::NotFound(name.to_string()))
    }

    async fn item_exists(&self, name: &str, _session: &dyn Session) -> Result<bool> {
        let items = self.items.read().await;
        Ok(items.contains_key(name))
    }

    async fn list_items(&self, _session: &dyn Session) -> Result<Vec<Item>> {
        let items = self.items.read().await;
        Ok(items.values().cloned().collect())
    }

    async fn create_item(
        &mut self,
        name: &str,
        content: &str,
        _session: &dyn Session,
    ) -> Result<()> {
        if let Some(ref err) = self.create_error {
            return Err(VaultmuxError::Other(anyhow::anyhow!("{}", err)));
        }

        let mut items = self.items.write().await;
        if items.contains_key(name) {
            return Err(VaultmuxError::AlreadyExists(name.to_string()));
        }

        let item = Item::new_secure_note(name, content);
        items.insert(name.to_string(), item);
        Ok(())
    }

    async fn update_item(
        &mut self,
        name: &str,
        content: &str,
        _session: &dyn Session,
    ) -> Result<()> {
        if let Some(ref err) = self.update_error {
            return Err(VaultmuxError::Other(anyhow::anyhow!("{}", err)));
        }

        let mut items = self.items.write().await;
        let item = items
            .get_mut(name)
            .ok_or_else(|| VaultmuxError::NotFound(name.to_string()))?;

        item.notes = Some(content.to_string());
        item.modified = Some(Utc::now());
        Ok(())
    }

    async fn delete_item(&mut self, name: &str, _session: &dyn Session) -> Result<()> {
        if let Some(ref err) = self.delete_error {
            return Err(VaultmuxError::Other(anyhow::anyhow!("{}", err)));
        }

        let mut items = self.items.write().await;
        items
            .remove(name)
            .ok_or_else(|| VaultmuxError::NotFound(name.to_string()))?;
        Ok(())
    }

    async fn list_locations(&self, _session: &dyn Session) -> Result<Vec<String>> {
        let locations = self.locations.read().await;
        Ok(locations.keys().cloned().collect())
    }

    async fn location_exists(&self, name: &str, _session: &dyn Session) -> Result<bool> {
        let locations = self.locations.read().await;
        Ok(locations.contains_key(name))
    }

    async fn create_location(&mut self, name: &str, _session: &dyn Session) -> Result<()> {
        let mut locations = self.locations.write().await;
        if locations.contains_key(name) {
            return Err(VaultmuxError::AlreadyExists(name.to_string()));
        }
        locations.insert(name.to_string(), true);
        Ok(())
    }

    async fn list_items_in_location(
        &self,
        _loc_type: &str,
        loc_value: &str,
        _session: &dyn Session,
    ) -> Result<Vec<Item>> {
        let items = self.items.read().await;
        Ok(items
            .values()
            .filter(|item| item.location.as_deref() == Some(loc_value))
            .cloned()
            .collect())
    }
}

/// Registers the mock backend with the factory.
pub fn register() {
    crate::factory::register_backend("mock", |_cfg| Ok(Box::new(MockBackend::new())));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_backend_create_and_get() {
        let mut backend = MockBackend::new();
        backend.init().await.unwrap();

        let session = backend.authenticate().await.unwrap();

        backend
            .create_item("test-key", "test-value", &*session)
            .await
            .unwrap();

        let notes = backend.get_notes("test-key", &*session).await.unwrap();
        assert_eq!(notes, "test-value");
    }

    #[tokio::test]
    async fn test_mock_backend_update() {
        let mut backend = MockBackend::new();
        backend.set_item("test-key", "original").await;

        let session = backend.authenticate().await.unwrap();

        backend
            .update_item("test-key", "updated", &*session)
            .await
            .unwrap();

        let notes = backend.get_notes("test-key", &*session).await.unwrap();
        assert_eq!(notes, "updated");
    }

    #[tokio::test]
    async fn test_mock_backend_delete() {
        let mut backend = MockBackend::new();
        backend.set_item("test-key", "value").await;

        let session = backend.authenticate().await.unwrap();

        backend.delete_item("test-key", &*session).await.unwrap();

        let result = backend.get_notes("test-key", &*session).await;
        assert!(matches!(result, Err(VaultmuxError::NotFound(_))));
    }

    #[tokio::test]
    async fn test_mock_backend_list() {
        let mut backend = MockBackend::new();
        backend.set_item("key1", "value1").await;
        backend.set_item("key2", "value2").await;

        let session = backend.authenticate().await.unwrap();

        let items = backend.list_items(&*session).await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_error_injection() {
        let mut backend = MockBackend::new();
        backend.get_error = Some(VaultmuxError::PermissionDenied("test".to_string()));

        let session = backend.authenticate().await.unwrap();
        let result = backend.get_notes("anything", &*session).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_locations() {
        let mut backend = MockBackend::new();
        backend.set_location("work").await;

        let session = backend.authenticate().await.unwrap();

        let exists = backend.location_exists("work", &*session).await.unwrap();
        assert!(exists);

        let locations = backend.list_locations(&*session).await.unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0], "work");
    }
}
