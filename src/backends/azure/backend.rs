//! Azure Key Vault backend implementation.

use crate::backends::azure::AzureSession;
use crate::validation::validate_item_name;
use crate::{Backend, Config, Item, ItemType, Result, Session, VaultmuxError};
use async_trait::async_trait;
use azure_identity::DefaultAzureCredential;
use azure_security_keyvault::SecretClient;
use futures::StreamExt;
use std::sync::Arc;

/// Azure Key Vault backend.
///
/// Integrates with Azure Key Vault using the official Microsoft SDK.
pub struct AzureBackend {
    client: Option<SecretClient>,
    vault_url: String,
    prefix: String,
}

impl AzureBackend {
    /// Creates a new Azure Key Vault backend from configuration.
    pub fn new(config: Config) -> Self {
        let vault_url = config.options.get("vault_url").cloned().unwrap_or_else(|| {
            std::env::var("AZURE_KEYVAULT_URL").unwrap_or_else(|_| "".to_string())
        });

        let prefix = config
            .options
            .get("prefix")
            .cloned()
            .unwrap_or(config.prefix);

        Self {
            client: None,
            vault_url,
            prefix,
        }
    }

    /// Constructs the full secret name with prefix.
    fn secret_name(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}{}", self.prefix, name)
        }
    }

    /// Gets the client.
    fn client(&self) -> Result<&SecretClient> {
        self.client.as_ref().ok_or(VaultmuxError::NotAuthenticated)
    }
}

#[async_trait]
impl Backend for AzureBackend {
    fn name(&self) -> &str {
        "azurekeyvault"
    }

    async fn init(&mut self) -> Result<()> {
        if self.vault_url.is_empty() {
            return Err(VaultmuxError::Other(anyhow::anyhow!(
                "Azure vault_url is required. Set via config or AZURE_KEYVAULT_URL environment variable"
            )));
        }

        // Create credential using DefaultAzureCredential
        let credential = Arc::new(DefaultAzureCredential::create(Default::default()).map_err(
            |e| VaultmuxError::Other(anyhow::anyhow!("Failed to create Azure credentials: {}", e)),
        )?);

        // Create Secret client
        self.client = Some(SecretClient::new(&self.vault_url, credential).map_err(|e| {
            VaultmuxError::Other(anyhow::anyhow!("Failed to create Secret client: {}", e))
        })?);

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.client = None;
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        self.client.is_some()
    }

    async fn authenticate(&mut self) -> Result<Arc<dyn Session>> {
        // Azure uses DefaultAzureCredential, no explicit authentication needed
        Ok(Arc::new(AzureSession::new(self.vault_url.clone())))
    }

    async fn sync(&mut self, _session: &dyn Session) -> Result<()> {
        // Azure Key Vault is always synchronized
        Ok(())
    }

    async fn get_item(&self, name: &str, _session: &dyn Session) -> Result<Item> {
        validate_item_name(name)?;

        let client = self.client()?;
        let secret_name = self.secret_name(name);

        // Get secret
        let secret = client.get(secret_name).into_future().await.map_err(|e| {
            if e.to_string().contains("SecretNotFound") || e.to_string().contains("404") {
                VaultmuxError::NotFound(name.to_string())
            } else {
                VaultmuxError::Other(anyhow::anyhow!("Azure error: {}", e))
            }
        })?;

        // Parse timestamps (Azure uses time::OffsetDateTime)
        let created =
            chrono::DateTime::from_timestamp(secret.attributes.created_on.unix_timestamp(), 0);

        let modified =
            chrono::DateTime::from_timestamp(secret.attributes.updated_on.unix_timestamp(), 0);

        Ok(Item {
            id: secret.id,
            name: name.to_string(),
            item_type: ItemType::SecureNote,
            notes: Some(secret.value),
            fields: None,
            location: None,
            created,
            modified,
        })
    }

    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String> {
        let item = self.get_item(name, session).await?;
        item.notes
            .ok_or_else(|| VaultmuxError::NotFound(format!("{} has no value", name)))
    }

    async fn item_exists(&self, name: &str, _session: &dyn Session) -> Result<bool> {
        validate_item_name(name)?;

        let client = self.client()?;
        let secret_name = self.secret_name(name);

        match client.get(secret_name).into_future().await {
            Ok(_) => Ok(true),
            Err(e) if e.to_string().contains("SecretNotFound") || e.to_string().contains("404") => {
                Ok(false)
            }
            Err(e) => Err(VaultmuxError::Other(anyhow::anyhow!("Azure error: {}", e))),
        }
    }

    async fn list_items(&self, _session: &dyn Session) -> Result<Vec<Item>> {
        let client = self.client()?;

        let mut items = Vec::new();

        // List all secrets
        let mut secrets_stream = client.list_secrets().into_stream();
        let mut secrets = Vec::new();

        while let Some(result) = secrets_stream.next().await {
            secrets.push(result);
        }

        for result in secrets {
            match result {
                Ok(secret_list) => {
                    for secret_item in secret_list.value {
                        let id = &secret_item.id;
                        // Extract secret name from ID (last segment of URL path)
                        if let Some(full_name) = id.rsplit('/').next() {
                            // Filter by prefix
                            if let Some(name) = full_name.strip_prefix(&self.prefix) {
                                items.push(Item {
                                    id: id.clone(),
                                    name: name.to_string(),
                                    item_type: ItemType::SecureNote,
                                    notes: None,
                                    fields: None,
                                    location: None,
                                    created: None,
                                    modified: None,
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    return Err(VaultmuxError::Other(anyhow::anyhow!(
                        "Failed to list secrets: {}",
                        e
                    )))
                }
            }
        }

        Ok(items)
    }

    async fn create_item(
        &mut self,
        name: &str,
        content: &str,
        _session: &dyn Session,
    ) -> Result<()> {
        validate_item_name(name)?;

        if self.item_exists(name, _session).await? {
            return Err(VaultmuxError::AlreadyExists(name.to_string()));
        }

        let client = self.client()?;
        let secret_name = self.secret_name(name);

        client
            .set(secret_name, content)
            .into_future()
            .await
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to create secret: {}", e)))?;

        Ok(())
    }

    async fn update_item(
        &mut self,
        name: &str,
        content: &str,
        _session: &dyn Session,
    ) -> Result<()> {
        validate_item_name(name)?;

        if !self.item_exists(name, _session).await? {
            return Err(VaultmuxError::NotFound(name.to_string()));
        }

        let client = self.client()?;
        let secret_name = self.secret_name(name);

        // Azure creates a new version when setting an existing secret
        client
            .set(secret_name, content)
            .into_future()
            .await
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to update secret: {}", e)))?;

        Ok(())
    }

    async fn delete_item(&mut self, name: &str, _session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        let client = self.client()?;
        let secret_name = self.secret_name(name);

        client
            .delete(secret_name)
            .into_future()
            .await
            .map_err(|e| {
                if e.to_string().contains("SecretNotFound") || e.to_string().contains("404") {
                    VaultmuxError::NotFound(name.to_string())
                } else {
                    VaultmuxError::Other(anyhow::anyhow!("Failed to delete secret: {}", e))
                }
            })?;

        Ok(())
    }

    async fn list_locations(&self, _session: &dyn Session) -> Result<Vec<String>> {
        Err(VaultmuxError::NotSupported(
            "Azure Key Vault does not support locations (secrets are vault-scoped)".to_string(),
        ))
    }

    async fn location_exists(&self, _name: &str, _session: &dyn Session) -> Result<bool> {
        Err(VaultmuxError::NotSupported(
            "Azure Key Vault does not support locations".to_string(),
        ))
    }

    async fn create_location(&mut self, _name: &str, _session: &dyn Session) -> Result<()> {
        Err(VaultmuxError::NotSupported(
            "Azure Key Vault does not support locations".to_string(),
        ))
    }

    async fn list_items_in_location(
        &self,
        _loc_type: &str,
        _loc_value: &str,
        _session: &dyn Session,
    ) -> Result<Vec<Item>> {
        Err(VaultmuxError::NotSupported(
            "Azure Key Vault does not support locations".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_name() {
        let config = Config::new(crate::BackendType::AzureKeyVault)
            .with_option("vault_url", "https://myvault.vault.azure.net")
            .with_option("prefix", "app-");
        let backend = AzureBackend::new(config);

        assert_eq!(backend.secret_name("api-key"), "app-api-key");
    }

    #[test]
    fn test_secret_name_no_prefix() {
        let config = Config::new(crate::BackendType::AzureKeyVault)
            .with_option("vault_url", "https://myvault.vault.azure.net")
            .with_option("prefix", "");
        let backend = AzureBackend::new(config);

        assert_eq!(backend.secret_name("api-key"), "api-key");
    }
}
