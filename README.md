# Vaultmux (Rust Port)

> **Unified interface for multi-vault secret management in Rust**

[![Blackwell Systems™](https://raw.githubusercontent.com/blackwell-systems/blackwell-docs-theme/main/badge-trademark.svg)](https://github.com/blackwell-systems)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.75+-blue.svg)](https://www.rust-lang.org/)

Vaultmux provides a unified async interface for interacting with multiple secret management systems. Write your code once and support Bitwarden, 1Password, pass, Windows Credential Manager, AWS Secrets Manager, Google Cloud Secret Manager, and Azure Key Vault with the same API.

This is a Rust port of the [Go vaultmux library](https://github.com/blackwell-systems/vaultmux).

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
| **Mock** | `mock` (default) | None | All | ✅ Implemented |
| **Bitwarden** | `bitwarden` | `bw` | All | 📋 Planned |
| **1Password** | `onepassword` | `op` | All | 📋 Planned |
| **pass** | `pass` | `pass`, `gpg` | Unix | 📋 Planned |
| **Windows Credential Manager** | `wincred` | PowerShell | Windows | 📋 Planned |
| **AWS Secrets Manager** | `aws` | None (SDK) | All | 📋 Planned |
| **GCP Secret Manager** | `gcp` | None (SDK) | All | 📋 Planned |
| **Azure Key Vault** | `azure` | None (SDK) | All | 📋 Planned |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
vaultmux = "0.1"
```

Or with specific backends:

```toml
[dependencies]
vaultmux = { version = "0.1", features = ["bitwarden", "aws"] }
```

Or enable all backends:

```toml
[dependencies]
vaultmux = { version = "0.1", features = ["full"] }
```

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

## Current Status

**Phase 1: Foundation** ✅ Complete (Week 1)
- ✅ Core traits (`Backend`, `Session`)
- ✅ Data types (`Item`, `ItemType`, `Config`)
- ✅ Error system with `thiserror`
- ✅ Factory pattern + registration
- ✅ Session caching (disk-based with 0600 permissions)
- ✅ Input validation (command injection prevention)
- ✅ Mock backend + comprehensive tests (41 tests passing)

**Phase 2: CLI Backends** 📋 Planned (Week 2)
- Bitwarden (`bw` CLI)
- 1Password (`op` CLI)
- pass (`pass` + `gpg`)

**Phase 3: Cloud Backends** 📋 Planned (Week 3)
- AWS Secrets Manager
- GCP Secret Manager
- Azure Key Vault

**Phase 4: Platform-Specific** 📋 Planned (Week 4)
- Windows Credential Manager
- Documentation
- Examples
- CI/CD

## Testing

The mock backend allows easy testing:

```rust
use vaultmux::backends::mock::MockBackend;
use vaultmux::{Backend, VaultmuxError};

#[tokio::test]
async fn test_my_code() {
    let mut backend = MockBackend::new();
    backend.set_item("test-key", "test-value").await;

    // Test error conditions
    backend.get_error = Some(VaultmuxError::PermissionDenied("test".into()));

    // Test your code...
}
```

Run tests:

```bash
cargo test
```

Run tests with output:

```bash
cargo test -- --nocapture
```

## Architecture

See [`RUST_PORT_PLAN.md`](RUST_PORT_PLAN.md) for the complete architectural design and implementation plan.

**Key Design Principles:**
1. **Async by default** - All I/O operations use `tokio`
2. **Type safety** - Rust enums instead of string constants
3. **Memory safety** - Zero data races via Rust's ownership system
4. **Feature flags** - Optional compilation of backends
5. **Error context** - Rich error types with full chaining

## Documentation

Generate and view documentation:

```bash
cargo doc --open
```

## API Reference

### Core Traits

- **`Backend`** - Main vault interface (15 methods)
- **`Session`** - Authentication session (4 methods)

### Data Types

- **`Item`** - Vault item with metadata
- **`ItemType`** - Item type enum (SecureNote, Login, SSHKey, etc.)
- **`Config`** - Backend configuration with builder pattern

### Errors

All errors are variants of `VaultmuxError`:
- `NotFound` - Item doesn't exist
- `AlreadyExists` - Item already exists
- `NotAuthenticated` - Not authenticated
- `SessionExpired` - Session has expired
- `BackendNotInstalled` - CLI tool missing
- `InvalidItemName` - Invalid item name (prevents injection)
- And more...

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

1. Implement Bitwarden backend
2. Implement 1Password backend
3. Implement pass backend
4. Add integration tests
5. Improve documentation

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Credits

Rust port by Dayna Blackwell (Blackwell Systems™).  
Original Go implementation: https://github.com/blackwell-systems/vaultmux
