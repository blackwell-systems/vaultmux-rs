//! AWS Secrets Manager integration tests using LocalStack.
//!
//! These tests require LocalStack to be running on localhost:4566.
//!
//! Run with:
//!   docker run -d -p 4566:4566 localstack/localstack
//!   cargo test --test integration_aws --features aws
//!
//! Or run in CI where LocalStack is configured as a service.

#![cfg(feature = "aws")]

use vaultmux::{factory, Backend, BackendType, Config, VaultmuxError};

// Initialize vaultmux library to register backends
fn init_library() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        vaultmux::init();
    });
}

fn aws_config() -> Config {
    let endpoint = std::env::var("LOCALSTACK_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4566".to_string());

    Config::new(BackendType::AWSSecretsManager)
        .with_option("region", "us-east-1")
        .with_option("endpoint", endpoint)
        .with_prefix("test-")
}

async fn setup_backend() -> (Box<dyn Backend>, std::sync::Arc<dyn vaultmux::Session>) {
    // Initialize library to register backends
    init_library();

    std::env::set_var("AWS_ACCESS_KEY_ID", "test");
    std::env::set_var("AWS_SECRET_ACCESS_KEY", "test");
    std::env::set_var("AWS_REGION", "us-east-1");

    let config = aws_config();
    let mut backend = factory::new_backend(config).expect("Failed to create backend");

    backend.init().await.expect("Failed to initialize backend");
    let session = backend
        .authenticate()
        .await
        .expect("Failed to authenticate");

    (backend, session)
}

#[tokio::test]
#[ignore] // Run only when LocalStack is available
async fn test_aws_create_and_get() {
    let (mut backend, session) = setup_backend().await;

    let secret_name = "test-secret-1";
    let secret_value = "my-secret-value";

    // Create secret
    backend
        .create_item(secret_name, secret_value, &*session)
        .await
        .expect("Failed to create secret");

    // Retrieve secret
    let retrieved = backend
        .get_notes(secret_name, &*session)
        .await
        .expect("Failed to get secret");

    assert_eq!(retrieved, secret_value);

    // Clean up
    backend.delete_item(secret_name, &*session).await.ok();
}

#[tokio::test]
#[ignore]
async fn test_aws_update() {
    let (mut backend, session) = setup_backend().await;

    let secret_name = "test-secret-2";
    let initial_value = "initial-value";
    let updated_value = "updated-value";

    // Create
    backend
        .create_item(secret_name, initial_value, &*session)
        .await
        .expect("Failed to create secret");

    // Update
    backend
        .update_item(secret_name, updated_value, &*session)
        .await
        .expect("Failed to update secret");

    // Verify update
    let retrieved = backend
        .get_notes(secret_name, &*session)
        .await
        .expect("Failed to get secret");

    assert_eq!(retrieved, updated_value);

    // Clean up
    backend.delete_item(secret_name, &*session).await.ok();
}

#[tokio::test]
#[ignore]
async fn test_aws_delete() {
    let (mut backend, session) = setup_backend().await;

    let secret_name = "test-secret-3";

    // Create
    backend
        .create_item(secret_name, "value", &*session)
        .await
        .expect("Failed to create secret");

    // Verify exists
    let exists_before = backend
        .item_exists(secret_name, &*session)
        .await
        .expect("Failed to check existence");
    assert!(exists_before);

    // Delete
    backend
        .delete_item(secret_name, &*session)
        .await
        .expect("Failed to delete secret");

    // Verify deleted
    let exists_after = backend
        .item_exists(secret_name, &*session)
        .await
        .expect("Failed to check existence");
    assert!(!exists_after);
}

#[tokio::test]
#[ignore]
async fn test_aws_list() {
    let (mut backend, session) = setup_backend().await;

    // Create multiple secrets
    let secrets = vec![
        ("test-list-1", "value1"),
        ("test-list-2", "value2"),
        ("test-list-3", "value3"),
    ];

    for (name, value) in &secrets {
        backend
            .create_item(name, value, &*session)
            .await
            .expect("Failed to create secret");
    }

    // List secrets
    let items = backend
        .list_items(&*session)
        .await
        .expect("Failed to list secrets");

    // Should find all our test secrets
    let test_items: Vec<_> = items
        .iter()
        .filter(|item| item.name.starts_with("test-list-"))
        .collect();

    assert!(
        test_items.len() >= 3,
        "Expected at least 3 test secrets, found {}",
        test_items.len()
    );

    // Clean up
    for (name, _) in &secrets {
        backend.delete_item(name, &*session).await.ok();
    }
}

#[tokio::test]
#[ignore]
async fn test_aws_already_exists_error() {
    let (mut backend, session) = setup_backend().await;

    let secret_name = "test-duplicate";

    // Create first time
    backend
        .create_item(secret_name, "value1", &*session)
        .await
        .expect("Failed to create secret");

    // Try to create again - should fail
    let result = backend.create_item(secret_name, "value2", &*session).await;

    assert!(matches!(result, Err(VaultmuxError::AlreadyExists(_))));

    // Clean up
    backend.delete_item(secret_name, &*session).await.ok();
}

#[tokio::test]
#[ignore]
async fn test_aws_not_found_error() {
    let (backend, session) = setup_backend().await;

    let result = backend.get_notes("nonexistent-secret", &*session).await;

    assert!(matches!(result, Err(VaultmuxError::NotFound(_))));
}

#[tokio::test]
#[ignore]
async fn test_aws_get_item_with_metadata() {
    let (mut backend, session) = setup_backend().await;

    let secret_name = "test-metadata";
    let secret_value = "value-with-metadata";

    // Create secret
    backend
        .create_item(secret_name, secret_value, &*session)
        .await
        .expect("Failed to create secret");

    // Get full item
    let item = backend
        .get_item(secret_name, &*session)
        .await
        .expect("Failed to get item");

    assert_eq!(item.name, secret_name);
    assert_eq!(item.notes, Some(secret_value.to_string()));
    assert!(item.id.contains("arn:aws:secretsmanager") || item.id.contains(secret_name));

    // Timestamps should be present (LocalStack may not set them)
    // assert!(item.created.is_some());
    // assert!(item.modified.is_some());

    // Clean up
    backend.delete_item(secret_name, &*session).await.ok();
}

#[tokio::test]
#[ignore]
async fn test_aws_prefix_isolation() {
    let (mut backend, session) = setup_backend().await;

    // Our backend has "test-" prefix
    // Create a secret
    backend
        .create_item("isolated", "value", &*session)
        .await
        .expect("Failed to create secret");

    // List should only show items with our prefix
    let items = backend
        .list_items(&*session)
        .await
        .expect("Failed to list items");

    // All items should have names without the prefix (stripped)
    for item in &items {
        assert!(
            !item.name.starts_with("test-"),
            "Prefix should be stripped from item names"
        );
    }

    // Clean up
    backend.delete_item("isolated", &*session).await.ok();
}
