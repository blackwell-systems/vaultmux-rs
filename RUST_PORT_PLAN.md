# Vaultmux Rust Port: 1-to-1 Implementation Plan

**Goal**: Create a production-ready Rust port of the Go vaultmux library with 100% feature parity and improved type safety.

**Source Analysis**: 3,500+ lines of Go code across core framework (838 LOC) and 8 backends (2,667 LOC)

---

## Part 1: Architecture Overview

### 1.1 Go → Rust Mapping Strategy

| Go Concept | Rust Equivalent | Rationale |
|------------|-----------------|-----------|
| `interface Backend` | `trait Backend` | Direct mapping with associated types |
| `interface Session` | `trait Session` | Direct mapping with lifetime annotations |
| `*Backend` (heap pointer) | `Box<dyn Backend>` | Trait objects for dynamic dispatch |
| `sync.RWMutex` | `tokio::sync::RwLock` | Async-aware concurrency |
| `exec.CommandContext` | `tokio::process::Command` | Async subprocess execution |
| `context.Context` | `&impl Context` or manual cancellation | Consider `tokio::select!` for cancellation |
| `encoding/json` | `serde_json` | Better type safety with derive macros |
| `time.Time` | `chrono::DateTime<Utc>` | Standard Rust datetime crate |
| `errors.New()` | `thiserror::Error` | Ergonomic error definitions |

### 1.2 Key Design Improvements

1. **Async by Default**: All I/O operations use `async/await` with tokio
2. **Type Safety**: Leverage Rust enums instead of string constants
3. **Memory Safety**: Eliminate data races with compile-time guarantees
4. **Zero-Cost Abstractions**: No runtime overhead for trait dispatch where possible
5. **Explicit Error Handling**: `Result<T, E>` instead of Go's `(T, error)`
6. **Feature Flags**: Optional backend compilation to minimize binary size

---

## Part 2: Crate Structure

```
vaultmux/
├── Cargo.toml
├── README.md
├── LICENSE
├── CHANGELOG.md
├── examples/
│   ├── basic.rs
│   ├── aws.rs
│   ├── multi_backend.rs
│   └── session_management.rs
├── tests/
│   ├── integration_tests.rs
│   └── backend_tests/
│       ├── mock.rs
│       ├── bitwarden.rs (cfg: integration-bitwarden)
│       └── aws.rs (cfg: integration-aws)
└── src/
    ├── lib.rs                    # Public API surface
    ├── backend.rs                # Backend trait (15 methods)
    ├── session.rs                # Session trait + SessionCache
    ├── item.rs                   # Item struct + ItemType enum
    ├── error.rs                  # VaultmuxError with thiserror
    ├── config.rs                 # Config struct + BackendType enum
    ├── factory.rs                # Backend registration + factory
    ├── validation.rs             # Input validation (prevent injection)
    └── backends/
        ├── mod.rs                # Re-exports based on features
        ├── mock.rs               # In-memory mock (always compiled)
        ├── bitwarden/
        │   ├── mod.rs
        │   ├── backend.rs
        │   └── session.rs
        ├── onepassword/
        │   ├── mod.rs
        │   ├── backend.rs
        │   └── session.rs
        ├── pass/
        │   ├── mod.rs
        │   └── backend.rs
        ├── wincred/
        │   ├── mod.rs
        │   ├── backend_windows.rs
        │   └── backend_unix.rs
        ├── aws/
        │   ├── mod.rs
        │   └── backend.rs
        ├── gcp/
        │   ├── mod.rs
        │   └── backend.rs
        └── azure/
            ├── mod.rs
            └── backend.rs
```

---

## Part 3: Core Implementation Plan

### Phase 1: Foundation (Week 1)

#### 1.1 Define Core Traits

**File: `src/backend.rs`**

```rust
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait Backend: Send + Sync {
    // Metadata
    fn name(&self) -> &str;

    // Lifecycle
    async fn init(&mut self) -> Result<()>;
    async fn close(&mut self) -> Result<()>;

    // Authentication
    async fn is_authenticated(&self) -> bool;
    async fn authenticate(&mut self) -> Result<Arc<dyn Session>>;
    async fn sync(&mut self, session: &dyn Session) -> Result<()>;

    // Item operations (CRUD)
    async fn get_item(&self, name: &str, session: &dyn Session) -> Result<Item>;
    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String>;
    async fn item_exists(&self, name: &str, session: &dyn Session) -> Result<bool>;
    async fn list_items(&self, session: &dyn Session) -> Result<Vec<Item>>;

    // Mutations
    async fn create_item(&mut self, name: &str, content: &str, session: &dyn Session) -> Result<()>;
    async fn update_item(&mut self, name: &str, content: &str, session: &dyn Session) -> Result<()>;
    async fn delete_item(&mut self, name: &str, session: &dyn Session) -> Result<()>;

    // Location management (optional - return ErrNotSupported)
    async fn list_locations(&self, session: &dyn Session) -> Result<Vec<String>>;
    async fn location_exists(&self, name: &str, session: &dyn Session) -> Result<bool>;
    async fn create_location(&mut self, name: &str, session: &dyn Session) -> Result<()>;
    async fn list_items_in_location(
        &self,
        loc_type: &str,
        loc_value: &str,
        session: &dyn Session
    ) -> Result<Vec<Item>>;
}
```

**File: `src/session.rs`**

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[async_trait]
pub trait Session: Send + Sync {
    /// Returns the session token (empty string for stateless backends like pass)
    fn token(&self) -> &str;

    /// Checks if the session is still valid
    async fn is_valid(&self) -> bool;

    /// Attempts to refresh an expired session
    async fn refresh(&mut self) -> Result<()>;

    /// Returns when the session expires (None for non-expiring)
    fn expires_at(&self) -> Option<DateTime<Utc>>;
}
```

**Key Decisions**:
- Use `async_trait` crate for async trait methods (Rust limitation)
- `Arc<dyn Session>` for shared ownership across threads
- Explicit lifetimes where needed for zero-copy operations

#### 1.2 Data Types

**File: `src/item.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ItemType {
    SecureNote,
    Login,
    SSHKey,
    Identity,
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
```

**Improvements over Go**:
- `Option<T>` instead of zero values for optional fields
- `Copy` trait for `ItemType` (efficient passing)
- Serde integration for JSON serialization

#### 1.3 Error System

**File: `src/error.rs`**

```rust
use thiserror::Error;

pub type Result<T> = std::result::Result<T, VaultmuxError>;

#[derive(Debug, Error)]
pub enum VaultmuxError {
    #[error("item not found: {0}")]
    NotFound(String),

    #[error("item already exists: {0}")]
    AlreadyExists(String),

    #[error("not authenticated")]
    NotAuthenticated,

    #[error("session expired")]
    SessionExpired,

    #[error("backend CLI not installed: {0}")]
    BackendNotInstalled(String),

    #[error("vault is locked")]
    BackendLocked,

    #[error("permission denied: {0}")]
    PermissionDenied(String),

    #[error("operation not supported by backend: {0}")]
    NotSupported(String),

    #[error("invalid item name: {0}")]
    InvalidItemName(String),

    #[error("{backend}: {operation} {item}: {source}")]
    BackendOperation {
        backend: String,
        operation: String,
        item: String,
        source: Box<VaultmuxError>,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("command execution failed: {0}")]
    CommandFailed(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl VaultmuxError {
    pub fn backend_op(backend: impl Into<String>, op: impl Into<String>, item: impl Into<String>, err: VaultmuxError) -> Self {
        Self::BackendOperation {
            backend: backend.into(),
            operation: op.into(),
            item: item.into(),
            source: Box::new(err),
        }
    }
}
```

**Improvements**:
- Type-safe error variants (no runtime string matching)
- Automatic `Display` implementation via `thiserror`
- Error chaining with `source` field
- Automatic conversion from `std::io::Error` and `serde_json::Error`

#### 1.4 Configuration

**File: `src/config.rs`**

```rust
use std::collections::HashMap;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BackendType {
    Bitwarden,
    OnePassword,
    Pass,
    WindowsCredentialManager,
    AWSSecretsManager,
    GCPSecretManager,
    AzureKeyVault,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bitwarden => write!(f, "bitwarden"),
            Self::OnePassword => write!(f, "1password"),
            Self::Pass => write!(f, "pass"),
            Self::WindowsCredentialManager => write!(f, "wincred"),
            Self::AWSSecretsManager => write!(f, "awssecrets"),
            Self::GCPSecretManager => write!(f, "gcpsecrets"),
            Self::AzureKeyVault => write!(f, "azurekeyvault"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    /// Backend type
    pub backend: BackendType,

    /// Pass-specific: password store path (default: ~/.password-store)
    pub store_path: Option<String>,

    /// Prefix for item names (default: "dotfiles")
    pub prefix: String,

    /// Session cache file location
    pub session_file: Option<String>,

    /// Session TTL (default: 30 minutes)
    pub session_ttl: Duration,

    /// Backend-specific options
    pub options: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backend: BackendType::Pass,
            store_path: None,
            prefix: "dotfiles".to_string(),
            session_file: None,
            session_ttl: Duration::from_secs(1800), // 30 minutes
            options: HashMap::new(),
        }
    }
}

impl Config {
    pub fn new(backend: BackendType) -> Self {
        Self {
            backend,
            ..Default::default()
        }
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    pub fn with_session_file(mut self, path: impl Into<String>) -> Self {
        self.session_file = Some(path.into());
        self
    }

    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }
}
```

**Improvements**:
- Builder pattern for ergonomic configuration
- Type-safe `BackendType` enum
- `Duration` instead of integer seconds

#### 1.5 Factory Pattern

**File: `src/factory.rs`**

```rust
use crate::{Backend, Config, Result, VaultmuxError};
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub type BackendFactory = fn(Config) -> Result<Box<dyn Backend>>;

lazy_static! {
    static ref BACKEND_REGISTRY: RwLock<HashMap<String, BackendFactory>> = RwLock::new(HashMap::new());
}

pub fn register_backend(backend_type: &str, factory: BackendFactory) {
    let mut registry = BACKEND_REGISTRY.write().unwrap();
    registry.insert(backend_type.to_string(), factory);
}

pub fn new_backend(config: Config) -> Result<Box<dyn Backend>> {
    let backend_name = config.backend.to_string();
    
    let registry = BACKEND_REGISTRY.read().unwrap();
    let factory = registry
        .get(&backend_name)
        .ok_or_else(|| VaultmuxError::Other(
            anyhow::anyhow!("unknown backend: {} (did you enable the feature flag?)", backend_name)
        ))?;

    factory(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_registration() {
        fn mock_factory(_cfg: Config) -> Result<Box<dyn Backend>> {
            unimplemented!()
        }

        register_backend("test", mock_factory);
        
        let registry = BACKEND_REGISTRY.read().unwrap();
        assert!(registry.contains_key("test"));
    }
}
```

**Key Features**:
- Thread-safe global registry with `RwLock`
- `lazy_static` for one-time initialization
- Clear error message about missing feature flags

#### 1.6 Input Validation

**File: `src/validation.rs`**

```rust
use crate::{Result, VaultmuxError};

const DANGEROUS_CHARS: &str = ";|&$`<>(){}[]!*?~#@%^\\\"'";
const MAX_ITEM_NAME_LENGTH: usize = 255;

pub fn validate_item_name(name: &str) -> Result<()> {
    // Check for empty name
    if name.is_empty() {
        return Err(VaultmuxError::InvalidItemName("name cannot be empty".to_string()));
    }

    // Check length
    if name.len() > MAX_ITEM_NAME_LENGTH {
        return Err(VaultmuxError::InvalidItemName(
            format!("name exceeds maximum length of {} characters", MAX_ITEM_NAME_LENGTH)
        ));
    }

    // Check for null bytes
    if name.contains('\0') {
        return Err(VaultmuxError::InvalidItemName("name contains null byte".to_string()));
    }

    // Check for control characters
    if name.chars().any(|c| c.is_control() && c != '\n' && c != '\t') {
        return Err(VaultmuxError::InvalidItemName("name contains control characters".to_string()));
    }

    // Check for dangerous shell characters (prevents command injection)
    if name.chars().any(|c| DANGEROUS_CHARS.contains(c)) {
        return Err(VaultmuxError::InvalidItemName(
            format!("name contains dangerous characters: {}", DANGEROUS_CHARS)
        ));
    }

    Ok(())
}

pub fn validate_location_name(name: &str) -> Result<()> {
    // Locations have same constraints as item names
    validate_item_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_names() {
        assert!(validate_item_name("my-secret").is_ok());
        assert!(validate_item_name("API_KEY_123").is_ok());
        assert!(validate_item_name("prod.database.password").is_ok());
    }

    #[test]
    fn test_invalid_names() {
        assert!(validate_item_name("").is_err());
        assert!(validate_item_name("name; rm -rf /").is_err());
        assert!(validate_item_name("name|grep").is_err());
        assert!(validate_item_name("name$(whoami)").is_err());
        assert!(validate_item_name(&"a".repeat(256)).is_err());
    }
}
```

**Critical for Security**:
- Prevents command injection in CLI backends
- Comprehensive validation with clear error messages

---

### Phase 2: Session Management (Week 1)

**File: `src/session.rs` (extended)**

```rust
use crate::{Result, VaultmuxError};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSession {
    pub token: String,
    pub created: DateTime<Utc>,
    pub expires: DateTime<Utc>,
    pub backend: String,
}

pub struct SessionCache {
    path: PathBuf,
    ttl: std::time::Duration,
}

impl SessionCache {
    pub async fn new(path: impl AsRef<Path>, ttl: std::time::Duration) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        
        // Create parent directory with restricted permissions
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
            
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(parent).await?.permissions();
                perms.set_mode(0o700);
                fs::set_permissions(parent, perms).await?;
            }
        }

        Ok(Self { path, ttl })
    }

    pub async fn load(&self) -> Result<Option<CachedSession>> {
        let data = match fs::read(&self.path).await {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let session: CachedSession = match serde_json::from_slice(&data) {
            Ok(s) => s,
            Err(_) => {
                // Invalid cache - remove it
                let _ = fs::remove_file(&self.path).await;
                return Ok(None);
            }
        };

        // Check if expired
        if Utc::now() > session.expires {
            let _ = fs::remove_file(&self.path).await;
            return Ok(None);
        }

        Ok(Some(session))
    }

    pub async fn save(&self, token: impl Into<String>, backend: impl Into<String>) -> Result<()> {
        let now = Utc::now();
        let ttl_duration = Duration::from_std(self.ttl)
            .map_err(|e| VaultmuxError::Other(e.into()))?;

        let session = CachedSession {
            token: token.into(),
            created: now,
            expires: now + ttl_duration,
            backend: backend.into(),
        };

        let json = serde_json::to_vec_pretty(&session)?;

        // Write with restricted permissions (0600 on Unix)
        let mut file = fs::File::create(&self.path).await?;
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = file.metadata().await?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&self.path, perms).await?;
        }

        file.write_all(&json).await?;
        file.flush().await?;

        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        match fs::remove_file(&self.path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

// Auto-refresh session wrapper
pub struct AutoRefreshSession {
    inner: Box<dyn Session>,
    backend: Arc<tokio::sync::Mutex<Box<dyn Backend>>>,
}

impl AutoRefreshSession {
    pub fn new(session: Box<dyn Session>, backend: Arc<tokio::sync::Mutex<Box<dyn Backend>>>) -> Self {
        Self { inner, backend }
    }
}

#[async_trait::async_trait]
impl Session for AutoRefreshSession {
    fn token(&self) -> &str {
        self.inner.token()
    }

    async fn is_valid(&self) -> bool {
        self.inner.is_valid().await
    }

    async fn refresh(&mut self) -> Result<()> {
        if !self.inner.is_valid().await {
            self.inner.refresh().await?;
        }
        Ok(())
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.inner.expires_at()
    }
}
```

**Key Features**:
- Async file I/O with `tokio::fs`
- Platform-specific permission setting (Unix only)
- Automatic cleanup of invalid/expired sessions

---

### Phase 3: Mock Backend (Week 1)

**File: `src/backends/mock.rs`**

```rust
use crate::*;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct MockBackend {
    items: Arc<RwLock<HashMap<String, Item>>>,
    locations: Arc<RwLock<HashMap<String, bool>>>,
    
    // Error injection for testing
    pub auth_error: Option<VaultmuxError>,
    pub get_error: Option<VaultmuxError>,
    pub create_error: Option<VaultmuxError>,
    pub update_error: Option<VaultmuxError>,
    pub delete_error: Option<VaultmuxError>,
}

impl MockBackend {
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

    pub async fn set_item(&self, name: impl Into<String>, content: impl Into<String>) {
        let item = Item {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            item_type: ItemType::SecureNote,
            notes: Some(content.into()),
            fields: None,
            location: None,
            created: Some(chrono::Utc::now()),
            modified: Some(chrono::Utc::now()),
        };

        let mut items = self.items.write().await;
        items.insert(item.name.clone(), item);
    }
}

pub struct MockSession {
    token: String,
    expires: Option<DateTime<Utc>>,
}

impl MockSession {
    pub fn new() -> Self {
        Self {
            token: "mock-session-token".to_string(),
            expires: None,
        }
    }
}

#[async_trait]
impl Session for MockSession {
    fn token(&self) -> &str {
        &self.token
    }

    async fn is_valid(&self) -> bool {
        if let Some(expires) = self.expires {
            chrono::Utc::now() < expires
        } else {
            true
        }
    }

    async fn refresh(&mut self) -> Result<()> {
        Ok(())
    }

    fn expires_at(&self) -> Option<DateTime<Utc>> {
        self.expires
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
        if let Some(err) = &self.auth_error {
            return Err(VaultmuxError::Other(anyhow::anyhow!("{}", err)));
        }
        Ok(Arc::new(MockSession::new()))
    }

    async fn sync(&mut self, _session: &dyn Session) -> Result<()> {
        Ok(())
    }

    async fn get_item(&self, name: &str, _session: &dyn Session) -> Result<Item> {
        if let Some(err) = &self.get_error {
            return Err(VaultmuxError::Other(anyhow::anyhow!("{}", err)));
        }

        let items = self.items.read().await;
        items.get(name)
            .cloned()
            .ok_or_else(|| VaultmuxError::NotFound(name.to_string()))
    }

    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String> {
        let item = self.get_item(name, session).await?;
        item.notes.ok_or_else(|| VaultmuxError::NotFound(name.to_string()))
    }

    async fn item_exists(&self, name: &str, _session: &dyn Session) -> Result<bool> {
        let items = self.items.read().await;
        Ok(items.contains_key(name))
    }

    async fn list_items(&self, _session: &dyn Session) -> Result<Vec<Item>> {
        let items = self.items.read().await;
        Ok(items.values().cloned().collect())
    }

    async fn create_item(&mut self, name: &str, content: &str, _session: &dyn Session) -> Result<()> {
        if let Some(err) = &self.create_error {
            return Err(VaultmuxError::Other(anyhow::anyhow!("{}", err)));
        }

        let mut items = self.items.write().await;
        if items.contains_key(name) {
            return Err(VaultmuxError::AlreadyExists(name.to_string()));
        }

        let item = Item {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            item_type: ItemType::SecureNote,
            notes: Some(content.to_string()),
            fields: None,
            location: None,
            created: Some(chrono::Utc::now()),
            modified: Some(chrono::Utc::now()),
        };

        items.insert(name.to_string(), item);
        Ok(())
    }

    async fn update_item(&mut self, name: &str, content: &str, _session: &dyn Session) -> Result<()> {
        if let Some(err) = &self.update_error {
            return Err(VaultmuxError::Other(anyhow::anyhow!("{}", err)));
        }

        let mut items = self.items.write().await;
        let item = items.get_mut(name)
            .ok_or_else(|| VaultmuxError::NotFound(name.to_string()))?;

        item.notes = Some(content.to_string());
        item.modified = Some(chrono::Utc::now());
        Ok(())
    }

    async fn delete_item(&mut self, name: &str, _session: &dyn Session) -> Result<()> {
        if let Some(err) = &self.delete_error {
            return Err(VaultmuxError::Other(anyhow::anyhow!("{}", err)));
        }

        let mut items = self.items.write().await;
        items.remove(name)
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
        _session: &dyn Session
    ) -> Result<Vec<Item>> {
        let items = self.items.read().await;
        Ok(items.values()
            .filter(|item| item.location.as_deref() == Some(loc_value))
            .cloned()
            .collect())
    }
}

// Register mock backend
pub(crate) fn register() {
    crate::factory::register_backend("mock", |_cfg| {
        Ok(Box::new(MockBackend::new()))
    });
}
```

**Complete test backend with error injection capabilities**

---

## Part 4: Cargo.toml Configuration

```toml
[package]
name = "vaultmux"
version = "0.1.0"
edition = "2021"
rust-version = "1.75"
authors = ["Dayna Blackwell <dayna@blackwellsystems.com>"]
license = "Apache-2.0"
description = "Unified interface for multi-vault secret management"
repository = "https://github.com/blackwell-systems/vaultmux-rs"
homepage = "https://github.com/blackwell-systems/vaultmux-rs"
documentation = "https://docs.rs/vaultmux"
keywords = ["vault", "secrets", "bitwarden", "1password", "aws"]
categories = ["authentication", "api-bindings"]

[dependencies]
# Core dependencies (always required)
tokio = { version = "1.35", features = ["process", "fs", "io-util", "time", "sync", "macros"] }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1.0"
anyhow = "1.0"
uuid = { version = "1.6", features = ["v4", "serde"] }
lazy_static = "1.4"

# Optional backend dependencies
aws-config = { version = "1.1", optional = true }
aws-sdk-secretsmanager = { version = "1.12", optional = true }
google-secretmanager1 = { version = "5.0", optional = true }
azure_security_keyvault = { version = "0.20", optional = true }
azure_identity = { version = "0.20", optional = true }

[target.'cfg(windows)'.dependencies]
windows = { version = "0.52", features = ["Win32_Security_Credentials"], optional = true }

[dev-dependencies]
tokio = { version = "1.35", features = ["rt-multi-thread", "test-util"] }
tempfile = "3.8"

[features]
default = ["mock"]
full = ["bitwarden", "onepassword", "pass", "wincred", "aws", "gcp", "azure"]

# CLI backends (no extra dependencies)
mock = []
bitwarden = []
onepassword = []
pass = []

# Platform-specific
wincred = ["windows"]

# SDK backends
aws = ["dep:aws-config", "dep:aws-sdk-secretsmanager"]
gcp = ["dep:google-secretmanager1"]
azure = ["dep:azure_security_keyvault", "dep:azure_identity"]

[[example]]
name = "basic"
required-features = ["mock"]

[[example]]
name = "aws"
required-features = ["aws"]
```

---

## Part 5: Implementation Phases

### Week 1: Foundation
- ✅ Core traits (`Backend`, `Session`)
- ✅ Data types (`Item`, `ItemType`, `Config`)
- ✅ Error system with `thiserror`
- ✅ Factory pattern + registration
- ✅ Session caching + auto-refresh
- ✅ Input validation
- ✅ Mock backend + tests

### Week 2: CLI Backends
- Bitwarden backend (CLI: `bw`)
- 1Password backend (CLI: `op`)
- pass backend (CLI: `pass` + `gpg`)
- Status caching (5-second TTL)
- Integration tests

### Week 3: Cloud Backends
- AWS Secrets Manager (SDK)
- GCP Secret Manager (SDK)
- Azure Key Vault (SDK)
- Credential handling
- Integration tests

### Week 4: Platform-Specific + Polish
- Windows Credential Manager (PowerShell)
- Cross-platform compilation
- Documentation (rustdoc)
- Examples
- CI/CD setup

---

## Part 6: Testing Strategy

### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_backend_create_and_get() {
        let mut backend = MockBackend::new();
        backend.init().await.unwrap();
        
        let session = backend.authenticate().await.unwrap();
        
        backend.create_item("test-key", "test-value", &*session).await.unwrap();
        let notes = backend.get_notes("test-key", &*session).await.unwrap();
        
        assert_eq!(notes, "test-value");
    }

    #[tokio::test]
    async fn test_error_injection() {
        let mut backend = MockBackend::new();
        backend.get_error = Some(VaultmuxError::PermissionDenied("test".to_string()));
        
        let session = backend.authenticate().await.unwrap();
        let result = backend.get_notes("anything", &*session).await;
        
        assert!(result.is_err());
    }
}
```

### Integration Tests
```rust
#[cfg(all(test, feature = "integration-bitwarden"))]
mod integration_tests {
    use vaultmux::*;

    #[tokio::test]
    async fn test_bitwarden_roundtrip() {
        let config = Config::new(BackendType::Bitwarden)
            .with_prefix("vaultmux-test");
        
        let mut backend = factory::new_backend(config).unwrap();
        backend.init().await.unwrap();
        
        let session = backend.authenticate().await.unwrap();
        
        // Create
        backend.create_item("test-item", "test-content", &*session).await.unwrap();
        
        // Read
        let notes = backend.get_notes("test-item", &*session).await.unwrap();
        assert_eq!(notes, "test-content");
        
        // Delete
        backend.delete_item("test-item", &*session).await.unwrap();
    }
}
```

---

## Part 7: Performance Considerations

### Status Caching Pattern
```rust
use std::time::{Duration, Instant};

struct StatusCache {
    authenticated: bool,
    timestamp: Instant,
    ttl: Duration,
}

impl StatusCache {
    fn new() -> Self {
        Self {
            authenticated: false,
            timestamp: Instant::now() - Duration::from_secs(10),
            ttl: Duration::from_secs(5),
        }
    }

    fn is_valid(&self) -> bool {
        self.timestamp.elapsed() < self.ttl
    }

    fn update(&mut self, authenticated: bool) {
        self.authenticated = authenticated;
        self.timestamp = Instant::now();
    }

    fn get(&self) -> Option<bool> {
        if self.is_valid() {
            Some(self.authenticated)
        } else {
            None
        }
    }
}
```

**Reduces subprocess overhead by 90%+ for authentication checks**

---

## Part 8: Documentation Plan

### Rustdoc Coverage
- All public types documented
- All public methods documented
- Module-level documentation
- Examples in doc comments

### Example Projects
- `examples/basic.rs` - Quick start with mock backend
- `examples/aws.rs` - AWS Secrets Manager usage
- `examples/multi_backend.rs` - Switching backends at runtime
- `examples/session_management.rs` - Session caching patterns

### README.md
- Feature comparison table
- Installation instructions per backend
- Quick start guide
- API overview
- Comparison to Go version

---

## Part 9: Success Criteria

### Functionality Parity Checklist
- [ ] All 7 backends implemented
- [ ] All 15 `Backend` methods working
- [ ] Session caching with disk persistence
- [ ] Auto-refresh sessions
- [ ] Input validation (command injection prevention)
- [ ] Error wrapping with context
- [ ] Location/folder management
- [ ] Cross-platform support (Windows/Unix)
- [ ] Mock backend for testing
- [ ] 90%+ test coverage

### Performance Targets
- [ ] Status caching reduces auth checks by 90%+
- [ ] Session caching prevents re-authentication
- [ ] Async I/O for all operations
- [ ] Zero-copy where possible (lifetimes)

### Quality Targets
- [ ] All clippy warnings resolved
- [ ] rustfmt applied
- [ ] No unsafe code (unless platform-specific)
- [ ] Full rustdoc coverage
- [ ] Integration tests for all backends

---

## Part 10: Migration Guide (Go → Rust)

### API Mapping

| Go | Rust | Notes |
|----|------|-------|
| `vaultmux.New(config)` | `factory::new_backend(config)` | Same pattern |
| `backend.Init(ctx)` | `backend.init().await` | Async, no context parameter |
| `backend.GetNotes(ctx, name, session)` | `backend.get_notes(name, session).await` | Async, no context |
| `errors.Is(err, vaultmux.ErrNotFound)` | `matches!(err, VaultmuxError::NotFound(_))` | Pattern matching |
| `session.Token()` | `session.token()` | Same API |
| `*Item` | `Item` | No pointers needed |

### Error Handling

**Go**:
```go
notes, err := backend.GetNotes(ctx, "key", session)
if err != nil {
    if errors.Is(err, vaultmux.ErrNotFound) {
        // Handle not found
    }
    return err
}
```

**Rust**:
```rust
let notes = match backend.get_notes("key", &*session).await {
    Ok(n) => n,
    Err(VaultmuxError::NotFound(_)) => {
        // Handle not found
        return;
    }
    Err(e) => return Err(e),
};

// Or with ? operator:
let notes = backend.get_notes("key", &*session).await?;
```

---

## Appendix: Complete File Checklist

### Core Files (8 files)
- [ ] `src/lib.rs` - Public API + re-exports
- [ ] `src/backend.rs` - Backend trait (15 methods)
- [ ] `src/session.rs` - Session trait + SessionCache + AutoRefreshSession
- [ ] `src/item.rs` - Item struct + ItemType enum
- [ ] `src/error.rs` - VaultmuxError with thiserror
- [ ] `src/config.rs` - Config struct + BackendType enum
- [ ] `src/factory.rs` - Backend registration + factory
- [ ] `src/validation.rs` - Input validation

### Backend Files (8 backends × ~3 files each = 24 files)
- [ ] `src/backends/mock.rs` (always compiled)
- [ ] `src/backends/bitwarden/mod.rs`
- [ ] `src/backends/bitwarden/backend.rs`
- [ ] `src/backends/bitwarden/session.rs`
- [ ] `src/backends/onepassword/...` (3 files)
- [ ] `src/backends/pass/...` (2 files)
- [ ] `src/backends/wincred/...` (3 files)
- [ ] `src/backends/aws/...` (2 files)
- [ ] `src/backends/gcp/...` (2 files)
- [ ] `src/backends/azure/...` (2 files)

### Test Files (3 files)
- [ ] `tests/integration_tests.rs`
- [ ] `tests/backend_tests/mock.rs`
- [ ] `tests/backend_tests/...` (per backend)

### Example Files (4 files)
- [ ] `examples/basic.rs`
- [ ] `examples/aws.rs`
- [ ] `examples/multi_backend.rs`
- [ ] `examples/session_management.rs`

### Documentation Files (5 files)
- [ ] `README.md`
- [ ] `CHANGELOG.md`
- [ ] `LICENSE` (Apache 2.0)
- [ ] `CONTRIBUTING.md`
- [ ] `RUST_PORT_PLAN.md` (this file)

**Total: ~50 files for complete 1-to-1 port**

---

## Timeline Summary

| Week | Focus | Deliverables | Status |
|------|-------|--------------|--------|
| **1** | Foundation | Core traits, errors, config, factory, mock backend | 📋 Planned |
| **2** | CLI Backends | Bitwarden, 1Password, pass | 📋 Planned |
| **3** | Cloud Backends | AWS, GCP, Azure | 📋 Planned |
| **4** | Polish | Windows, docs, examples, CI/CD | 📋 Planned |

**Total Estimated Effort**: 4 weeks full-time (160 hours)

---

*This document serves as the complete implementation plan for porting vaultmux from Go to Rust with 100% feature parity and improved type safety.*
