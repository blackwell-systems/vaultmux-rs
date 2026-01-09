# Vaultmux

> **Write once, run anywhere - unified secret management for Rust**

[![Blackwell Systems™](https://raw.githubusercontent.com/blackwell-systems/blackwell-docs-theme/main/badge-trademark.svg)](https://github.com/blackwell-systems)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.88+-blue.svg)](https://www.rust-lang.org/)

One async interface for 8 secret backends. Switch from pass to AWS Secrets Manager without changing your code.

```rust
let config = Config::new(BackendType::Pass).with_prefix("myapp");
let mut backend = factory::new_backend(config)?;
```

Change one line, support any vault.

## Why

Your application needs secrets. But which vault?

- **Developers** want pass or Bitwarden
- **CI/CD** needs cloud vaults (AWS, GCP, Azure)
- **Windows servers** need Credential Manager
- **Tests** need a mock

Without vaultmux, you write 8 different integrations. With vaultmux, you write one.

Rust port of the [Go vaultmux library](https://github.com/blackwell-systems/vaultmux).

## Features

- **Unified API** - Single `Backend` trait works with any vault
- **Async/Await** - Built on tokio for non-blocking I/O
- **Type Safety** - Leverage Rust's type system for compile-time guarantees
- **Session Caching** - Avoid repeated authentication with disk-based caching
- **Error Context** - Rich error types with full context and chaining
- **Feature Flags** - Optional backend compilation to minimize dependencies
- **Testable** - Includes mock backend for unit testing

## Supported Backends

| Backend | Feature Flag | CLI Required | Platform | Status |
|---------|-------------|--------------|----------|--------|
| **Mock** | `mock` (default) | None | All | Implemented |
| **pass** | `pass` | `pass`, `gpg` | Unix | Implemented |
| **Bitwarden** | `bitwarden` | `bw` | All | Implemented |
| **1Password** | `onepassword` | `op` | All | Implemented |
| **AWS Secrets Manager** | `aws` | None (SDK) | All | Implemented |
| **GCP Secret Manager** | `gcp` | None (SDK) | All | Implemented |
| **Azure Key Vault** | `azure` | None (SDK) | All | Implemented |
| **Windows Credential Manager** | `wincred` | PowerShell | Windows | Implemented |

## Installation

```toml
[dependencies]
vaultmux = { version = "0.1", features = ["bitwarden", "aws"] }
```

Available features: `mock`, `pass`, `bitwarden`, `onepassword`, `aws`, `gcp`, `azure`, `wincred`, or `full` for all backends.

## Quick Start

```rust
use vaultmux::{factory, Backend, Config, BackendType};

#[tokio::main]
async fn main() -> vaultmux::Result<()> {
    // Create backend
    let config = Config::new(BackendType::Pass)
        .with_prefix("myapp");

    let mut backend = factory::new_backend(config)?;
    backend.init().await?;

    // Authenticate
    let session = backend.authenticate().await?;

    // Store a secret
    backend.create_item("api-key", "secret-value", &*session).await?;

    // Retrieve it
    let secret = backend.get_notes("api-key", &*session).await?;
    println!("Secret: {}", secret);

    Ok(())
}
```

## Testing

Use the mock backend to test without real vaults:

```rust
use vaultmux::backends::mock::MockBackend;

#[tokio::test]
async fn test_my_code() {
    let mut backend = MockBackend::new();
    backend.set_item("test-key", "test-value").await;
    // Test your code...
}
```

Run tests: `cargo test --all-features`

See [User Guide](docs/user-guide.md#testing) for integration testing with LocalStack.

## Documentation

- **[User Guide](docs/user-guide.md)** - Setup, patterns, best practices
- **[API Reference](docs/api-reference.md)** - Complete API documentation
- **[Examples](examples/)** - Real-world usage patterns

Or generate docs locally:
```bash
cargo doc --open
```

## Comparison to Go Version

| Feature | Go | Rust |
|---------|-----|------|
| **Type Safety** | Runtime | Compile-time |
| **Memory Safety** | GC | Ownership system |
| **Concurrency** | Goroutines | async/await + tokio |
| **Error Handling** | `(T, error)` | `Result<T, E>` |
| **Dependencies** | Modules | Crates + features |

## Contributing

Contributions welcome! Current priorities:

1. Add integration tests for all backends
2. Create comprehensive usage examples
3. Improve documentation and API docs
4. Set up CI/CD pipeline
5. Add performance benchmarks

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Credits

Rust port by Dayna Blackwell (Blackwell Systems™).  
Original Go implementation: https://github.com/blackwell-systems/vaultmux
