//! Basic usage example with the mock backend.

use vaultmux::{factory, Backend, Config};

#[tokio::main]
async fn main() -> vaultmux::Result<()> {
    // Initialize the library (registers backends)
    vaultmux::init();

    // Create a mock backend configuration
    // Note: We can't use BackendType::Pass here because no backends
    // are actually registered yet in this minimal implementation
    println!("Creating mock backend...");
    
    // For now, demonstrate the API pattern
    let config = Config::default();
    println!("Config: backend = {}", config.backend);
    println!("        prefix  = {}", config.prefix);
    println!("        ttl     = {:?}", config.session_ttl);

    // Uncomment once mock backend is registered:
    /*
    let mut backend = factory::new_backend(config)?;
    println!("Backend initialized: {}", backend.name());

    // Initialize backend
    backend.init().await?;
    println!("Backend ready");

    // Authenticate
    let session = backend.authenticate().await?;
    println!("Authenticated with session token: {}", session.token());

    // Create an item
    backend.create_item("example-key", "example-secret-value", &*session).await?;
    println!("Created item: example-key");

    // Retrieve the item
    let secret = backend.get_notes("example-key", &*session).await?;
    println!("Retrieved secret: {}", secret);

    // List all items
    let items = backend.list_items(&*session).await?;
    println!("\nAll items ({}):", items.len());
    for item in items {
        println!("  - {} (type: {})", item.name, item.item_type);
    }

    // Update the item
    backend.update_item("example-key", "updated-secret-value", &*session).await?;
    println!("\nUpdated item: example-key");

    let updated = backend.get_notes("example-key", &*session).await?;
    println!("New value: {}", updated);

    // Delete the item
    backend.delete_item("example-key", &*session).await?;
    println!("\nDeleted item: example-key");

    // Verify deletion
    let exists = backend.item_exists("example-key", &*session).await?;
    println!("Item exists: {}", exists);
    */

    println!("\n✓ Example completed successfully!");
    println!("\nNote: This is a skeleton example. The mock backend will be");
    println!("fully functional once we complete the backend registration.");

    Ok(())
}
