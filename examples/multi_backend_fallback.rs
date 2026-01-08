//! Multi-backend fallback example.
//!
//! Demonstrates trying multiple backends until one succeeds.
//! Useful for applications that can work with different secret managers.
//!
//! Run with: cargo run --example multi_backend_fallback --features "pass,bitwarden"

use vaultmux::{factory, Backend, Config, BackendType, VaultmuxError};

async fn try_backend(backend_type: BackendType) -> vaultmux::Result<Box<dyn Backend>> {
    println!("Trying backend: {:?}...", backend_type);
    
    let config = Config::new(backend_type);
    let mut backend = factory::new_backend(config)?;
    
    // Try to initialize
    backend.init().await?;
    
    // Try to authenticate
    let session = backend.authenticate().await?;
    
    // Test if we can actually use it
    let _ = backend.list_items(&*session).await?;
    
    println!("✓ Backend {:?} is available and working!", backend_type);
    Ok(backend)
}

#[tokio::main]
async fn main() -> vaultmux::Result<()> {
    println!("=== Multi-Backend Fallback Example ===\n");
    
    // List of backends to try, in order of preference
    let backends_to_try = vec![
        BackendType::Pass,
        BackendType::Bitwarden,
        BackendType::OnePassword,
        BackendType::Pass, // Fallback to mock for testing
    ];
    
    let mut selected_backend: Option<Box<dyn Backend>> = None;
    
    for backend_type in backends_to_try {
        match try_backend(backend_type).await {
            Ok(backend) => {
                selected_backend = Some(backend);
                break;
            }
            Err(e) => {
                println!("✗ Backend {:?} unavailable: {}", backend_type, e);
                continue;
            }
        }
    }
    
    let mut backend = selected_backend.ok_or_else(|| {
        VaultmuxError::Other(anyhow::anyhow!("No available backends found"))
    })?;
    
    println!("\n=== Using backend: {} ===\n", backend.name());
    
    // Now use the selected backend
    let session = backend.authenticate().await?;
    
    // Example operation: List all secrets
    println!("Listing secrets...");
    let items = backend.list_items(&*session).await?;
    println!("Found {} secret(s)", items.len());
    
    for item in items.iter().take(5) {
        println!("  - {}", item.name);
    }
    
    if items.len() > 5 {
        println!("  ... and {} more", items.len() - 5);
    }
    
    println!("\n=== Example Complete ===");
    Ok(())
}
