//! GCP Secret Manager backend implementation.

use crate::backends::gcp::GCPSession;
use crate::validation::validate_item_name;
use crate::{Backend, Config, Item, ItemType, Result, Session, VaultmuxError};
use async_trait::async_trait;
use google_secretmanager1::api::{AddSecretVersionRequest, Replication, Secret};
use google_secretmanager1::{hyper, hyper_rustls, oauth2, SecretManager};
use std::sync::Arc;

/// GCP Secret Manager backend.
///
/// Integrates with Google Cloud Secret Manager using the official API client.
pub struct GCPBackend {
    hub: Option<SecretManager<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>>,
    project_id: String,
    prefix: String,
}

impl GCPBackend {
    /// Creates a new GCP Secret Manager backend from configuration.
    pub fn new(config: Config) -> Self {
        let project_id = config
            .options
            .get("project_id")
            .cloned()
            .unwrap_or_else(|| std::env::var("GCP_PROJECT").unwrap_or_else(|_| "".to_string()));

        let prefix = config
            .options
            .get("prefix")
            .cloned()
            .unwrap_or(config.prefix);

        Self {
            hub: None,
            project_id,
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

    /// Constructs the full resource path for a secret.
    fn secret_path(&self, name: &str) -> String {
        let secret_name = self.secret_name(name);
        format!("projects/{}/secrets/{}", self.project_id, secret_name)
    }

    /// Constructs the resource path for a secret version.
    fn version_path(&self, name: &str, version: &str) -> String {
        format!("{}/versions/{}", self.secret_path(name), version)
    }

    /// Gets the hub (API client).
    fn hub(
        &self,
    ) -> Result<&SecretManager<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>> {
        self.hub.as_ref().ok_or(VaultmuxError::NotAuthenticated)
    }
}

#[async_trait]
impl Backend for GCPBackend {
    fn name(&self) -> &str {
        "gcpsecrets"
    }

    async fn init(&mut self) -> Result<()> {
        if self.project_id.is_empty() {
            return Err(VaultmuxError::Other(anyhow::anyhow!(
                "GCP project_id is required. Set via config or GCP_PROJECT environment variable"
            )));
        }

        // Create hyper client
        let client = hyper::Client::builder().build(
            hyper_rustls::HttpsConnectorBuilder::new()
                .with_native_roots()
                .unwrap()
                .https_or_http()
                .enable_http1()
                .build(),
        );

        // Get application default credentials
        let opts = oauth2::ApplicationDefaultCredentialsFlowOpts::default();
        let auth_builder = oauth2::ApplicationDefaultCredentialsAuthenticator::builder(opts).await;

        let auth = match auth_builder {
            oauth2::authenticator::ApplicationDefaultCredentialsTypes::InstanceMetadata(auth) => {
                auth.build().await.map_err(|e| {
                    VaultmuxError::Other(anyhow::anyhow!("Failed to build GCP auth: {}", e))
                })?
            }
            oauth2::authenticator::ApplicationDefaultCredentialsTypes::ServiceAccount(auth) => {
                auth.build().await.map_err(|e| {
                    VaultmuxError::Other(anyhow::anyhow!("Failed to build GCP auth: {}", e))
                })?
            }
        };

        self.hub = Some(SecretManager::new(client, auth));
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.hub = None;
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        self.hub.is_some()
    }

    async fn authenticate(&mut self) -> Result<Arc<dyn Session>> {
        // GCP uses ADC, no explicit authentication needed
        Ok(Arc::new(GCPSession::new(self.project_id.clone())))
    }

    async fn sync(&mut self, _session: &dyn Session) -> Result<()> {
        // GCP Secret Manager is always synchronized
        Ok(())
    }

    async fn get_item(&self, name: &str, _session: &dyn Session) -> Result<Item> {
        validate_item_name(name)?;

        let hub = self.hub()?;
        let secret_path = self.secret_path(name);

        // Get secret metadata
        let (_, secret) = hub
            .projects()
            .secrets_get(&secret_path)
            .doit()
            .await
            .map_err(|e| {
                if e.to_string().contains("NOT_FOUND") {
                    VaultmuxError::NotFound(name.to_string())
                } else {
                    VaultmuxError::Other(anyhow::anyhow!("GCP error: {}", e))
                }
            })?;

        // Get latest version value
        let version_path = self.version_path(name, "latest");
        let (_, version_access) = hub
            .projects()
            .secrets_versions_access(&version_path)
            .doit()
            .await
            .map_err(|e| {
                VaultmuxError::Other(anyhow::anyhow!("Failed to access secret value: {}", e))
            })?;

        let payload = version_access
            .payload
            .and_then(|p| p.data)
            .and_then(|data| String::from_utf8(data).ok())
            .ok_or_else(|| VaultmuxError::Other(anyhow::anyhow!("Secret has no value")))?;

        // GCP API already returns DateTime<Utc>
        let created = secret.create_time;

        Ok(Item {
            id: secret.name.unwrap_or_default(),
            name: name.to_string(),
            item_type: ItemType::SecureNote,
            notes: Some(payload),
            fields: None,
            location: None,
            created,
            modified: None, // GCP doesn't track modification time at secret level
        })
    }

    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String> {
        let item = self.get_item(name, session).await?;
        item.notes
            .ok_or_else(|| VaultmuxError::NotFound(format!("{} has no value", name)))
    }

    async fn item_exists(&self, name: &str, _session: &dyn Session) -> Result<bool> {
        validate_item_name(name)?;

        let hub = self.hub()?;
        let secret_path = self.secret_path(name);

        match hub.projects().secrets_get(&secret_path).doit().await {
            Ok(_) => Ok(true),
            Err(e) if e.to_string().contains("NOT_FOUND") => Ok(false),
            Err(e) => Err(VaultmuxError::Other(anyhow::anyhow!("GCP error: {}", e))),
        }
    }

    async fn list_items(&self, _session: &dyn Session) -> Result<Vec<Item>> {
        let hub = self.hub()?;
        let parent = format!("projects/{}", self.project_id);

        let mut items = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut req = hub.projects().secrets_list(&parent);
            if let Some(token) = page_token {
                req = req.page_token(&token);
            }

            let (_, response) = req.doit().await.map_err(|e| {
                VaultmuxError::Other(anyhow::anyhow!("Failed to list secrets: {}", e))
            })?;

            if let Some(secrets) = response.secrets {
                for secret in secrets {
                    if let Some(full_name) = secret.name {
                        // Extract secret ID from full name
                        if let Some(secret_id) = full_name.rsplit('/').next() {
                            // Filter by prefix
                            if let Some(name) = secret_id.strip_prefix(&self.prefix) {
                                let created = secret.create_time;

                                items.push(Item {
                                    id: full_name.clone(),
                                    name: name.to_string(),
                                    item_type: ItemType::SecureNote,
                                    notes: None,
                                    fields: None,
                                    location: None,
                                    created,
                                    modified: None,
                                });
                            }
                        }
                    }
                }
            }

            page_token = response.next_page_token;
            if page_token.is_none() {
                break;
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

        let hub = self.hub()?;
        let parent = format!("projects/{}", self.project_id);
        let secret_name = self.secret_name(name);

        // Create the secret
        let secret = Secret {
            replication: Some(Replication {
                automatic: Some(Default::default()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let (_, created_secret) = hub
            .projects()
            .secrets_create(secret, &parent)
            .secret_id(&secret_name)
            .doit()
            .await
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to create secret: {}", e)))?;

        // Add the first version with content
        let secret_path = created_secret
            .name
            .unwrap_or_else(|| self.secret_path(name));

        let version_request = AddSecretVersionRequest {
            payload: Some(google_secretmanager1::api::SecretPayload {
                data: Some(content.as_bytes().to_vec()),
                ..Default::default()
            }),
        };

        hub.projects()
            .secrets_add_version(version_request, &secret_path)
            .doit()
            .await
            .map_err(|e| {
                VaultmuxError::Other(anyhow::anyhow!("Failed to add secret version: {}", e))
            })?;

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

        let hub = self.hub()?;
        let secret_path = self.secret_path(name);

        // Add a new version (GCP is versioned, updates create new versions)
        let version_request = AddSecretVersionRequest {
            payload: Some(google_secretmanager1::api::SecretPayload {
                data: Some(content.as_bytes().to_vec()),
                ..Default::default()
            }),
        };

        hub.projects()
            .secrets_add_version(version_request, &secret_path)
            .doit()
            .await
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to update secret: {}", e)))?;

        Ok(())
    }

    async fn delete_item(&mut self, name: &str, _session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        let hub = self.hub()?;
        let secret_path = self.secret_path(name);

        hub.projects()
            .secrets_delete(&secret_path)
            .doit()
            .await
            .map_err(|e| {
                if e.to_string().contains("NOT_FOUND") {
                    VaultmuxError::NotFound(name.to_string())
                } else {
                    VaultmuxError::Other(anyhow::anyhow!("Failed to delete secret: {}", e))
                }
            })?;

        Ok(())
    }

    async fn list_locations(&self, _session: &dyn Session) -> Result<Vec<String>> {
        Err(VaultmuxError::NotSupported(
            "GCP Secret Manager does not support locations (secrets are project-scoped)"
                .to_string(),
        ))
    }

    async fn location_exists(&self, _name: &str, _session: &dyn Session) -> Result<bool> {
        Err(VaultmuxError::NotSupported(
            "GCP Secret Manager does not support locations".to_string(),
        ))
    }

    async fn create_location(&mut self, _name: &str, _session: &dyn Session) -> Result<()> {
        Err(VaultmuxError::NotSupported(
            "GCP Secret Manager does not support locations".to_string(),
        ))
    }

    async fn list_items_in_location(
        &self,
        _loc_type: &str,
        _loc_value: &str,
        _session: &dyn Session,
    ) -> Result<Vec<Item>> {
        Err(VaultmuxError::NotSupported(
            "GCP Secret Manager does not support locations".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_name() {
        let config = Config::new(crate::BackendType::GCPSecretManager)
            .with_option("project_id", "my-project")
            .with_option("prefix", "app-");
        let backend = GCPBackend::new(config);

        assert_eq!(backend.secret_name("api-key"), "app-api-key");
    }

    #[test]
    fn test_secret_name_no_prefix() {
        let config = Config::new(crate::BackendType::GCPSecretManager)
            .with_option("project_id", "my-project")
            .with_option("prefix", "");
        let backend = GCPBackend::new(config);

        assert_eq!(backend.secret_name("api-key"), "api-key");
    }

    #[test]
    fn test_secret_path() {
        let config = Config::new(crate::BackendType::GCPSecretManager)
            .with_option("project_id", "my-project")
            .with_option("prefix", "app-");
        let backend = GCPBackend::new(config);

        assert_eq!(
            backend.secret_path("api-key"),
            "projects/my-project/secrets/app-api-key"
        );
    }

    #[test]
    fn test_version_path() {
        let config = Config::new(crate::BackendType::GCPSecretManager)
            .with_option("project_id", "my-project")
            .with_option("prefix", "app-");
        let backend = GCPBackend::new(config);

        assert_eq!(
            backend.version_path("api-key", "latest"),
            "projects/my-project/secrets/app-api-key/versions/latest"
        );
    }
}
