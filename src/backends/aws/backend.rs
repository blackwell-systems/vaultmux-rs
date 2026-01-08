//! AWS Secrets Manager backend implementation.

use crate::backends::aws::AWSSession;
use crate::validation::validate_item_name;
use crate::{Backend, Config, Item, ItemType, Result, Session, VaultmuxError};
use async_trait::async_trait;
use aws_sdk_secretsmanager::Client;
use std::sync::Arc;

/// AWS Secrets Manager backend.
///
/// Integrates with AWS Secrets Manager using the official AWS SDK.
pub struct AWSBackend {
    client: Option<Client>,
    region: String,
    prefix: String,
    endpoint: Option<String>,
}

impl AWSBackend {
    /// Creates a new AWS Secrets Manager backend from configuration.
    pub fn new(config: Config) -> Self {
        let region = config
            .options
            .get("region")
            .cloned()
            .unwrap_or_else(|| "us-east-1".to_string());

        let prefix = config
            .options
            .get("prefix")
            .cloned()
            .unwrap_or(config.prefix);

        let endpoint = config.options.get("endpoint").cloned();

        Self {
            client: None,
            region,
            prefix,
            endpoint,
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
}

#[async_trait]
impl Backend for AWSBackend {
    fn name(&self) -> &str {
        "awssecrets"
    }

    async fn init(&mut self) -> Result<()> {
        // Set up AWS SDK config
        let mut config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(self.region.clone()));

        // Use custom endpoint if provided (for LocalStack testing)
        if let Some(ref endpoint) = self.endpoint {
            config_loader = config_loader.endpoint_url(endpoint);
        }

        let config = config_loader.load().await;
        self.client = Some(Client::new(&config));

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.client = None;
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        // AWS SDK handles credentials automatically
        self.client.is_some()
    }

    async fn authenticate(&mut self) -> Result<Arc<dyn Session>> {
        // AWS uses credentials from environment/config
        // No explicit authentication needed
        Ok(Arc::new(AWSSession::new(self.region.clone())))
    }

    async fn sync(&mut self, _session: &dyn Session) -> Result<()> {
        // AWS Secrets Manager is always synchronized
        Ok(())
    }

    async fn get_item(&self, name: &str, _session: &dyn Session) -> Result<Item> {
        validate_item_name(name)?;

        let client = self
            .client
            .as_ref()
            .ok_or(VaultmuxError::NotAuthenticated)?;

        let secret_name = self.secret_name(name);

        let response = client
            .describe_secret()
            .secret_id(&secret_name)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("ResourceNotFoundException") {
                    VaultmuxError::NotFound(name.to_string())
                } else {
                    VaultmuxError::Other(anyhow::anyhow!("AWS error: {}", e))
                }
            })?;

        // Get the secret value
        let value_response = client
            .get_secret_value()
            .secret_id(&secret_name)
            .send()
            .await
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to get secret value: {}", e)))?;

        let secret_string = value_response
            .secret_string()
            .ok_or_else(|| VaultmuxError::Other(anyhow::anyhow!("Secret has no string value")))?;

        Ok(Item {
            id: response.arn().unwrap_or("").to_string(),
            name: name.to_string(),
            item_type: ItemType::SecureNote,
            notes: Some(secret_string.to_string()),
            fields: None,
            location: None,
            created: response.created_date().and_then(|d| {
                chrono::DateTime::from_timestamp(d.secs(), d.subsec_nanos() as u32)
            }),
            modified: response.last_changed_date().and_then(|d| {
                chrono::DateTime::from_timestamp(d.secs(), d.subsec_nanos() as u32)
            }),
        })
    }

    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String> {
        let item = self.get_item(name, session).await?;
        item.notes
            .ok_or_else(|| VaultmuxError::NotFound(format!("{} has no value", name)))
    }

    async fn item_exists(&self, name: &str, _session: &dyn Session) -> Result<bool> {
        validate_item_name(name)?;

        let client = self
            .client
            .as_ref()
            .ok_or(VaultmuxError::NotAuthenticated)?;

        let secret_name = self.secret_name(name);

        match client.describe_secret().secret_id(&secret_name).send().await {
            Ok(_) => Ok(true),
            Err(e) if e.to_string().contains("ResourceNotFoundException") => Ok(false),
            Err(e) => Err(VaultmuxError::Other(anyhow::anyhow!("AWS error: {}", e))),
        }
    }

    async fn list_items(&self, _session: &dyn Session) -> Result<Vec<Item>> {
        let client = self
            .client
            .as_ref()
            .ok_or(VaultmuxError::NotAuthenticated)?;

        let mut items = Vec::new();
        let mut next_token: Option<String> = None;

        loop {
            let mut request = client.list_secrets();
            
            if let Some(token) = next_token {
                request = request.next_token(token);
            }

            let response = request
                .send()
                .await
                .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to list secrets: {}", e)))?;

            for secret in response.secret_list() {
                let Some(full_name) = secret.name() else { continue };
                
                // Filter by prefix
                if let Some(name) = full_name.strip_prefix(&self.prefix) {
                    items.push(Item {
                        id: secret.arn().unwrap_or("").to_string(),
                        name: name.to_string(),
                        item_type: ItemType::SecureNote,
                        notes: None, // Don't fetch values for list
                        fields: None,
                        location: None,
                        created: secret.created_date().and_then(|d| {
                            chrono::DateTime::from_timestamp(d.secs(), d.subsec_nanos() as u32)
                        }),
                        modified: secret.last_changed_date().and_then(|d| {
                            chrono::DateTime::from_timestamp(d.secs(), d.subsec_nanos() as u32)
                        }),
                    });
                }
            }

            // Check for more results
            next_token = response.next_token().map(|s| s.to_string());
            if next_token.is_none() {
                break;
            }
        }

        Ok(items)
    }

    async fn create_item(&mut self, name: &str, content: &str, _session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        let client = self
            .client
            .as_ref()
            .ok_or(VaultmuxError::NotAuthenticated)?;

        // Check if exists
        if self.item_exists(name, _session).await? {
            return Err(VaultmuxError::AlreadyExists(name.to_string()));
        }

        let secret_name = self.secret_name(name);

        client
            .create_secret()
            .name(&secret_name)
            .secret_string(content)
            .send()
            .await
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to create secret: {}", e)))?;

        Ok(())
    }

    async fn update_item(&mut self, name: &str, content: &str, _session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        let client = self
            .client
            .as_ref()
            .ok_or(VaultmuxError::NotAuthenticated)?;

        // Check if exists
        if !self.item_exists(name, _session).await? {
            return Err(VaultmuxError::NotFound(name.to_string()));
        }

        let secret_name = self.secret_name(name);

        client
            .put_secret_value()
            .secret_id(&secret_name)
            .secret_string(content)
            .send()
            .await
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to update secret: {}", e)))?;

        Ok(())
    }

    async fn delete_item(&mut self, name: &str, _session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        let client = self
            .client
            .as_ref()
            .ok_or(VaultmuxError::NotAuthenticated)?;

        let secret_name = self.secret_name(name);

        client
            .delete_secret()
            .secret_id(&secret_name)
            .force_delete_without_recovery(true)
            .send()
            .await
            .map_err(|e| {
                if e.to_string().contains("ResourceNotFoundException") {
                    VaultmuxError::NotFound(name.to_string())
                } else {
                    VaultmuxError::Other(anyhow::anyhow!("Failed to delete secret: {}", e))
                }
            })?;

        Ok(())
    }

    async fn list_locations(&self, _session: &dyn Session) -> Result<Vec<String>> {
        // AWS Secrets Manager doesn't have folders
        // Could use tags for organization
        Err(VaultmuxError::NotSupported(
            "AWS Secrets Manager does not support locations (use tags instead)".to_string(),
        ))
    }

    async fn location_exists(&self, _name: &str, _session: &dyn Session) -> Result<bool> {
        Err(VaultmuxError::NotSupported(
            "AWS Secrets Manager does not support locations".to_string(),
        ))
    }

    async fn create_location(&mut self, _name: &str, _session: &dyn Session) -> Result<()> {
        Err(VaultmuxError::NotSupported(
            "AWS Secrets Manager does not support locations".to_string(),
        ))
    }

    async fn list_items_in_location(
        &self,
        _loc_type: &str,
        _loc_value: &str,
        _session: &dyn Session,
    ) -> Result<Vec<Item>> {
        Err(VaultmuxError::NotSupported(
            "AWS Secrets Manager does not support locations".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_name() {
        let config = Config::new(crate::BackendType::AWSSecretsManager)
            .with_option("prefix", "myapp/");
        let backend = AWSBackend::new(config);

        assert_eq!(backend.secret_name("api-key"), "myapp/api-key");
    }

    #[test]
    fn test_secret_name_no_prefix() {
        let config = Config::new(crate::BackendType::AWSSecretsManager)
            .with_option("prefix", "");
        let backend = AWSBackend::new(config);

        assert_eq!(backend.secret_name("api-key"), "api-key");
    }
}
