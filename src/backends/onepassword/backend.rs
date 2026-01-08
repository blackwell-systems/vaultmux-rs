//! 1Password backend implementation.

use crate::backends::onepassword::OnePasswordSession;
use crate::cli::{check_command_exists, run_command, StatusCache};
use crate::validation::validate_item_name;
use crate::{Backend, Config, Item, ItemType, Result, Session, VaultmuxError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// 1Password backend.
///
/// Integrates with 1Password via the `op` CLI tool.
pub struct OnePasswordBackend {
    account: Option<String>,
    vault: String,
    prefix: String,
    status_cache: Arc<Mutex<StatusCache>>,
}

impl OnePasswordBackend {
    /// Creates a new 1Password backend from configuration.
    pub fn new(config: Config) -> Self {
        let account = config.options.get("account").cloned();

        let vault = config
            .options
            .get("vault")
            .cloned()
            .unwrap_or_else(|| "Private".to_string());

        let prefix = config
            .options
            .get("prefix")
            .cloned()
            .unwrap_or(config.prefix);

        Self {
            account,
            vault,
            prefix,
            status_cache: Arc::new(Mutex::new(StatusCache::new(Duration::from_secs(5)))),
        }
    }

    /// Constructs the full item name with prefix.
    fn item_name(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}{}", self.prefix, name)
        }
    }

    /// Strips prefix from item name.
    fn strip_prefix<'a>(&self, name: &'a str) -> Option<&'a str> {
        if self.prefix.is_empty() {
            Some(name)
        } else {
            name.strip_prefix(&self.prefix)
        }
    }

    /// Gets account from session or uses configured account.
    fn get_account(&self, _session: &dyn Session) -> String {
        if let Some(ref account) = self.account {
            account.clone()
        } else {
            "".to_string()
        }
    }
}

#[derive(Debug, Deserialize)]
struct OpAccount {
    shorthand: String,
    #[allow(dead_code)]
    user_uuid: String,
    #[allow(dead_code)]
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpItem {
    id: String,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vault: Option<OpVault>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<Vec<OpField>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    urls: Option<Vec<OpUrl>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpVault {
    id: String,
    name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpField {
    id: String,
    #[serde(rename = "type")]
    field_type: String,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpUrl {
    label: String,
    primary: bool,
    href: String,
}

#[async_trait]
impl Backend for OnePasswordBackend {
    fn name(&self) -> &str {
        "onepassword"
    }

    async fn init(&mut self) -> Result<()> {
        if !check_command_exists("op").await? {
            return Err(VaultmuxError::BackendNotInstalled(
                "1Password CLI (op) is not installed. Install from https://1password.com/downloads/command-line/".to_string()
            ));
        }

        // Verify we can list accounts
        run_command("op", &["account", "list", "--format=json"], &[]).await?;

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        let mut cache = self.status_cache.lock().await;

        if let Some(status) = cache.get() {
            return status;
        }

        // Check if we can run a simple command
        let result = run_command("op", &["vault", "list", "--format=json"], &[]).await;
        let authenticated = result.is_ok();

        cache.set(authenticated);
        authenticated
    }

    async fn authenticate(&mut self) -> Result<Arc<dyn Session>> {
        // Determine account to use
        let account = if let Some(ref acc) = self.account {
            acc.clone()
        } else {
            // Get default account
            let output = run_command("op", &["account", "list", "--format=json"], &[]).await?;
            let accounts: Vec<OpAccount> = serde_json::from_str(&output).map_err(|e| {
                VaultmuxError::Other(anyhow::anyhow!("Failed to parse accounts: {}", e))
            })?;

            accounts
                .first()
                .ok_or_else(|| VaultmuxError::NotAuthenticated)?
                .shorthand
                .clone()
        };

        // Sign in to get session token
        let token_output = run_command("op", &["signin", "--account", &account, "--raw"], &[])
            .await
            .map_err(|e| {
                if e.to_string().contains("authentication required") {
                    VaultmuxError::NotAuthenticated
                } else {
                    e
                }
            })?;

        let token = token_output.trim().to_string();
        Ok(Arc::new(OnePasswordSession::new(token, account)))
    }

    async fn sync(&mut self, _session: &dyn Session) -> Result<()> {
        // 1Password is always synchronized
        Ok(())
    }

    async fn get_item(&self, name: &str, session: &dyn Session) -> Result<Item> {
        validate_item_name(name)?;

        let item_name = self.item_name(name);
        let account = self.get_account(session);

        let env = if !account.is_empty() {
            vec![("OP_SESSION", session.token())]
        } else {
            vec![]
        };

        let output = run_command(
            "op",
            &[
                "item",
                "get",
                &item_name,
                "--vault",
                &self.vault,
                "--format=json",
            ],
            &env,
        )
        .await
        .map_err(|e| {
            if e.to_string().contains("isn't an item") {
                VaultmuxError::NotFound(name.to_string())
            } else {
                e
            }
        })?;

        let op_item: OpItem = serde_json::from_str(&output)
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to parse item: {}", e)))?;

        // Extract notes from notesPlain field
        let notes = op_item.fields.as_ref().and_then(|fields| {
            fields
                .iter()
                .find(|f| f.label == "notesPlain" || f.id == "notesPlain")
                .and_then(|f| f.value.clone())
        });

        // Parse timestamps
        let created = op_item.created_at.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        let modified = op_item.updated_at.as_ref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        });

        Ok(Item {
            id: op_item.id,
            name: name.to_string(),
            item_type: ItemType::SecureNote,
            notes,
            fields: None,
            location: op_item.vault.as_ref().map(|v| v.name.clone()),
            created,
            modified,
        })
    }

    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String> {
        let item = self.get_item(name, session).await?;
        item.notes
            .ok_or_else(|| VaultmuxError::NotFound(format!("{} has no notes", name)))
    }

    async fn item_exists(&self, name: &str, session: &dyn Session) -> Result<bool> {
        validate_item_name(name)?;

        let item_name = self.item_name(name);
        let account = self.get_account(session);

        let env = if !account.is_empty() {
            vec![("OP_SESSION", session.token())]
        } else {
            vec![]
        };

        let result = run_command(
            "op",
            &[
                "item",
                "get",
                &item_name,
                "--vault",
                &self.vault,
                "--format=json",
            ],
            &env,
        )
        .await;

        Ok(result.is_ok())
    }

    async fn list_items(&self, session: &dyn Session) -> Result<Vec<Item>> {
        let account = self.get_account(session);

        let env = if !account.is_empty() {
            vec![("OP_SESSION", session.token())]
        } else {
            vec![]
        };

        let output = run_command(
            "op",
            &["item", "list", "--vault", &self.vault, "--format=json"],
            &env,
        )
        .await?;

        let op_items: Vec<OpItem> = serde_json::from_str(&output)
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to parse items: {}", e)))?;

        let mut items = Vec::new();
        for op_item in op_items {
            if let Some(name) = self.strip_prefix(&op_item.title) {
                let created = op_item.created_at.as_ref().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                });

                let modified = op_item.updated_at.as_ref().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                });

                items.push(Item {
                    id: op_item.id,
                    name: name.to_string(),
                    item_type: ItemType::SecureNote,
                    notes: None,
                    fields: None,
                    location: op_item.vault.as_ref().map(|v| v.name.clone()),
                    created,
                    modified,
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

        if self.item_exists(name, session).await? {
            return Err(VaultmuxError::AlreadyExists(name.to_string()));
        }

        let item_name = self.item_name(name);
        let account = self.get_account(session);

        let env = if !account.is_empty() {
            vec![("OP_SESSION", session.token())]
        } else {
            vec![]
        };

        // Create secure note with content
        let template = serde_json::json!({
            "title": item_name,
            "category": "SECURE_NOTE",
            "fields": [{
                "id": "notesPlain",
                "type": "STRING",
                "purpose": "NOTES",
                "label": "notesPlain",
                "value": content
            }]
        });

        let template_str = serde_json::to_string(&template).map_err(|e| {
            VaultmuxError::Other(anyhow::anyhow!("Failed to create template: {}", e))
        })?;

        run_command(
            "op",
            &[
                "item",
                "create",
                "--vault",
                &self.vault,
                "--template",
                &template_str,
            ],
            &env,
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

        if !self.item_exists(name, session).await? {
            return Err(VaultmuxError::NotFound(name.to_string()));
        }

        let item_name = self.item_name(name);
        let account = self.get_account(session);

        let env = if !account.is_empty() {
            vec![("OP_SESSION", session.token())]
        } else {
            vec![]
        };

        run_command(
            "op",
            &[
                "item",
                "edit",
                &item_name,
                &format!("notesPlain={}", content),
                "--vault",
                &self.vault,
            ],
            &env,
        )
        .await?;

        Ok(())
    }

    async fn delete_item(&mut self, name: &str, session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        let item_name = self.item_name(name);
        let account = self.get_account(session);

        let env = if !account.is_empty() {
            vec![("OP_SESSION", session.token())]
        } else {
            vec![]
        };

        run_command(
            "op",
            &["item", "delete", &item_name, "--vault", &self.vault],
            &env,
        )
        .await
        .map_err(|e| {
            if e.to_string().contains("isn't an item") {
                VaultmuxError::NotFound(name.to_string())
            } else {
                e
            }
        })?;

        Ok(())
    }

    async fn list_locations(&self, session: &dyn Session) -> Result<Vec<String>> {
        let account = self.get_account(session);

        let env = if !account.is_empty() {
            vec![("OP_SESSION", session.token())]
        } else {
            vec![]
        };

        let output = run_command("op", &["vault", "list", "--format=json"], &env).await?;

        let vaults: Vec<OpVault> = serde_json::from_str(&output)
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to parse vaults: {}", e)))?;

        Ok(vaults.into_iter().map(|v| v.name).collect())
    }

    async fn location_exists(&self, name: &str, session: &dyn Session) -> Result<bool> {
        let locations = self.list_locations(session).await?;
        Ok(locations.iter().any(|v| v == name))
    }

    async fn create_location(&mut self, _name: &str, _session: &dyn Session) -> Result<()> {
        Err(VaultmuxError::NotSupported(
            "1Password vault creation requires account permissions".to_string(),
        ))
    }

    async fn list_items_in_location(
        &self,
        _loc_type: &str,
        loc_value: &str,
        session: &dyn Session,
    ) -> Result<Vec<Item>> {
        let account = self.get_account(session);

        let env = if !account.is_empty() {
            vec![("OP_SESSION", session.token())]
        } else {
            vec![]
        };

        let output = run_command(
            "op",
            &["item", "list", "--vault", loc_value, "--format=json"],
            &env,
        )
        .await?;

        let op_items: Vec<OpItem> = serde_json::from_str(&output)
            .map_err(|e| VaultmuxError::Other(anyhow::anyhow!("Failed to parse items: {}", e)))?;

        let mut items = Vec::new();
        for op_item in op_items {
            if let Some(name) = self.strip_prefix(&op_item.title) {
                let created = op_item.created_at.as_ref().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                });

                let modified = op_item.updated_at.as_ref().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                });

                items.push(Item {
                    id: op_item.id,
                    name: name.to_string(),
                    item_type: ItemType::SecureNote,
                    notes: None,
                    fields: None,
                    location: Some(loc_value.to_string()),
                    created,
                    modified,
                });
            }
        }

        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_name() {
        let config = Config::new(crate::BackendType::OnePassword).with_option("prefix", "test-");
        let backend = OnePasswordBackend::new(config);

        assert_eq!(backend.item_name("api-key"), "test-api-key");
    }

    #[test]
    fn test_item_name_no_prefix() {
        let config = Config::new(crate::BackendType::OnePassword).with_option("prefix", "");
        let backend = OnePasswordBackend::new(config);

        assert_eq!(backend.item_name("api-key"), "api-key");
    }

    #[test]
    fn test_strip_prefix() {
        let config = Config::new(crate::BackendType::OnePassword).with_option("prefix", "test-");
        let backend = OnePasswordBackend::new(config);

        assert_eq!(backend.strip_prefix("test-api-key"), Some("api-key"));
        assert_eq!(backend.strip_prefix("other-api-key"), None);
    }
}
