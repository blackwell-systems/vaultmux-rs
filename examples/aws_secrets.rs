//! AWS Secrets Manager example.
//!
//! Prerequisites:
//! - AWS credentials configured (via environment, ~/.aws/credentials, or IAM role)
//! - AWS_REGION environment variable set (or use config option)
//!
//! Run with: cargo run --example aws_secrets --features aws

#[cfg(feature = "aws")]
use vaultmux::{factory, Backend, Config, BackendType};

#[cfg(feature = "aws")]
#[tokio::main]
async fn main() -> vaultmux::Result<()> {
    println!("=== AWS Secrets Manager Example ===\n");

    // Get region from environment or use default
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    println!("Using region: {}", region);

    // Create AWS backend with configuration
    let config = Config::new(BackendType::AWSSecretsManager)
        .with_option("region", region)
        .with_prefix("myapp/"); // Prefix all secret names

    let mut backend = factory::new_backend(config)?;

    println!("Initializing AWS Secrets Manager backend...");
    backend.init().await?;

    println!("Authenticating (using AWS credentials)...");
    let session = backend.authenticate().await?;
    println!("✓ Authenticated\n");

    // Example: Create a database connection string
    let secret_name = "database-url";
    let secret_value = "postgresql://user:pass@localhost:5432/mydb";

    println!("Creating secret '{}'...", secret_name);
    match backend
        .create_item(secret_name, secret_value, &*session)
        .await
    {
        Ok(_) => println!("✓ Secret created"),
        Err(vaultmux::VaultmuxError::AlreadyExists(_)) => {
            println!("Secret already exists, updating...");
            backend
                .update_item(secret_name, secret_value, &*session)
                .await?;
            println!("✓ Secret updated");
        }
        Err(e) => return Err(e),
    }

    // Retrieve the secret
    println!("\nRetrieving secret '{}'...", secret_name);
    let retrieved = backend.get_notes(secret_name, &*session).await?;
    println!("✓ Retrieved: {}", retrieved);

    // List all secrets with our prefix
    println!("\nListing all secrets with prefix 'myapp/'...");
    let items = backend.list_items(&*session).await?;
    println!("✓ Found {} secret(s):", items.len());
    for item in &items {
        println!("  - {} (ID: {})", item.name, item.id);
    }

    // Clean up (optional - comment out to keep the secret)
    println!("\nCleaning up...");
    backend.delete_item(secret_name, &*session).await?;
    println!("✓ Secret deleted");

    println!("\n=== Example Complete ===");
    Ok(())
}

#[cfg(not(feature = "aws"))]
fn main() {
    eprintln!("This example requires the 'aws' feature.");
    eprintln!("Run with: cargo run --example aws_secrets --features aws");
    std::process::exit(1);
}
