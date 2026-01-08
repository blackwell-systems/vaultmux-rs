# Vaultmux Examples

This directory contains runnable examples demonstrating various vaultmux features and patterns.

## Running Examples

All examples can be run with `cargo run --example <name>`:

```bash
# Basic usage with pass backend
cargo run --example basic_usage --features pass

# Multi-backend fallback
cargo run --example multi_backend_fallback --features "pass,bitwarden"

# AWS Secrets Manager
cargo run --example aws_secrets --features aws

# Environment-based configuration  
VAULT_BACKEND=pass cargo run --example environment_config --features pass

# Error handling patterns
cargo run --example error_handling --features pass

# Credential rotation
cargo run --example credential_rotation --features pass
```

## Examples Overview

### [`basic_usage.rs`](basic_usage.rs)
Demonstrates fundamental operations: create, read, update, delete, and list secrets.

**Features:** Basic CRUD operations, authentication, session management  
**Backend:** pass (Unix password manager)  
**Prerequisites:** `pass` and `gpg` commands installed

### [`aws_secrets.rs`](aws_secrets.rs)
Shows how to use AWS Secrets Manager with proper configuration and error handling.

**Features:** Cloud SDK integration, region configuration, prefix namespacing  
**Backend:** AWS Secrets Manager  
**Prerequisites:** AWS credentials configured, AWS_REGION set

### [`multi_backend_fallback.rs`](multi_backend_fallback.rs)
Demonstrates trying multiple backends until one succeeds - useful for cross-platform applications.

**Features:** Backend detection, graceful fallback, error recovery  
**Backends:** Pass → Bitwarden → 1Password (tries in order)  
**Prerequisites:** At least one backend available

### [`environment_config.rs`](environment_config.rs)
Shows 12-factor app configuration using environment variables.

**Features:** Environment-based config, multiple backend support  
**Environment Variables:**
- `VAULT_BACKEND`: Backend type (pass, bitwarden, aws, gcp, azure)
- `VAULT_PREFIX`: Secret name prefix
- `AWS_REGION`, `GCP_PROJECT`, `AZURE_KEYVAULT_URL`: Backend-specific

### [`credential_rotation.rs`](credential_rotation.rs)
Demonstrates automatic credential rotation with audit logging.

**Features:** Create-or-update pattern, rotation metadata, best practices  
**Backend:** pass  
**Use Case:** Automated security compliance, key rotation schedules

### [`error_handling.rs`](error_handling.rs)
Comprehensive error handling patterns and recovery strategies.

**Features:**
- Specific error variant matching
- Idempotent operations (create-or-update)
- Graceful degradation with fallbacks
- Retry logic for transient failures
- Error context and debugging

**Backend:** pass

## Prerequisites by Backend

### Pass (Unix Password Manager)
```bash
# Install on Ubuntu/Debian
sudo apt install pass gnupg

# Install on macOS  
brew install pass gnupg

# Initialize (first time only)
gpg --gen-key  # Create GPG key
pass init <your-gpg-key-id>
```

### Bitwarden
```bash
# Install Bitwarden CLI
npm install -g @bitwarden/cli

# Login
bw login
bw unlock  # Get session token
```

### 1Password
```bash
# Install 1Password CLI
# Download from: https://1password.com/downloads/command-line/

# Sign in
op signin
```

### AWS Secrets Manager
```bash
# Configure AWS credentials
aws configure

# Or use environment variables
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...
export AWS_REGION=us-east-1
```

### GCP Secret Manager
```bash
# Install gcloud and authenticate
gcloud auth application-default login

# Set project
export GCP_PROJECT=my-project-id
```

### Azure Key Vault
```bash
# Install Azure CLI and login
az login

# Set vault URL
export AZURE_KEYVAULT_URL=https://myvault.vault.azure.net
```

## Common Patterns

### Create or Update (Idempotent)
```rust
match backend.create_item(name, value, session).await {
    Ok(_) => println!("Created"),
    Err(VaultmuxError::AlreadyExists(_)) => {
        backend.update_item(name, value, session).await?;
        println!("Updated");
    }
    Err(e) => return Err(e),
}
```

### Get with Fallback
```rust
let value = match backend.get_notes(name, session).await {
    Ok(v) => v,
    Err(VaultmuxError::NotFound(_)) => "default-value".to_string(),
    Err(e) => return Err(e),
};
```

### Retry on Failure
```rust
for attempt in 1..=3 {
    match backend.get_notes(name, session).await {
        Ok(value) => return Ok(value),
        Err(e) if attempt < 3 => {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }
        Err(e) => return Err(e),
    }
}
```

## Testing Examples

Most examples use the `pass` backend for demonstration. To test without installing dependencies, you can modify the examples to use feature-gated backends you have available.

## Contributing

Have a useful pattern or use case? Feel free to contribute additional examples!

1. Create a new `.rs` file in this directory
2. Add documentation at the top explaining what it demonstrates
3. Include prerequisites and run instructions
4. Test with `cargo run --example your_example`
5. Submit a pull request

## See Also

- [Main Documentation](../README.md)
- [API Documentation](https://docs.rs/vaultmux)
- [Architecture Guide](../RUST_PORT_PLAN.md)
