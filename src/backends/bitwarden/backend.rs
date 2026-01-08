//! Bitwarden backend implementation.

use crate::backends::bitwarden::BitwardenSession;
use crate::cli::{check_command_exists, run_command, StatusCache};
use crate::session::SessionCache;
use crate::validation::{validate_item_name, validate_location_name};
use crate::{Backend, Config, Item, ItemType, Result, Session, VaultmuxError};
use async_trait::async_trait;
use base64::Engine;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Bitwarden CLI backend.
///
/// Integrates with the `bw` command-line tool for Bitwarden vault management.
pub struct BitwardenBackend {
    prefix: String,
    session_cache: Option<SessionCache>,
    status_cache: Arc<Mutex<StatusCache>>,
}

impl BitwardenBackend {
    /// Creates a new Bitwarden backend from configuration.
    pub fn new(config: Config) -> Self {
        let session_cache = config.session_file.as_ref().and_then(|path| {
            // Note: This is synchronous - in real impl should be async
            futures::executor::block_on(SessionCache::new(path, config.session_ttl)).ok()
        });

        Self {
            prefix: config.prefix,
            session_cache,
            status_cache: Arc::new(Mutex::new(StatusCache::default())),
        }
    }

    /// Constructs item name with prefix.
    fn prefixed_name(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", self.prefix, name)
        }
    }

    /// Checks vault lock status.
    async fn check_lock_status(&self) -> Result<bool> {
        let output = run_command("bw", &["status"], &[]).await?;
        let status: BitwardenStatus = serde_json::from_str(&output).map_err(|e| {
            VaultmuxError::Other(anyhow::anyhow!("Failed to parse bw status: {}", e))
        })?;

        Ok(status.status == "unlocked")
    }
}

/// Bitwarden status response.
#[derive(Debug, Deserialize)]
struct BitwardenStatus {
    status: String,
}

/// Bitwarden item from JSON.
#[derive(Debug, Serialize, Deserialize)]
struct BitwardenItem {
    id: String,
    name: String,
    #[serde(rename = "type")]
    item_type: u8,
    notes: Option<String>,
    #[serde(rename = "folderId")]
    folder_id: Option<String>,
    #[serde(rename = "revisionDate")]
    revision_date: Option<String>,
}

/// Bitwarden folder.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BitwardenFolder {
    id: String,
    name: String,
}

#[async_trait]
impl Backend for BitwardenBackend {
    fn name(&self) -> &str {
        "bitwarden"
    }

    async fn init(&mut self) -> Result<()> {
        // Check if bw command exists
        if !check_command_exists("bw").await? {
            return Err(VaultmuxError::BackendNotInstalled(
                "bw command not found - install Bitwarden CLI from https://bitwarden.com/download/"
                    .to_string(),
            ));
        }

        // Check if logged in
        let output = run_command("bw", &["status"], &[]).await?;
        let status: BitwardenStatus = serde_json::from_str(&output).map_err(|e| {
            VaultmuxError::Other(anyhow::anyhow!("Failed to parse bw status: {}", e))
        })?;

        if status.status == "unauthenticated" {
            return Err(VaultmuxError::NotAuthenticated);
        }

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        // Lock the vault
        let _ = run_command("bw", &["lock"], &[]).await;
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        // Check cache first
        if let Ok(cache) = self.status_cache.lock() {
            if let Some(authenticated) = cache.get() {
                return authenticated;
            }
        }

        // Check actual status
        let authenticated = self.check_lock_status().await.unwrap_or(false);

        // Update cache
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.set(authenticated);
        }

        authenticated
    }

    async fn authenticate(&mut self) -> Result<Arc<dyn Session>> {
        // Try to load from cache first
        if let Some(ref cache) = self.session_cache {
            if let Ok(Some(cached)) = cache.load().await {
                if Utc::now() < cached.expires {
                    let session =
                        BitwardenSession::from_token_and_expiry(cached.token, cached.expires);
                    return Ok(Arc::new(session));
                }
            }
        }

        // Check if already unlocked
        if self.check_lock_status().await? {
            // Already unlocked - need to get the session token
            // Unfortunately, bw doesn't provide a way to get current session
            // User must unlock again
            return Err(VaultmuxError::Other(anyhow::anyhow!(
                "Vault is unlocked but no session token available. Please run 'bw lock' and unlock again."
            )));
        }

        // Vault is locked - unlock it
        // Note: This will prompt for password interactively
        let output = run_command("bw", &["unlock", "--raw"], &[])
            .await
            .map_err(|e| {
                if e.to_string().contains("Invalid master password") {
                    VaultmuxError::NotAuthenticated
                } else {
                    e
                }
            })?;

        let token = output.trim().to_string();

        // Create session
        let ttl = std::time::Duration::from_secs(1800); // 30 minutes
        let session = BitwardenSession::new(token.clone(), ttl);

        // Save to cache
        if let Some(ref cache) = self.session_cache {
            cache.save(&token, "bitwarden").await?;
        }

        // Invalidate status cache since we just authenticated
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.invalidate();
        }

        Ok(Arc::new(session))
    }

    async fn sync(&mut self, session: &dyn Session) -> Result<()> {
        run_command("bw", &["sync"], &[("BW_SESSION", session.token())]).await?;
        Ok(())
    }

    async fn get_item(&self, name: &str, session: &dyn Session) -> Result<Item> {
        validate_item_name(name)?;

        let full_name = self.prefixed_name(name);

        // Get item by name
        let output = run_command(
            "bw",
            &["get", "item", &full_name],
            &[("BW_SESSION", session.token())],
        )
        .await
        .map_err(|e| {
            if e.to_string().contains("Not found") {
                VaultmuxError::NotFound(name.to_string())
            } else {
                e
            }
        })?;

        let bw_item: BitwardenItem = serde_json::from_str(&output)
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to parse item: {}", e)))?;

        // Convert to our Item type
        let item_type = match bw_item.item_type {
            1 => ItemType::Login,
            2 => ItemType::SecureNote,
            3 => ItemType::Card,
            4 => ItemType::Identity,
            _ => ItemType::SecureNote,
        };

        Ok(Item {
            id: bw_item.id,
            name: name.to_string(),
            item_type,
            notes: bw_item.notes,
            fields: None,
            location: bw_item.folder_id,
            created: None,
            modified: bw_item.revision_date.and_then(|d| d.parse().ok()),
        })
    }

    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String> {
        let item = self.get_item(name, session).await?;
        item.notes
            .ok_or_else(|| VaultmuxError::NotFound(format!("{} has no notes", name)))
    }

    async fn item_exists(&self, name: &str, session: &dyn Session) -> Result<bool> {
        validate_item_name(name)?;

        match self.get_item(name, session).await {
            Ok(_) => Ok(true),
            Err(VaultmuxError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn list_items(&self, session: &dyn Session) -> Result<Vec<Item>> {
        // List all items
        let output =
            run_command("bw", &["list", "items"], &[("BW_SESSION", session.token())]).await?;

        let bw_items: Vec<BitwardenItem> = serde_json::from_str(&output)
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to parse items: {}", e)))?;

        // Filter by prefix and convert
        let mut items = Vec::new();
        for bw_item in bw_items {
            // Check if name matches our prefix
            let matches_prefix = if self.prefix.is_empty() {
                true
            } else {
                bw_item.name.starts_with(&format!("{}/", self.prefix))
            };

            if matches_prefix {
                let item_type = match bw_item.item_type {
                    1 => ItemType::Login,
                    2 => ItemType::SecureNote,
                    3 => ItemType::Card,
                    4 => ItemType::Identity,
                    _ => ItemType::SecureNote,
                };

                // Strip prefix from name
                let name = if self.prefix.is_empty() {
                    bw_item.name.clone()
                } else {
                    bw_item
                        .name
                        .strip_prefix(&format!("{}/", self.prefix))
                        .unwrap_or(&bw_item.name)
                        .to_string()
                };

                items.push(Item {
                    id: bw_item.id,
                    name,
                    item_type,
                    notes: None, // Don't fetch notes for all items
                    fields: None,
                    location: bw_item.folder_id,
                    created: None,
                    modified: None,
                });
            }
        }

        Ok(items)
    }

    async fn create_item(
        &mut self,
        name: &str,
        content: &str,
        session: &dyn Session,
    ) -> Result<()> {
        validate_item_name(name)?;

        // Check if exists
        if self.item_exists(name, session).await? {
            return Err(VaultmuxError::AlreadyExists(name.to_string()));
        }

        let full_name = self.prefixed_name(name);

        // Create item template
        let template = serde_json::json!({
            "type": 2, // Secure note
            "name": full_name,
            "notes": content,
            "secureNote": {
                "type": 0
            }
        });

        // Encode as base64 for bw create
        let template_str = template.to_string();
        let encoded = base64::engine::general_purpose::STANDARD.encode(template_str.as_bytes());

        run_command(
            "bw",
            &["create", "item", &encoded],
            &[("BW_SESSION", session.token())],
        )
        .await?;

        Ok(())
    }

    async fn update_item(
        &mut self,
        name: &str,
        content: &str,
        session: &dyn Session,
    ) -> Result<()> {
        validate_item_name(name)?;

        // Get existing item
        let item = self.get_item(name, session).await?;

        // Update template
        let template = serde_json::json!({
            "id": item.id,
            "type": 2,
            "name": self.prefixed_name(name),
            "notes": content,
            "secureNote": {
                "type": 0
            }
        });

        let template_str = template.to_string();
        let encoded = base64::engine::general_purpose::STANDARD.encode(template_str.as_bytes());

        run_command(
            "bw",
            &["edit", "item", &item.id, &encoded],
            &[("BW_SESSION", session.token())],
        )
        .await?;

        Ok(())
    }

    async fn delete_item(&mut self, name: &str, session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        // Get item ID
        let item = self.get_item(name, session).await?;

        run_command(
            "bw",
            &["delete", "item", &item.id],
            &[("BW_SESSION", session.token())],
        )
        .await?;

        Ok(())
    }

    async fn list_locations(&self, session: &dyn Session) -> Result<Vec<String>> {
        let output = run_command(
            "bw",
            &["list", "folders"],
            &[("BW_SESSION", session.token())],
        )
        .await?;

        let folders: Vec<BitwardenFolder> = serde_json::from_str(&output)
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to parse folders: {}", e)))?;

        Ok(folders.into_iter().map(|f| f.name).collect())
    }

    async fn location_exists(&self, name: &str, session: &dyn Session) -> Result<bool> {
        validate_location_name(name)?;

        let locations = self.list_locations(session).await?;
        Ok(locations.contains(&name.to_string()))
    }

    async fn create_location(&mut self, name: &str, session: &dyn Session) -> Result<()> {
        validate_location_name(name)?;

        // Check if exists
        if self.location_exists(name, session).await? {
            return Err(VaultmuxError::AlreadyExists(name.to_string()));
        }

        let template = serde_json::json!({
            "name": name
        });

        let template_str = template.to_string();
        let encoded = base64::engine::general_purpose::STANDARD.encode(template_str.as_bytes());

        run_command(
            "bw",
            &["create", "folder", &encoded],
            &[("BW_SESSION", session.token())],
        )
        .await?;

        Ok(())
    }

    async fn list_items_in_location(
        &self,
        loc_type: &str,
        loc_value: &str,
        session: &dyn Session,
    ) -> Result<Vec<Item>> {
        if loc_type != "folder" {
            return Err(VaultmuxError::NotSupported(
                "Bitwarden only supports 'folder' location type".to_string(),
            ));
        }

        // Get all items and filter by folder
        let all_items = self.list_items(session).await?;

        // This is simplified - in reality we'd need to resolve folder name to ID
        Ok(all_items
            .into_iter()
            .filter(|item| item.location.as_deref() == Some(loc_value))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefixed_name() {
        let config = Config::new(crate::BackendType::Bitwarden).with_prefix("myapp");
        let backend = BitwardenBackend::new(config);

        assert_eq!(backend.prefixed_name("api-key"), "myapp/api-key");
    }

    #[test]
    fn test_prefixed_name_empty() {
        let config = Config::new(crate::BackendType::Bitwarden).with_prefix("");
        let backend = BitwardenBackend::new(config);

        assert_eq!(backend.prefixed_name("api-key"), "api-key");
    }
}
