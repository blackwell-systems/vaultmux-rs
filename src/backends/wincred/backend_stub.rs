//! Stub implementation for non-Windows platforms.

use crate::{Backend, Config, Item, Result, Session, VaultmuxError};
use async_trait::async_trait;
use std::sync::Arc;

/// Stub Windows Credential Manager backend for non-Windows platforms.
///
/// This backend always returns errors indicating that it's only available on Windows.
pub struct WincredBackend {}

impl WincredBackend {
    /// Creates a new stub backend.
    pub fn new(_config: Config) -> Self {
        Self {}
    }
}

#[async_trait]
impl Backend for WincredBackend {
    fn name(&self) -> &str {
        "wincred"
    }

    async fn init(&mut self) -> Result<()> {
        Err(VaultmuxError::BackendNotInstalled(
            "Windows Credential Manager is only available on Windows".to_string(),
        ))
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        false
    }

    async fn authenticate(&mut self) -> Result<Arc<dyn Session>> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn sync(&mut self, _session: &dyn Session) -> Result<()> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn get_item(&self, _name: &str, _session: &dyn Session) -> Result<Item> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn get_notes(&self, _name: &str, _session: &dyn Session) -> Result<String> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn item_exists(&self, _name: &str, _session: &dyn Session) -> Result<bool> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn list_items(&self, _session: &dyn Session) -> Result<Vec<Item>> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn create_item(&mut self, _name: &str, _content: &str, _session: &dyn Session) -> Result<()> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn update_item(&mut self, _name: &str, _content: &str, _session: &dyn Session) -> Result<()> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn delete_item(&mut self, _name: &str, _session: &dyn Session) -> Result<()> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn list_locations(&self, _session: &dyn Session) -> Result<Vec<String>> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn location_exists(&self, _name: &str, _session: &dyn Session) -> Result<bool> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn create_location(&mut self, _name: &str, _session: &dyn Session) -> Result<()> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }

    async fn list_items_in_location(
        &self,
        _loc_type: &str,
        _loc_value: &str,
        _session: &dyn Session,
    ) -> Result<Vec<Item>> {
        Err(VaultmuxError::Other(anyhow::anyhow!(
            "Windows Credential Manager is only available on Windows"
        )))
    }
}
