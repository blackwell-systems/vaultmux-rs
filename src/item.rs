//! Item data structures for vault entries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An item stored in a vault.
///
/// Items represent secrets, credentials, or other sensitive data managed
/// by the backend. All fields except `id`, `name`, and `item_type` are optional.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Item {
    /// Unique identifier (backend-specific format)
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Item type
    #[serde(rename = "type")]
    pub item_type: ItemType,

    /// Notes/content field (main storage for secret data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Additional structured fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<HashMap<String, String>>,

    /// Location (folder/vault/directory name)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,

    /// When the item was created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,

    /// When the item was last modified
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
}

impl Item {
    /// Creates a new secure note item.
    ///
    /// This is the most common item type for storing arbitrary secret data.
    ///
    /// # Example
    ///
    /// ```
    /// use vaultmux::Item;
    ///
    /// let item = Item::new_secure_note("api-key", "sk_live_abc123");
    /// assert_eq!(item.name, "api-key");
    /// assert_eq!(item.notes.as_deref(), Some("sk_live_abc123"));
    /// ```
    pub fn new_secure_note(name: impl Into<String>, notes: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            item_type: ItemType::SecureNote,
            notes: Some(notes.into()),
            fields: None,
            location: None,
            created: Some(Utc::now()),
            modified: Some(Utc::now()),
        }
    }

    /// Creates a new login item.
    pub fn new_login(name: impl Into<String>, username: String, password: String) -> Self {
        let mut fields = HashMap::new();
        fields.insert("username".to_string(), username);
        fields.insert("password".to_string(), password);

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            item_type: ItemType::Login,
            notes: None,
            fields: Some(fields),
            location: None,
            created: Some(Utc::now()),
            modified: Some(Utc::now()),
        }
    }
}

/// Type of vault item.
///
/// Different backends support different item types. The most universally
/// supported type is [`SecureNote`](ItemType::SecureNote).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ItemType {
    /// Secure note (arbitrary text/data)
    SecureNote,
    /// Login credentials (username/password)
    Login,
    /// SSH key
    SSHKey,
    /// Identity information
    Identity,
    /// Credit card
    Card,
}

impl std::fmt::Display for ItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SecureNote => write!(f, "SecureNote"),
            Self::Login => write!(f, "Login"),
            Self::SSHKey => write!(f, "SSHKey"),
            Self::Identity => write!(f, "Identity"),
            Self::Card => write!(f, "Card"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_secure_note() {
        let item = Item::new_secure_note("test-key", "test-value");
        assert_eq!(item.name, "test-key");
        assert_eq!(item.notes.as_deref(), Some("test-value"));
        assert_eq!(item.item_type, ItemType::SecureNote);
        assert!(item.created.is_some());
    }

    #[test]
    fn test_new_login() {
        let item = Item::new_login("github", "user@example.com".to_string(), "password123".to_string());
        assert_eq!(item.name, "github");
        assert_eq!(item.item_type, ItemType::Login);
        
        let fields = item.fields.unwrap();
        assert_eq!(fields.get("username"), Some(&"user@example.com".to_string()));
        assert_eq!(fields.get("password"), Some(&"password123".to_string()));
    }

    #[test]
    fn test_item_type_display() {
        assert_eq!(ItemType::SecureNote.to_string(), "SecureNote");
        assert_eq!(ItemType::Login.to_string(), "Login");
    }

    #[test]
    fn test_item_serialization() {
        let item = Item::new_secure_note("test", "value");
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: Item = serde_json::from_str(&json).unwrap();
        assert_eq!(item, deserialized);
    }
}
