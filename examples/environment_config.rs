//! Environment-based configuration example.
//!
//! Demonstrates how to configure backends using environment variables.
//! This is useful for 12-factor apps and container deployments.
//!
//! Run with: cargo run --example environment_config
//!
//! Environment variables:
//! - VAULT_BACKEND: Backend type (mock, pass, bitwarden, aws, gcp, azure)
//! - VAULT_PREFIX: Secret name prefix
//! - AWS_REGION: AWS region (for AWS backend)
//! - GCP_PROJECT: GCP project ID (for GCP backend)  
//! - AZURE_KEYVAULT_URL: Azure Key Vault URL (for Azure backend)

use std::env;
use vaultmux::{factory, Backend, Config, BackendType};

fn get_backend_from_env() -> vaultmux::Result<BackendType> {
    let backend_str = env::var("VAULT_BACKEND").unwrap_or_else(|_| "mock".to_string());
    
    let backend_type = match backend_str.to_lowercase().as_str() {
        "mock" => BackendType::Pass,
        "pass" => BackendType::Pass,
        "bitwarden" => BackendType::Bitwarden,
        "onepassword" | "1password" => BackendType::OnePassword,
        "aws" | "awssecrets" => BackendType::AWSSecretsManager,
        "gcp" | "gcpsecrets" => BackendType::GCPSecretManager,
        "azure" | "azurekeyvault" => BackendType::AzureKeyVault,
        "wincred" | "windows" => BackendType::WindowsCredentialManager,
        _ => {
            return Err(vaultmux::VaultmuxError::Other(anyhow::anyhow!(
                "Unknown backend type: {}. Valid options: mock, pass, bitwarden, onepassword, aws, gcp, azure, wincred",
                backend_str
            )));
        }
    };
    
    Ok(backend_type)
}

fn build_config() -> vaultmux::Result<Config> {
    let backend_type = get_backend_from_env()?;
    let mut config = Config::new(backend_type);
    
    // Add prefix if specified
    if let Ok(prefix) = env::var("VAULT_PREFIX") {
        config = config.with_prefix(&prefix);
    }
    
    // Backend-specific configuration
    match backend_type {
        BackendType::AWSSecretsManager => {
            if let Ok(region) = env::var("AWS_REGION") {
                config = config.with_option("region", region);
            }
        }
        BackendType::GCPSecretManager => {
            if let Ok(project) = env::var("GCP_PROJECT") {
                config = config.with_option("project_id", project);
            }
        }
        BackendType::AzureKeyVault => {
            if let Ok(vault_url) = env::var("AZURE_KEYVAULT_URL") {
                config = config.with_option("vault_url", vault_url);
            }
        }
        _ => {}
    }
    
    Ok(config)
}

#[tokio::main]
async fn main() -> vaultmux::Result<()> {
    println!("=== Environment-Based Configuration Example ===\n");
    
    // Show current environment configuration
    println!("Environment configuration:");
    println!("  VAULT_BACKEND: {}", env::var("VAULT_BACKEND").unwrap_or_else(|_| "mock (default)".to_string()));
    println!("  VAULT_PREFIX: {}", env::var("VAULT_PREFIX").unwrap_or_else(|_| "(none)".to_string()));
    
    // Build configuration from environment
    let config = build_config()?;
    println!("\nBackend: {:?}", config.backend);
    println!("Prefix: {}", config.prefix);
    
    // Create and initialize backend
    let mut backend = factory::new_backend(config)?;
    println!("\nInitializing backend: {}", backend.name());
    backend.init().await?;
    
    println!("Authenticating...");
    let session = backend.authenticate().await?;
    println!("✓ Authentication successful\n");
    
    // Demonstrate usage
    let test_secret = "app-config";
    let test_value = r#"{"database": "postgres://localhost/myapp", "cache": "redis://localhost"}"#;
    
    println!("Creating example secret '{}'...", test_secret);
    match backend.create_item(test_secret, test_value, &*session).await {
        Ok(_) => println!("✓ Secret created"),
        Err(vaultmux::VaultmuxError::AlreadyExists(_)) => {
            println!("Secret already exists");
        }
        Err(e) => return Err(e),
    }
    
    // Retrieve and display
    println!("\nRetrieving secret...");
    let retrieved = backend.get_notes(test_secret, &*session).await?;
    println!("✓ Retrieved:\n{}", retrieved);
    
    // Clean up
    println!("\nCleaning up...");
    backend.delete_item(test_secret, &*session).await?;
    println!("✓ Secret deleted");
    
    println!("\n=== Configuration Tips ===");
    println!("• Use environment variables for 12-factor app compliance");
    println!("• Set VAULT_BACKEND to change backends without code changes");
    println!("• Use VAULT_PREFIX to namespace secrets per environment");
    println!("• Backend-specific vars (AWS_REGION, GCP_PROJECT, etc.)");
    println!("\n=== Example Complete ===");
    
    Ok(())
}
