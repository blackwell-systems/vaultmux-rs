# Vaultmux User Guide

Complete guide to using vaultmux for unified secret management in Rust.

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
- [Backend Configuration](#backend-configuration)
- [Common Patterns](#common-patterns)
- [Error Handling](#error-handling)
- [Testing](#testing)
- [Best Practices](#best-practices)

## Overview

Vaultmux provides a unified async interface for interacting with multiple secret management systems. Write your code once and support any backend through a consistent API.

### When to Use Vaultmux

- **Multi-cloud deployments** - Support AWS, GCP, and Azure without code changes
- **Developer flexibility** - Let developers use their preferred password manager (pass, 1Password, Bitwarden)
- **Testing** - Use mock backend in tests, real backend in production
- **Migration** - Switch backends without rewriting application code

### Key Features

- Unified `Backend` trait works with any vault
- Async/await built on tokio
- Type safety with Rust enums
- Session caching for performance
- Optional backend compilation via feature flags
- Mock backend for testing

## Installation

Add vaultmux to your `Cargo.toml`:

```toml
[dependencies]
vaultmux = "0.1"
```

### Feature Flags

Enable only the backends you need:

```toml
[dependencies]
vaultmux = { version = "0.1", features = ["bitwarden", "aws"] }
```

Available features:
- `mock` (default) - Mock backend for testing
- `pass` - Unix password-store
- `bitwarden` - Bitwarden CLI
- `onepassword` - 1Password CLI
- `aws` - AWS Secrets Manager
- `gcp` - Google Cloud Secret Manager
- `azure` - Azure Key Vault
- `wincred` - Windows Credential Manager
- `full` - All backends

## Quick Start

```rust
use vaultmux::{factory, Backend, BackendType, Config};

#[tokio::main]
async fn main() -> vaultmux::Result<()> {
    // Initialize the library (registers backends)
    vaultmux::init();

    // Create backend configuration
    let config = Config::new(BackendType::Pass)
        .with_prefix("myapp");

    // Create and initialize backend
    let mut backend = factory::new_backend(config)?;
    backend.init().await?;

    // Authenticate
    let session = backend.authenticate().await?;

    // Store a secret
    backend.create_item("api-key", "secret-value", &*session).await?;

    // Retrieve it
    let secret = backend.get_notes("api-key", &*session).await?;
    println!("Secret: {}", secret);

    // Clean up
    backend.delete_item("api-key", &*session).await?;

    Ok(())
}
```

## Core Concepts

### Backend

The `Backend` trait defines the interface all backends implement:

```rust
#[async_trait]
pub trait Backend: Send + Sync {
    fn name(&self) -> &str;
    async fn init(&mut self) -> Result<()>;
    async fn authenticate(&self) -> Result<Arc<dyn Session>>;
    async fn create_item(&self, name: &str, notes: &str, session: &dyn Session) -> Result<()>;
    async fn get_notes(&self, name: &str, session: &dyn Session) -> Result<String>;
    async fn update_item(&self, name: &str, notes: &str, session: &dyn Session) -> Result<()>;
    async fn delete_item(&self, name: &str, session: &dyn Session) -> Result<()>;
    async fn list_items(&self, session: &dyn Session) -> Result<Vec<Item>>;
    // ... more methods
}
```

### Session

Sessions represent authenticated connections to backends. They handle:
- Authentication state
- Token caching
- Expiry checking

```rust
let session = backend.authenticate().await?;

// Check if still valid
if session.is_expired() {
    let session = backend.authenticate().await?;
}
```

### Configuration

`Config` uses the builder pattern for flexible setup:

```rust
let config = Config::new(BackendType::AWSSecretsManager)
    .with_prefix("prod-")                    // Namespace secrets
    .with_option("region", "us-west-2")     // Backend-specific options
    .with_session_cache_enabled(true);      // Enable session caching
```

### Prefixes

Prefixes namespace your secrets within a backend:

```rust
let config = Config::new(BackendType::Pass)
    .with_prefix("myapp/");

// Creates: myapp/api-key
backend.create_item("api-key", "value", &*session).await?;

// Returns: "api-key" (prefix stripped)
let items = backend.list_items(&*session).await?;
```

## Backend Configuration

### Pass (Unix Password Store)

```rust
let config = Config::new(BackendType::Pass)
    .with_prefix("myapp/");
```

Requirements:
- `pass` command installed
- `gpg` configured with keys
- Password store initialized (`pass init`)

### Bitwarden

```rust
let config = Config::new(BackendType::Bitwarden)
    .with_prefix("myapp-");
```

Requirements:
- `bw` CLI installed
- User logged in (`bw login`)

### 1Password

```rust
let config = Config::new(BackendType::OnePassword)
    .with_prefix("myapp-");
```

Requirements:
- `op` CLI installed
- User signed in (`op signin`)

### AWS Secrets Manager

```rust
let config = Config::new(BackendType::AWSSecretsManager)
    .with_prefix("myapp/")
    .with_option("region", "us-east-1");
```

Requirements:
- AWS credentials configured (env vars, ~/.aws/credentials, or IAM role)
- Permissions: `secretsmanager:GetSecretValue`, `secretsmanager:CreateSecret`, etc.

### Google Cloud Secret Manager

```rust
let config = Config::new(BackendType::GCPSecretManager)
    .with_prefix("myapp-")
    .with_option("project_id", "my-project");
```

Requirements:
- GCP credentials configured (GOOGLE_APPLICATION_CREDENTIALS or default service account)
- Permissions: `secretmanager.secrets.create`, `secretmanager.versions.access`, etc.

### Azure Key Vault

```rust
let config = Config::new(BackendType::AzureKeyVault)
    .with_prefix("myapp-")
    .with_option("vault_url", "https://myvault.vault.azure.net");
```

Requirements:
- Azure credentials configured (env vars or managed identity)
- Permissions: `Key Vault Secrets User` role or equivalent

### Windows Credential Manager

```rust
let config = Config::new(BackendType::WindowsCredentialManager)
    .with_prefix("myapp:");
```

Requirements:
- Windows OS
- PowerShell available

## Common Patterns

### Environment-Based Backend Selection

```rust
use std::env;

let backend_type = match env::var("VAULT_BACKEND").as_deref() {
    Ok("aws") => BackendType::AWSSecretsManager,
    Ok("gcp") => BackendType::GCPSecretManager,
    Ok("pass") => BackendType::Pass,
    _ => BackendType::Mock, // Default for development
};

let config = Config::new(backend_type).with_prefix("myapp/");
let mut backend = factory::new_backend(config)?;
backend.init().await?;
```

### Multi-Backend Fallback

```rust
async fn get_secret(name: &str) -> Result<String> {
    let backends = vec![
        BackendType::Pass,
        BackendType::Bitwarden,
        BackendType::Mock,
    ];

    for backend_type in backends {
        let config = Config::new(backend_type).with_prefix("myapp/");
        if let Ok(mut backend) = factory::new_backend(config) {
            if backend.init().await.is_ok() {
                if let Ok(session) = backend.authenticate().await {
                    if let Ok(secret) = backend.get_notes(name, &*session).await {
                        return Ok(secret);
                    }
                }
            }
        }
    }

    Err(VaultmuxError::NotFound(name.to_string()))
}
```

### Session Reuse

```rust
struct SecretManager {
    backend: Box<dyn Backend>,
    session: Arc<dyn Session>,
}

impl SecretManager {
    async fn new(config: Config) -> Result<Self> {
        let mut backend = factory::new_backend(config)?;
        backend.init().await?;
        let session = backend.authenticate().await?;
        Ok(Self { backend, session })
    }

    async fn get(&self, name: &str) -> Result<String> {
        // Reuse cached session
        self.backend.get_notes(name, &*self.session).await
    }

    async fn set(&self, name: &str, value: &str) -> Result<()> {
        self.backend.create_item(name, value, &*self.session).await
    }
}
```

### Bulk Operations

```rust
async fn backup_secrets(
    source: &dyn Backend,
    dest: &dyn Backend,
    session: &dyn Session,
) -> Result<usize> {
    let items = source.list_items(session).await?;
    let mut count = 0;

    for item in items {
        let value = source.get_notes(&item.name, session).await?;
        dest.create_item(&item.name, &value, session).await?;
        count += 1;
    }

    Ok(count)
}
```

### Item Type Filtering

```rust
use vaultmux::ItemType;

async fn list_ssh_keys(
    backend: &dyn Backend,
    session: &dyn Session,
) -> Result<Vec<Item>> {
    let items = backend.list_items(session).await?;
    Ok(items
        .into_iter()
        .filter(|item| item.item_type == ItemType::SSHKey)
        .collect())
}
```

## Error Handling

### Error Types

```rust
use vaultmux::VaultmuxError;

match backend.get_notes("api-key", &*session).await {
    Ok(value) => println!("Secret: {}", value),
    Err(VaultmuxError::NotFound(name)) => {
        println!("Secret {} not found", name);
    }
    Err(VaultmuxError::NotAuthenticated) => {
        println!("Session expired, re-authenticating...");
        let session = backend.authenticate().await?;
    }
    Err(VaultmuxError::BackendNotInstalled(cmd)) => {
        println!("Missing CLI: {}", cmd);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

### Retry Logic

```rust
async fn get_with_retry(
    backend: &dyn Backend,
    name: &str,
    session: &dyn Session,
    retries: u32,
) -> Result<String> {
    for attempt in 0..retries {
        match backend.get_notes(name, session).await {
            Ok(value) => return Ok(value),
            Err(VaultmuxError::SessionExpired) if attempt < retries - 1 => {
                // Re-authenticate and retry
                let session = backend.authenticate().await?;
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!()
}
```

## Testing

### Using Mock Backend

```rust
use vaultmux::backends::mock::MockBackend;
use vaultmux::Backend;

#[tokio::test]
async fn test_my_code() {
    let mut backend = MockBackend::new();
    backend.set_item("test-key", "test-value").await;

    // Test your code
    let result = my_function(&backend).await;
    assert!(result.is_ok());
}
```

### Injecting Errors

```rust
use vaultmux::VaultmuxError;

#[tokio::test]
async fn test_error_handling() {
    let mut backend = MockBackend::new();
    backend.get_error = Some(VaultmuxError::NotFound("test".into()));

    let result = my_function(&backend).await;
    assert!(result.is_err());
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    async fn setup_backend() -> Box<dyn Backend> {
        let backend_type = if cfg!(test) {
            BackendType::Mock
        } else {
            BackendType::Pass
        };

        let config = Config::new(backend_type).with_prefix("test-");
        let mut backend = factory::new_backend(config).unwrap();
        backend.init().await.unwrap();
        backend
    }

    #[tokio::test]
    async fn test_full_workflow() {
        let backend = setup_backend().await;
        let session = backend.authenticate().await.unwrap();

        // Create, read, update, delete
        backend.create_item("test", "value1", &*session).await.unwrap();
        let val = backend.get_notes("test", &*session).await.unwrap();
        assert_eq!(val, "value1");

        backend.update_item("test", "value2", &*session).await.unwrap();
        let val = backend.get_notes("test", &*session).await.unwrap();
        assert_eq!(val, "value2");

        backend.delete_item("test", &*session).await.unwrap();
    }
}
```

## Best Practices

### 1. Initialize Once

```rust
// Good: Initialize at application startup
#[tokio::main]
async fn main() {
    vaultmux::init(); // Registers all backends
    // ... rest of application
}

// Bad: Initialize in hot path
async fn get_secret() {
    vaultmux::init(); // Don't do this repeatedly
}
```

### 2. Use Prefixes for Namespacing

```rust
// Good: Namespace by environment and app
let config = Config::new(backend_type)
    .with_prefix("prod/myapp/");

// Bad: No prefix, secrets collide
let config = Config::new(backend_type);
```

### 3. Handle Session Expiry

```rust
// Good: Check expiry and re-authenticate
if session.is_expired() {
    session = backend.authenticate().await?;
}

// Bad: Assume session is always valid
let value = backend.get_notes(name, &*session).await?;
```

### 4. Validate Input

```rust
// Good: Validate before calling backend
fn validate_secret_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(VaultmuxError::InvalidItemName(name.to_string()));
    }
    if name.contains(['/', '\\', '\0']) {
        return Err(VaultmuxError::InvalidItemName(name.to_string()));
    }
    Ok(())
}

// Use it
validate_secret_name(&secret_name)?;
backend.create_item(&secret_name, value, &*session).await?;
```

### 5. Clean Up Resources

```rust
// Good: Explicit cleanup
backend.delete_item("temp-secret", &*session).await?;

// Better: RAII pattern
struct TempSecret<'a> {
    backend: &'a dyn Backend,
    session: &'a dyn Session,
    name: String,
}

impl<'a> Drop for TempSecret<'a> {
    fn drop(&mut self) {
        // Note: Can't await in Drop, use async-dropper crate or explicit cleanup
    }
}
```

### 6. Log Appropriately

```rust
use tracing::{info, warn, error};

// Good: Log operations without exposing secrets
info!("Retrieving secret: {}", secret_name);
let value = backend.get_notes(&secret_name, &*session).await?;
info!("Retrieved secret successfully");

// Bad: Log secret values
error!("Failed to get secret: {} = {}", secret_name, value); // Don't log values!
```

### 7. Use Type Safety

```rust
// Good: Use ItemType enum
let item = Item {
    name: "key".to_string(),
    item_type: ItemType::SSHKey,
    notes: Some(private_key),
    ..Default::default()
};

// Bad: String constants
let item_type = "ssh_key"; // Typo-prone
```

### 8. Handle Backend Unavailability

```rust
// Good: Graceful degradation
let backend = match factory::new_backend(config) {
    Ok(b) => b,
    Err(VaultmuxError::BackendNotInstalled(cmd)) => {
        warn!("Backend {} not available, using mock", cmd);
        factory::new_backend(Config::new(BackendType::Mock))?
    }
    Err(e) => return Err(e),
};
```

## Next Steps

- See [API Reference](api-reference.md) for detailed API documentation
- Check [examples/](../examples/) for complete working examples
- Read [CONTRIBUTING.md](../CONTRIBUTING.md) to contribute backends or fixes

## Getting Help

- Open an issue on GitHub
- Check existing issues for solutions
- Read the inline documentation: `cargo doc --open`
