//! Basic usage example showing common operations with the mock backend.
//!
//! Run with: cargo run --example basic_usage

use vaultmux::{factory, Backend, Config, BackendType};

#[tokio::main]
async fn main() -> vaultmux::Result<()> {
    println!("=== Basic Vaultmux Usage ===\n");

    // Create a mock backend (no external dependencies)
    let config = Config::new(BackendType::Pass);
    let mut backend = factory::new_backend(config)?;

    println!("1. Initializing backend: {}", backend.name());
    backend.init().await?;

    println!("2. Authenticating...");
    let session = backend.authenticate().await?;
    println!("   ✓ Authenticated");

    // Create a secret
    println!("\n3. Creating secret 'api-key'...");
    backend
        .create_item("api-key", "secret-value-12345", &*session)
        .await?;
    println!("   ✓ Secret created");

    // Retrieve the secret
    println!("\n4. Retrieving secret 'api-key'...");
    let value = backend.get_notes("api-key", &*session).await?;
    println!("   ✓ Retrieved value: {}", value);

    // List all secrets
    println!("\n5. Listing all secrets...");
    let items = backend.list_items(&*session).await?;
    println!("   ✓ Found {} secret(s):", items.len());
    for item in &items {
        println!("     - {}", item.name);
    }

    // Update the secret
    println!("\n6. Updating secret 'api-key'...");
    backend
        .update_item("api-key", "new-secret-value-67890", &*session)
        .await?;
    println!("   ✓ Secret updated");

    let updated_value = backend.get_notes("api-key", &*session).await?;
    println!("   ✓ New value: {}", updated_value);

    // Check if secret exists
    println!("\n7. Checking if secret exists...");
    let exists = backend.item_exists("api-key", &*session).await?;
    println!("   ✓ Secret exists: {}", exists);

    // Delete the secret
    println!("\n8. Deleting secret 'api-key'...");
    backend.delete_item("api-key", &*session).await?;
    println!("   ✓ Secret deleted");

    // Verify deletion
    let exists_after = backend.item_exists("api-key", &*session).await?;
    println!("   ✓ Secret exists after deletion: {}", exists_after);

    println!("\n=== Example Complete ===");
    Ok(())
}
