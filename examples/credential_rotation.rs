//! Credential rotation example.
//!
//! Demonstrates a common security practice: automatically rotating secrets.
//! This example shows how to update secrets and maintain multiple versions.
//!
//! Run with: cargo run --example credential_rotation

use chrono::Utc;
use vaultmux::{factory, Backend, Config, BackendType};

async fn rotate_secret(
    backend: &mut dyn Backend,
    secret_name: &str,
    new_value: &str,
    session: &dyn vaultmux::Session,
) -> vaultmux::Result<()> {
    println!("Rotating secret '{}'...", secret_name);
    
    // Check if secret exists
    if backend.item_exists(secret_name, session).await? {
        // Update existing secret
        backend.update_item(secret_name, new_value, session).await?;
        println!("✓ Secret updated");
    } else {
        // Create new secret
        backend.create_item(secret_name, new_value, session).await?;
        println!("✓ Secret created");
    }
    
    Ok(())
}

fn generate_api_key() -> String {
    use uuid::Uuid;
    format!("api_key_{}", Uuid::new_v4())
}

#[tokio::main]
async fn main() -> vaultmux::Result<()> {
    println!("=== Credential Rotation Example ===\n");
    
    // Use mock backend for demonstration
    let config = Config::new(BackendType::Pass);
    let mut backend = factory::new_backend(config)?;
    
    backend.init().await?;
    let session = backend.authenticate().await?;
    
    // Initial secret creation
    let secret_name = "api-key";
    let initial_key = generate_api_key();
    
    println!("1. Creating initial API key...");
    backend.create_item(secret_name, &initial_key, &*session).await?;
    println!("   ✓ Initial key: {}", initial_key);
    
    // Simulate usage period
    println!("\n2. Simulating 30-day usage period...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // Rotation #1
    println!("\n3. Rotating key after 30 days...");
    let rotated_key_1 = generate_api_key();
    rotate_secret(&mut *backend, secret_name, &rotated_key_1, &*session).await?;
    println!("   ✓ New key: {}", rotated_key_1);
    
    // Verify rotation
    let current = backend.get_notes(secret_name, &*session).await?;
    assert_eq!(current, rotated_key_1);
    println!("   ✓ Rotation verified");
    
    // Rotation #2
    println!("\n4. Rotating key again after another 30 days...");
    let rotated_key_2 = generate_api_key();
    rotate_secret(&mut *backend, secret_name, &rotated_key_2, &*session).await?;
    println!("   ✓ New key: {}", rotated_key_2);
    
    // Best practice: Store rotation metadata
    let rotation_metadata = format!(
        "Rotated at: {}\nPrevious key: {} (first 8 chars)",
        Utc::now().to_rfc3339(),
        &rotated_key_1[..8]
    );
    backend
        .create_item("api-key-rotation-log", &rotation_metadata, &*session)
        .await
        .ok(); // Ignore if exists
    
    println!("\n5. Rotation history saved");
    
    // List all secrets
    println!("\n6. Current secrets:");
    let items = backend.list_items(&*session).await?;
    for item in &items {
        println!("   - {}", item.name);
    }
    
    println!("\n=== Best Practices for Credential Rotation ===");
    println!("• Rotate credentials every 30-90 days");
    println!("• Keep audit log of rotations");
    println!("• Use automated rotation schedules");
    println!("• Notify dependent services after rotation");
    println!("• Maintain grace period for old credentials");
    
    println!("\n=== Example Complete ===");
    Ok(())
}
