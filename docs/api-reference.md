# Vaultmux API Reference

Complete API reference for vaultmux v0.1.0.

## Table of Contents

- [Core Traits](#core-traits)
- [Types](#types)
- [Factory Functions](#factory-functions)
- [Error Types](#error-types)
- [Backend Implementations](#backend-implementations)

## Core Traits

### Backend

The main trait all backends implement.

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    fn name(&self) -> &str;
    async fn init(&mut self) -> Result<()>;
    async fn authenticate(&self) -> Result<Arc<dyn Session>>;
    async fn create_item(&self, name: &str, notes: &str, session: &dyn Session) -> Result<()>;
    async fn get_item(&self, name: &str, session: &dyn Session) -> Result<Item>;
    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String>;
    async fn update_item(&self, name: &str, notes: &str, session: &dyn Session) -> Result<()>;
    async fn delete_item(&self, name: &str, session: &dyn Session) -> Result<()>;
    async fn item_exists(&self, name: &str, session: &dyn Session) -> Result<bool>;
    async fn list_items(&self, session: &dyn Session) -> Result<Vec<Item>>;
    async fn get_session_token(&self, session: &dyn Session) -> Result<String>;
    async fn validate_session(&self, session: &dyn Session) -> Result<bool>;
}
```

#### Methods

##### `fn name(&self) -> &str`

Returns the backend's identifier string.

```rust
let name = backend.name(); // "pass", "bitwarden", "awssecrets", etc.
```

##### `async fn init(&mut self) -> Result<()>`

Initializes the backend. Must be called before any other operations.

```rust
let mut backend = factory::new_backend(config)?;
backend.init().await?; // Connects to backend, validates config
```

**Errors:**
- `BackendNotInstalled` - Required CLI not found
- `Other` - Initialization failed

##### `async fn authenticate(&self) -> Result<Arc<dyn Session>>`

Authenticates with the backend and returns a session.

```rust
let session = backend.authenticate().await?;
```

**Caching:** Sessions may be cached to disk for performance.

**Errors:**
- `NotAuthenticated` - Authentication failed
- `BackendNotInstalled` - Required CLI not found
- `Other` - Authentication error

##### `async fn create_item(&self, name: &str, notes: &str, session: &dyn Session) -> Result<()>`

Creates a new secret item.

```rust
backend.create_item("api-key", "secret-value", &*session).await?;
```

**Parameters:**
- `name` - Item name (will be prefixed if configured)
- `notes` - Secret value to store
- `session` - Valid session from `authenticate()`

**Errors:**
- `AlreadyExists` - Item with this name exists
- `InvalidItemName` - Name contains invalid characters
- `SessionExpired` - Session no longer valid
- `PermissionDenied` - Insufficient permissions
- `Other` - Backend-specific error

##### `async fn get_item(&self, name: &str, session: &dyn Session) -> Result<Item>`

Retrieves complete item with metadata.

```rust
let item = backend.get_item("api-key", &*session).await?;
println!("Created: {:?}", item.created);
println!("Type: {:?}", item.item_type);
println!("Value: {}", item.notes.unwrap_or_default());
```

**Returns:** `Item` with all fields populated

**Errors:**
- `NotFound` - Item doesn't exist
- `SessionExpired` - Session no longer valid
- `Other` - Backend-specific error

##### `async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String>`

Retrieves only the secret value (optimized).

```rust
let value = backend.get_notes("api-key", &*session).await?;
```

**Returns:** Secret value as string

**Errors:**
- `NotFound` - Item doesn't exist
- `SessionExpired` - Session no longer valid
- `Other` - Backend-specific error

##### `async fn update_item(&self, name: &str, notes: &str, session: &dyn Session) -> Result<()>`

Updates an existing item's value.

```rust
backend.update_item("api-key", "new-value", &*session).await?;
```

**Errors:**
- `NotFound` - Item doesn't exist
- `SessionExpired` - Session no longer valid
- `Other` - Backend-specific error

##### `async fn delete_item(&self, name: &str, session: &dyn Session) -> Result<()>`

Deletes an item permanently.

```rust
backend.delete_item("api-key", &*session).await?;
```

**Note:** This operation is irreversible in most backends.

**Errors:**
- `NotFound` - Item doesn't exist (may succeed anyway)
- `SessionExpired` - Session no longer valid
- `PermissionDenied` - Insufficient permissions
- `Other` - Backend-specific error

##### `async fn item_exists(&self, name: &str, session: &dyn Session) -> Result<bool>`

Checks if an item exists without retrieving it.

```rust
if backend.item_exists("api-key", &*session).await? {
    println!("Key exists");
}
```

**Returns:** `true` if item exists, `false` otherwise

**Errors:**
- `SessionExpired` - Session no longer valid
- `Other` - Backend-specific error

##### `async fn list_items(&self, session: &dyn Session) -> Result<Vec<Item>>`

Lists all items matching the configured prefix.

```rust
let items = backend.list_items(&*session).await?;
for item in items {
    println!("{}: {}", item.name, item.item_type);
}
```

**Returns:** Vector of items (without secret values for performance)

**Note:** Prefix is automatically stripped from returned names.

**Errors:**
- `SessionExpired` - Session no longer valid
- `Other` - Backend-specific error

##### `async fn get_session_token(&self, session: &dyn Session) -> Result<String>`

Gets the raw session token for advanced use cases.

```rust
let token = backend.get_session_token(&*session).await?;
```

**Errors:**
- `NotAuthenticated` - No valid session
- `Other` - Backend-specific error

##### `async fn validate_session(&self, session: &dyn Session) -> Result<bool>`

Validates if a session is still active.

```rust
if !backend.validate_session(&*session).await? {
    let session = backend.authenticate().await?;
}
```

**Returns:** `true` if valid, `false` if expired

**Errors:**
- `Other` - Backend-specific error

### Session

Represents an authenticated session with a backend.

```rust
pub trait Session: Send + Sync {
    fn token(&self) -> &str;
    fn expiry(&self) -> Option<chrono::DateTime<chrono::Utc>>;
    fn is_expired(&self) -> bool;
    fn backend_type(&self) -> &str;
}
```

#### Methods

##### `fn token(&self) -> &str`

Returns the session token.

```rust
let token = session.token();
```

##### `fn expiry(&self) -> Option<chrono::DateTime<chrono::Utc>>`

Returns session expiration time, if known.

```rust
if let Some(expiry) = session.expiry() {
    println!("Expires at: {}", expiry);
}
```

##### `fn is_expired(&self) -> bool`

Checks if session has expired.

```rust
if session.is_expired() {
    // Re-authenticate
}
```

##### `fn backend_type(&self) -> &str`

Returns the backend type identifier.

```rust
let backend = session.backend_type(); // "pass", "bitwarden", etc.
```

## Types

### Config

Backend configuration builder.

```rust
pub struct Config {
    pub backend_type: BackendType,
    pub prefix: String,
    pub options: std::collections::HashMap<String, String>,
    pub session_cache_enabled: bool,
}
```

#### Methods

```rust
impl Config {
    pub fn new(backend_type: BackendType) -> Self;
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self;
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn with_session_cache_enabled(mut self, enabled: bool) -> Self;
}
```

**Example:**

```rust
let config = Config::new(BackendType::AWSSecretsManager)
    .with_prefix("prod/")
    .with_option("region", "us-west-2")
    .with_session_cache_enabled(true);
```

### BackendType

Enum of all supported backends.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackendType {
    Mock,
    Pass,
    Bitwarden,
    OnePassword,
    AWSSecretsManager,
    GCPSecretManager,
    AzureKeyVault,
    WindowsCredentialManager,
}
```

#### Methods

```rust
impl BackendType {
    pub fn as_str(&self) -> &'static str;
}
```

**Example:**

```rust
let name = BackendType::Pass.as_str(); // "pass"
```

### Item

Represents a secret item with metadata.

```rust
#[derive(Debug, Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub item_type: ItemType,
    pub notes: Option<String>,
    pub fields: Option<serde_json::Value>,
    pub location: Option<String>,
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}
```

**Fields:**
- `id` - Backend-specific unique identifier
- `name` - Item name (prefix stripped)
- `item_type` - Type of secret
- `notes` - Secret value (optional, not populated by `list_items`)
- `fields` - Additional structured data (backend-specific)
- `location` - Organizational location (folder, vault, etc.)
- `created` - Creation timestamp
- `modified` - Last modification timestamp

### ItemType

Type of secret item.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    SecureNote,
    Login,
    CreditCard,
    Identity,
    SSHKey,
    APIKey,
    Database,
    Other,
}
```

## Factory Functions

### `factory::new_backend`

Creates a backend from configuration.

```rust
pub fn new_backend(config: Config) -> Result<Box<dyn Backend>>
```

**Example:**

```rust
let config = Config::new(BackendType::Pass).with_prefix("app/");
let backend = factory::new_backend(config)?;
```

**Errors:**
- `Other` - Unknown backend or backend not enabled via feature flag

### `init()`

Initializes the vaultmux library by registering all enabled backends.

```rust
pub fn init()
```

**Example:**

```rust
fn main() {
    vaultmux::init();
    // Now backends can be created
}
```

**Note:** Must be called before `factory::new_backend()`.

## Error Types

### VaultmuxError

```rust
#[derive(Debug, thiserror::Error)]
pub enum VaultmuxError {
    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("Item already exists: {0}")]
    AlreadyExists(String),

    #[error("Not authenticated")]
    NotAuthenticated,

    #[error("Session expired")]
    SessionExpired,

    #[error("Backend not installed: {0}")]
    BackendNotInstalled(String),

    #[error("Invalid item name: {0}")]
    InvalidItemName(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

### Result Type Alias

```rust
pub type Result<T> = std::result::Result<T, VaultmuxError>;
```

## Backend Implementations

### MockBackend

Test backend with in-memory storage.

```rust
use vaultmux::backends::mock::MockBackend;

let mut backend = MockBackend::new();
backend.set_item("key", "value").await;
```

**Additional Methods:**

```rust
impl MockBackend {
    pub fn new() -> Self;
    pub async fn set_item(&mut self, name: &str, value: &str);
    pub async fn clear(&mut self);
    pub fn set_authenticate_error(&mut self, error: Option<VaultmuxError>);
    pub fn set_get_error(&mut self, error: Option<VaultmuxError>);
}
```

### PassBackend

Unix password-store backend.

```rust
use vaultmux::backends::pass::PassBackend;

let config = Config::new(BackendType::Pass).with_prefix("myapp/");
let mut backend = factory::new_backend(config)?;
```

**Requirements:**
- `pass` command
- `gpg` with configured keys

### BitwardenBackend

Bitwarden CLI backend.

```rust
let config = Config::new(BackendType::Bitwarden).with_prefix("myapp-");
let mut backend = factory::new_backend(config)?;
```

**Requirements:**
- `bw` CLI
- User logged in

### OnePasswordBackend

1Password CLI backend.

```rust
let config = Config::new(BackendType::OnePassword).with_prefix("myapp-");
let mut backend = factory::new_backend(config)?;
```

**Requirements:**
- `op` CLI v2+
- User signed in

### AWSBackend

AWS Secrets Manager backend.

```rust
let config = Config::new(BackendType::AWSSecretsManager)
    .with_prefix("myapp/")
    .with_option("region", "us-east-1");
let mut backend = factory::new_backend(config)?;
```

**Options:**
- `region` - AWS region (default: "us-east-1")
- `endpoint` - Custom endpoint URL (for LocalStack testing)

**Requirements:**
- AWS credentials configured
- IAM permissions for Secrets Manager

### GCPBackend

Google Cloud Secret Manager backend.

```rust
let config = Config::new(BackendType::GCPSecretManager)
    .with_prefix("myapp-")
    .with_option("project_id", "my-project");
let mut backend = factory::new_backend(config)?;
```

**Options:**
- `project_id` - GCP project ID (required)

**Requirements:**
- GCP credentials configured
- Secret Manager API enabled

### AzureBackend

Azure Key Vault backend.

```rust
let config = Config::new(BackendType::AzureKeyVault)
    .with_prefix("myapp-")
    .with_option("vault_url", "https://myvault.vault.azure.net");
let mut backend = factory::new_backend(config)?;
```

**Options:**
- `vault_url` - Key Vault URL (required)

**Requirements:**
- Azure credentials configured
- Key Vault access policy set

### WinCredBackend

Windows Credential Manager backend.

```rust
let config = Config::new(BackendType::WindowsCredentialManager)
    .with_prefix("myapp:");
let mut backend = factory::new_backend(config)?;
```

**Requirements:**
- Windows OS
- PowerShell

## Version Information

This reference documents vaultmux v0.1.0.

For the latest documentation, see:
- Online docs: https://docs.rs/vaultmux
- Local docs: `cargo doc --open`
- Examples: [../examples/](../examples/)
