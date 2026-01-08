//! Windows Credential Manager backend implementation.

use crate::backends::wincred::WincredSession;
use crate::validation::validate_item_name;
use crate::{Backend, Config, Item, ItemType, Result, Session, VaultmuxError};
use async_trait::async_trait;
use serde::Deserialize;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;

/// Windows Credential Manager backend.
///
/// Uses PowerShell to interact with the Windows Credential Manager.
pub struct WincredBackend {
    prefix: String,
}

impl WincredBackend {
    /// Creates a new Windows Credential Manager backend from configuration.
    pub fn new(config: Config) -> Self {
        let prefix = config.options.get("prefix").cloned().unwrap_or_else(|| {
            if config.prefix.is_empty() {
                "vaultmux".to_string()
            } else {
                config.prefix.clone()
            }
        });

        Self { prefix }
    }

    /// Constructs the full credential target name with prefix.
    fn credential_target(&self, name: &str) -> String {
        format!("{}:{}", self.prefix, name)
    }

    /// Escapes single quotes in PowerShell strings by doubling them.
    fn escape_powershell_string(s: &str) -> String {
        s.replace('\'', "''")
    }

    /// Executes a PowerShell script and returns the output.
    async fn run_powershell(&self, script: &str) -> Result<String> {
        let output = Command::new("powershell.exe")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(script)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                VaultmuxError::Other(anyhow::anyhow!("Failed to execute PowerShell: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if output.status.code() == Some(1) {
                return Err(VaultmuxError::NotFound("Item not found".to_string()));
            }
            return Err(VaultmuxError::Other(anyhow::anyhow!(
                "PowerShell command failed: {}",
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[derive(Deserialize)]
struct CredentialListItem {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Target")]
    target: String,
}

#[async_trait]
impl Backend for WincredBackend {
    fn name(&self) -> &str {
        "wincred"
    }

    async fn init(&mut self) -> Result<()> {
        let script = "$PSVersionTable.PSVersion.Major";
        match self.run_powershell(script).await {
            Ok(_) => Ok(()),
            Err(_) => Err(VaultmuxError::BackendNotInstalled(
                "PowerShell is required for Windows Credential Manager backend".to_string(),
            )),
        }
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        true
    }

    async fn authenticate(&mut self) -> Result<Arc<dyn Session>> {
        Ok(Arc::new(WincredSession::new()))
    }

    async fn sync(&mut self, _session: &dyn Session) -> Result<()> {
        Ok(())
    }

    async fn get_item(&self, name: &str, _session: &dyn Session) -> Result<Item> {
        validate_item_name(name)?;

        let notes = self.get_notes(name, _session).await?;

        Ok(Item {
            id: self.credential_target(name),
            name: name.to_string(),
            item_type: ItemType::SecureNote,
            notes: Some(notes),
            fields: None,
            location: None,
            created: None,
            modified: None,
        })
    }

    async fn get_notes(&self, name: &str, _session: &dyn Session) -> Result<String> {
        validate_item_name(name)?;

        let target = self.credential_target(name);
        let target_escaped = Self::escape_powershell_string(&target);

        let script = format!(
            r#"
$cred = Get-StoredCredential -Target '{}' -ErrorAction SilentlyContinue
if ($cred) {{
    $ptr = [System.Runtime.InteropServices.Marshal]::SecureStringToCoTaskMemUnicode($cred.Password)
    $password = [System.Runtime.InteropServices.Marshal]::PtrToStringUni($ptr)
    [System.Runtime.InteropServices.Marshal]::ZeroFreeCoTaskMemUnicode($ptr)
    Write-Output $password
}} else {{
    exit 1
}}
"#,
            target_escaped
        );

        self.run_powershell(&script)
            .await
            .map_err(|_| VaultmuxError::NotFound(name.to_string()))
    }

    async fn item_exists(&self, name: &str, _session: &dyn Session) -> Result<bool> {
        validate_item_name(name)?;

        let target = self.credential_target(name);
        let target_escaped = Self::escape_powershell_string(&target);

        let script = format!(
            r#"
$cred = Get-StoredCredential -Target '{}' -ErrorAction SilentlyContinue
if ($cred) {{ exit 0 }} else {{ exit 1 }}
"#,
            target_escaped
        );

        match self.run_powershell(&script).await {
            Ok(_) => Ok(true),
            Err(VaultmuxError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn list_items(&self, _session: &dyn Session) -> Result<Vec<Item>> {
        let prefix_len = self.prefix.len() + 1;

        let script = format!(
            r#"
$creds = Get-StoredCredential | Where-Object {{ $_.TargetName -like '{}:*' }}
$creds | ForEach-Object {{
    [PSCustomObject]@{{
        Name = $_.TargetName.Substring({})
        Target = $_.TargetName
    }}
}} | ConvertTo-Json -Compress
"#,
            self.prefix, prefix_len
        );

        let output = self.run_powershell(&script).await?;

        if output.is_empty() {
            return Ok(Vec::new());
        }

        let items: Vec<Item> = if output.trim_start().starts_with('[') {
            let list: Vec<CredentialListItem> = serde_json::from_str(&output).map_err(|e| {
                VaultmuxError::Other(anyhow::anyhow!("Failed to parse credential list: {}", e))
            })?;
            list.into_iter()
                .map(|cred| Item {
                    id: cred.target,
                    name: cred.name,
                    item_type: ItemType::SecureNote,
                    notes: None,
                    fields: None,
                    location: None,
                    created: None,
                    modified: None,
                })
                .collect()
        } else {
            let single: CredentialListItem = serde_json::from_str(&output).map_err(|e| {
                VaultmuxError::Other(anyhow::anyhow!("Failed to parse credential: {}", e))
            })?;
            vec![Item {
                id: single.target,
                name: single.name,
                item_type: ItemType::SecureNote,
                notes: None,
                fields: None,
                location: None,
                created: None,
                modified: None,
            }]
        };

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

        let target = self.credential_target(name);
        let target_escaped = Self::escape_powershell_string(&target);
        let content_escaped = Self::escape_powershell_string(content);

        let script = format!(
            r#"
$password = ConvertTo-SecureString -String '{}' -AsPlainText -Force
$cred = New-Object System.Management.Automation.PSCredential('vaultmux', $password)
New-StoredCredential -Target '{}' -Credential $cred -Type Generic -Persist LocalMachine
"#,
            content_escaped, target_escaped
        );

        self.run_powershell(&script).await?;
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

        let target = self.credential_target(name);
        let target_escaped = Self::escape_powershell_string(&target);
        let content_escaped = Self::escape_powershell_string(content);

        let script = format!(
            r#"
Remove-StoredCredential -Target '{}' -ErrorAction SilentlyContinue
$password = ConvertTo-SecureString -String '{}' -AsPlainText -Force
$cred = New-Object System.Management.Automation.PSCredential('vaultmux', $password)
New-StoredCredential -Target '{}' -Credential $cred -Type Generic -Persist LocalMachine
"#,
            target_escaped, content_escaped, target_escaped
        );

        self.run_powershell(&script).await?;
        Ok(())
    }

    async fn delete_item(&mut self, name: &str, _session: &dyn Session) -> Result<()> {
        validate_item_name(name)?;

        let target = self.credential_target(name);
        let target_escaped = Self::escape_powershell_string(&target);

        let script = format!(
            r#"
Remove-StoredCredential -Target '{}' -ErrorAction SilentlyContinue
"#,
            target_escaped
        );

        self.run_powershell(&script).await?;
        Ok(())
    }

    async fn list_locations(&self, _session: &dyn Session) -> Result<Vec<String>> {
        Err(VaultmuxError::NotSupported(
            "Windows Credential Manager does not support locations".to_string(),
        ))
    }

    async fn location_exists(&self, _name: &str, _session: &dyn Session) -> Result<bool> {
        Err(VaultmuxError::NotSupported(
            "Windows Credential Manager does not support locations".to_string(),
        ))
    }

    async fn create_location(&mut self, _name: &str, _session: &dyn Session) -> Result<()> {
        Err(VaultmuxError::NotSupported(
            "Windows Credential Manager does not support locations".to_string(),
        ))
    }

    async fn list_items_in_location(
        &self,
        _loc_type: &str,
        _loc_value: &str,
        _session: &dyn Session,
    ) -> Result<Vec<Item>> {
        Err(VaultmuxError::NotSupported(
            "Windows Credential Manager does not support locations".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credential_target() {
        let config = Config::new(crate::BackendType::WindowsCredentialManager).with_prefix("myapp");
        let backend = WincredBackend::new(config);

        assert_eq!(backend.credential_target("api-key"), "myapp:api-key");
    }

    #[test]
    fn test_escape_powershell_string() {
        assert_eq!(
            WincredBackend::escape_powershell_string("it's a test"),
            "it''s a test"
        );
        assert_eq!(
            WincredBackend::escape_powershell_string("no quotes"),
            "no quotes"
        );
    }
}
