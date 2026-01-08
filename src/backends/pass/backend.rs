//! pass backend implementation.

use crate::backends::pass::PassSession;
use crate::cli::{check_command_exists, run_command, StatusCache};
use crate::validation::{validate_item_name, validate_location_name};
use crate::{Backend, Config, Item, ItemType, Result, Session, VaultmuxError};
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// pass (Unix password manager) backend.
///
/// Integrates with the `pass` command-line tool for GPG-encrypted password storage.
pub struct PassBackend {
    store_path: PathBuf,
    prefix: String,
    status_cache: Arc<Mutex<StatusCache>>,
}

impl PassBackend {
    /// Creates a new pass backend from configuration.
    pub fn new(config: Config) -> Self {
        let store_path = config
            .store_path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                PathBuf::from(home).join(".password-store")
            });

        Self {
            store_path,
            prefix: config.prefix,
            status_cache: Arc::new(Mutex::new(StatusCache::default())),
        }
    }

    /// Constructs the full path for an item.
    ///
    /// Items are prefixed (e.g., "myapp/api-key" becomes "myapp/api-key.gpg")
    fn item_path(&self, name: &str) -> String {
        if self.prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", self.prefix, name)
        }
    }

    /// Lists all items in the password store (recursively).
    async fn list_all_items(&self) -> Result<Vec<String>> {
        let output = run_command("pass", &["ls"], &[]).await?;

        let mut items = Vec::new();
        for line in output.lines() {
            let line = line.trim();
            
            // Skip the store path header and tree characters
            if line.starts_with("Password Store") || line.is_empty() {
                continue;
            }

            // Remove tree characters (├──, │, └──, etc.)
            let cleaned = line
                .trim_start_matches("├── ")
                .trim_start_matches("└── ")
                .trim_start_matches("│   ")
                .trim();

            // Skip if it's a directory marker (no .gpg extension)
            if !cleaned.contains(".gpg") {
                continue;
            }

            // Remove .gpg extension
            let item_name = cleaned.trim_end_matches(".gpg");

            // Only include items with our prefix
            if !self.prefix.is_empty() {
                if let Some(stripped) = item_name.strip_prefix(&format!("{}/", self.prefix)) {
                    items.push(stripped.to_string());
                }
            } else {
                items.push(item_name.to_string());
            }
        }

        Ok(items)
    }
}

#[async_trait]
impl Backend for PassBackend {
    fn name(&self) -> &str {
        "pass"
    }

    async fn init(&mut self) -> Result<()> {
        // Check if pass command exists
        if !check_command_exists("pass").await? {
            return Err(VaultmuxError::BackendNotInstalled(
                "pass command not found - install pass (Unix password manager)".to_string(),
            ));
        }

        // Check if GPG command exists
        if !check_command_exists("gpg").await? {
            return Err(VaultmuxError::BackendNotInstalled(
                "gpg command not found - install GnuPG".to_string(),
            ));
        }

        // Check if password store is initialized
        if !self.store_path.exists() {
            return Err(VaultmuxError::Other(anyhow::anyhow!(
                "Password store not initialized at {:?}. Run: pass init <gpg-key-id>",
                self.store_path
            )));
        }

        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        // Check cache first
        if let Ok(cache) = self.status_cache.lock() {
            if let Some(authenticated) = cache.get() {
                return authenticated;
            }
        }

        // pass doesn't have explicit authentication - just check if we can list
        let authenticated = run_command("pass", &["ls"], &[]).await.is_ok();

        // Update cache
        if let Ok(mut cache) = self.status_cache.lock() {
            cache.set(authenticated);
        }

        authenticated
    }

    async fn authenticate(&mut self) -> Result<Arc<dyn Session>> {
        // pass doesn't require explicit authentication
        // GPG agent handles passphrase prompts automatically
        Ok(Arc::new(PassSession::new()))
    }

    async fn sync(&mut self, _session: &dyn Session) -> Result<()> {
        // If pass is using git, we could do: pass git pull
        // For now, no-op (fully offline)
        Ok(())
    }

    async fn get_item(&self, name: &str, _session: &dyn Session) -> Result<Item> {
        validate_item_name(name)?;

        let path = self.item_path(name);
        let content = run_command("pass", &["show", &path], &[]).await.map_err(|e| {
            if e.to_string().contains("not in the password store") {
                VaultmuxError::NotFound(name.to_string())
            } else {
                e
            }
        })?;

        Ok(Item {
            id: name.to_string(),
            name: name.to_string(),
            item_type: ItemType::SecureNote,
            notes: Some(content.trim().to_string()),
            fields: None,
            location: if self.prefix.is_empty() {
                None
            } else {
                Some(self.prefix.clone())
            },
            created: None,
            modified: None,
        })
    }

    async fn get_notes(&self, name: &str, _session: &dyn Session) -> Result<String> {
        validate_item_name(name)?;

        let path = self.item_path(name);
        let content = run_command("pass", &["show", &path], &[]).await.map_err(|e| {
            if e.to_string().contains("not in the password store") {
                VaultmuxError::NotFound(name.to_string())
            } else {
                e
            }
        })?;

        Ok(content.trim().to_string())
    }

    async fn item_exists(&self, name: &str, _session: &dyn Session) -> Result<bool> {
        validate_item_name(name)?;

        let path = self.item_path(name);
        match run_command("pass", &["show", &path], &[]).await {
            Ok(_) => Ok(true),
            Err(VaultmuxError::NotFound(_)) => Ok(false),
            Err(e) if e.to_string().contains("not in the password store") => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn list_items(&self, _session: &dyn Session) -> Result<Vec<Item>> {
        let item_names = self.list_all_items().await?;

        let mut items = Vec::new();
        for name in item_names {
            items.push(Item {
                id: name.clone(),
                name: name.clone(),
                item_type: ItemType::SecureNote,
                notes: None,
                fields: None,
                location: if self.prefix.is_empty() {
                    None
                } else {
                    Some(self.prefix.clone())
                },
                created: None,
                modified: None,
            });
        }

        Ok(items)
    }

    async fn create_item(&mut self, name: &str, content: &str, _session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        // Check if item already exists
        if self.item_exists(name, _session).await? {
            return Err(VaultmuxError::AlreadyExists(name.to_string()));
        }

        let path = self.item_path(name);
        
        // pass insert reads from stdin
        crate::cli::run_command_with_stdin("pass", &["insert", "-m", &path], &[], content)
            .await?;

        Ok(())
    }

    async fn update_item(&mut self, name: &str, content: &str, _session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        // Check if item exists
        if !self.item_exists(name, _session).await? {
            return Err(VaultmuxError::NotFound(name.to_string()));
        }

        let path = self.item_path(name);
        
        // pass insert with -f (force) overwrites
        crate::cli::run_command_with_stdin("pass", &["insert", "-m", "-f", &path], &[], content)
            .await?;

        Ok(())
    }

    async fn delete_item(&mut self, name: &str, _session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        let path = self.item_path(name);
        
        // pass rm with -f (force, no confirmation)
        run_command("pass", &["rm", "-f", &path], &[])
            .await
            .map_err(|e| {
                if e.to_string().contains("not in the password store") {
                    VaultmuxError::NotFound(name.to_string())
                } else {
                    e
                }
            })?;

        Ok(())
    }

    async fn list_locations(&self, _session: &dyn Session) -> Result<Vec<String>> {
        // pass uses directory structure
        // For now, return error - full directory enumeration would be complex
        Err(VaultmuxError::NotSupported(
            "pass backend: list_locations not yet implemented".to_string(),
        ))
    }

    async fn location_exists(&self, _name: &str, _session: &dyn Session) -> Result<bool> {
        Err(VaultmuxError::NotSupported(
            "pass backend: location_exists not yet implemented".to_string(),
        ))
    }

    async fn create_location(&mut self, name: &str, _session: &dyn Session) -> Result<()> {
        validate_location_name(name)?;

        // In pass, locations are just directories
        // We can't create empty directories in pass, but we can create
        // a dummy .gitkeep file if using git
        Err(VaultmuxError::NotSupported(
            "pass backend: create_location not yet implemented (create items with paths instead)"
                .to_string(),
        ))
    }

    async fn list_items_in_location(
        &self,
        _loc_type: &str,
        _loc_value: &str,
        _session: &dyn Session,
    ) -> Result<Vec<Item>> {
        Err(VaultmuxError::NotSupported(
            "pass backend: list_items_in_location not yet implemented".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_path() {
        let config = Config::new(crate::BackendType::Pass).with_prefix("myapp");
        let backend = PassBackend::new(config);

        assert_eq!(backend.item_path("api-key"), "myapp/api-key");
    }

    #[test]
    fn test_item_path_no_prefix() {
        let config = Config::new(crate::BackendType::Pass).with_prefix("");
        let backend = PassBackend::new(config);

        assert_eq!(backend.item_path("api-key"), "api-key");
    }
}
