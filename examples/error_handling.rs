//! Error handling example.
//!
//! Demonstrates proper error handling patterns with vaultmux.
//!
//! Run with: cargo run --example error_handling

use vaultmux::{factory, Backend, BackendType, Config, VaultmuxError};

#[tokio::main]
async fn main() -> vaultmux::Result<()> {
    println!("=== Error Handling Example ===\n");

    let config = Config::new(BackendType::Pass);
    let mut backend = factory::new_backend(config)?;
    backend.init().await?;
    let session = backend.authenticate().await?;

    // Example 1: NotFound error
    println!("1. Handling NotFound error:");
    match backend.get_notes("nonexistent-key", &*session).await {
        Ok(value) => println!("   Found: {}", value),
        Err(VaultmuxError::NotFound(name)) => {
            println!("   ✓ Secret '{}' not found (expected)", name);
        }
        Err(e) => println!("   Unexpected error: {}", e),
    }

    // Example 2: AlreadyExists error
    println!("\n2. Handling AlreadyExists error:");
    let secret_name = "existing-key";
    backend
        .create_item(secret_name, "value1", &*session)
        .await?;

    match backend.create_item(secret_name, "value2", &*session).await {
        Ok(_) => println!("   Created"),
        Err(VaultmuxError::AlreadyExists(name)) => {
            println!("   ✓ Secret '{}' already exists (expected)", name);
            println!("   → Use update_item() to modify existing secrets");
        }
        Err(e) => println!("   Unexpected error: {}", e),
    }

    // Example 3: Idempotent operations using error handling
    println!("\n3. Idempotent create-or-update:");
    let result = create_or_update(&mut *backend, "config-key", "config-value", &*session).await;
    println!("   ✓ {}", result?);

    // Example 4: Graceful degradation
    println!("\n4. Graceful degradation with fallback:");
    let value =
        get_with_fallback(&*backend, "maybe-missing-key", "default-value", &*session).await?;
    println!("   ✓ Got value: {}", value);

    // Example 5: Error context
    println!("\n5. Error with context:");
    match delete_with_context(&mut *backend, "nonexistent", &*session).await {
        Ok(_) => println!("   Deleted"),
        Err(e) => {
            println!("   ✓ Error with context: {}", e);
            println!("   → Full error chain available for debugging");
        }
    }

    // Example 6: Retry logic
    println!("\n6. Retry logic for transient errors:");
    retry_operation(&*backend, "retry-key", &*session, 3).await?;
    println!("   ✓ Operation succeeded (with or without retries)");

    println!("\n=== Error Handling Best Practices ===");
    println!("• Match on specific error variants for precise handling");
    println!("• Use create_or_update pattern for idempotency");
    println!("• Provide fallback values for non-critical secrets");
    println!("• Add context to errors for better debugging");
    println!("• Implement retry logic for transient failures");
    println!("• Log errors but don't expose sensitive details to users");

    println!("\n=== Example Complete ===");
    Ok(())
}

/// Create a secret if it doesn't exist, update if it does
async fn create_or_update(
    backend: &mut dyn Backend,
    name: &str,
    value: &str,
    session: &dyn vaultmux::Session,
) -> vaultmux::Result<String> {
    match backend.create_item(name, value, session).await {
        Ok(_) => Ok(format!("Created secret '{}'", name)),
        Err(VaultmuxError::AlreadyExists(_)) => {
            backend.update_item(name, value, session).await?;
            Ok(format!("Updated existing secret '{}'", name))
        }
        Err(e) => Err(e),
    }
}

/// Get a secret with a fallback value if not found
async fn get_with_fallback(
    backend: &dyn Backend,
    name: &str,
    fallback: &str,
    session: &dyn vaultmux::Session,
) -> vaultmux::Result<String> {
    match backend.get_notes(name, session).await {
        Ok(value) => Ok(value),
        Err(VaultmuxError::NotFound(_)) => {
            println!("   → Secret not found, using fallback");
            Ok(fallback.to_string())
        }
        Err(e) => Err(e),
    }
}

/// Delete with additional error context
async fn delete_with_context(
    backend: &mut dyn Backend,
    name: &str,
    session: &dyn vaultmux::Session,
) -> vaultmux::Result<()> {
    backend.delete_item(name, session).await.map_err(|e| {
        VaultmuxError::Other(anyhow::anyhow!("Failed to delete secret '{}': {}", name, e))
    })
}

/// Retry an operation up to max_retries times
async fn retry_operation(
    backend: &dyn Backend,
    name: &str,
    session: &dyn vaultmux::Session,
    max_retries: u32,
) -> vaultmux::Result<bool> {
    let mut attempts = 0;

    loop {
        attempts += 1;

        match backend.item_exists(name, session).await {
            Ok(exists) => {
                if attempts > 1 {
                    println!("   → Succeeded after {} attempt(s)", attempts);
                }
                return Ok(exists);
            }
            Err(e) if attempts < max_retries => {
                println!("   → Attempt {} failed: {}", attempts, e);
                println!("   → Retrying...");
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                continue;
            }
            Err(e) => {
                return Err(VaultmuxError::Other(anyhow::anyhow!(
                    "Operation failed after {} attempts: {}",
                    attempts,
                    e
                )));
            }
        }
    }
}
