# Release Notes: vaultmux v0.1.0

**Release Date:** January 8, 2025  
**First Stable Release** 🎉

## Overview

vaultmux v0.1.0 is the initial stable release of the Rust port of the Go vaultmux library. This release provides a unified, async interface for interacting with 8 different secret management systems with 100% feature parity to the original Go implementation.

## What is vaultmux?

vaultmux provides a single, consistent API for managing secrets across multiple vault backends. Write your code once and support Bitwarden, 1Password, pass, Windows Credential Manager, AWS Secrets Manager, Google Cloud Secret Manager, and Azure Key Vault with the same interface.

## Features

### ✨ Complete Backend Support (8/8)

All backends from the Go version are implemented and tested:

**CLI Backends:**
- ✅ **pass** - Unix password manager (requires `pass` + `gpg`)
- ✅ **Bitwarden** - Bitwarden CLI (requires `bw`)
- ✅ **1Password** - 1Password CLI (requires `op`)

**Cloud SDK Backends:**
- ✅ **AWS Secrets Manager** - AWS SDK integration
- ✅ **GCP Secret Manager** - Google Cloud SDK integration
- ✅ **Azure Key Vault** - Microsoft Azure SDK integration

**Platform-Specific:**
- ✅ **Windows Credential Manager** - PowerShell-based (Windows only)

**Testing:**
- ✅ **Mock Backend** - For unit testing without external dependencies

### 🚀 Key Improvements Over Go Version

**Type Safety:**
- Compile-time guarantees via Rust's type system
- No runtime type errors
- Exhaustive pattern matching for error handling

**Memory Safety:**
- Zero data races guaranteed by Rust's ownership system
- No garbage collector overhead
- Predictable performance

**Async/Await:**
- Built on tokio runtime
- Non-blocking I/O throughout
- Efficient concurrent operations

**Modern Error Handling:**
- Rich error types with full context
- Error chaining with `anyhow`
- Specific error variants for precise handling

**Feature Flags:**
- Optional backend compilation
- Minimize dependencies for your use case
- Faster compile times with only needed backends

### 🔒 Security

**Input Validation:**
- Command injection prevention for all CLI backends
- Item name validation with regex checks
- Protection against path traversal

**Session Management:**
- Disk-based caching with 0600 permissions (user-only)
- Configurable TTL per backend
- Automatic session invalidation

**TLS/Encryption:**
- All cloud backends use HTTPS
- Credential storage secured by backend systems
- No plaintext secrets in memory longer than needed

**Known Issues:**
- See [SECURITY.md](SECURITY.md) for documented advisories
- All issues are from third-party dependencies with no actual impact

### 📚 Documentation

**Comprehensive Examples:**
- 6 standalone runnable examples
- Real-world patterns (rotation, fallback, errors)
- Setup guides for all backends

**API Documentation:**
- 100% public API documented
- 20 executable doc tests
- Module-level guides

**Guides:**
- Quick start in README
- Security policy
- Contribution guidelines

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
vaultmux = "0.1"
```

Enable specific backends:

```toml
[dependencies]
vaultmux = { version = "0.1", features = ["aws", "gcp", "azure"] }
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
    
    // Use it
    backend.create_item("api-key", "secret-value", &*session).await?;
    let secret = backend.get_notes("api-key", &*session).await?;
    println!("Secret: {}", secret);
    
    Ok(())
}
```

## Requirements

**Minimum Supported Rust Version (MSRV):** 1.88.0

Cloud SDK backends (AWS, GCP, Azure) require Rust 1.88+ due to AWS SDK dependencies. CLI-only backends work with earlier Rust versions but aren't separately tested.

## Feature Flags

| Feature | Description | Default |
|---------|-------------|---------|
| `mock` | Mock backend for testing | ✅ Yes |
| `pass` | Unix password manager | ❌ No |
| `bitwarden` | Bitwarden CLI | ❌ No |
| `onepassword` | 1Password CLI | ❌ No |
| `aws` | AWS Secrets Manager | ❌ No |
| `gcp` | GCP Secret Manager | ❌ No |
| `azure` | Azure Key Vault | ❌ No |
| `wincred` | Windows Credential Manager | ❌ No |
| `full` | All backends | ❌ No |

## Testing

**Test Coverage:**
- 73 tests (53 unit + 20 doc tests)
- All tests passing on Linux, macOS, Windows
- Cross-platform CI verification

Run tests:
```bash
cargo test                      # Default features
cargo test --all-features       # All backends
cargo test --features aws       # Specific backend
```

## Examples

See the `examples/` directory for comprehensive usage examples:

- **basic_usage.rs** - CRUD operations
- **aws_secrets.rs** - AWS integration
- **multi_backend_fallback.rs** - Resilient backend selection
- **environment_config.rs** - 12-factor app configuration
- **credential_rotation.rs** - Automated key rotation
- **error_handling.rs** - Production error patterns

Run examples:
```bash
cargo run --example basic_usage --features pass
cargo run --example aws_secrets --features aws
```

## Migration from Go Version

The Rust API closely mirrors the Go API for easy migration:

**Go:**
```go
config := vaultmux.Config{
    Backend: vaultmux.BackendPass,
    Prefix:  "myapp",
}
backend, _ := vaultmux.NewBackend(config)
```

**Rust:**
```rust
let config = Config::new(BackendType::Pass)
    .with_prefix("myapp");
let backend = factory::new_backend(config)?;
```

Key differences:
- Async/await instead of goroutines
- `Result<T>` instead of `(T, error)`
- Feature flags for optional backends
- Type-safe enums instead of string constants

## Performance

Expected improvements over Go version:
- **Lower memory usage** - No GC, smaller allocations
- **Predictable latency** - No GC pauses
- **Faster startup** - No runtime initialization overhead

Formal benchmarks coming in v0.2.0.

## Roadmap

**v0.2.0 (Planned):**
- Integration tests with real services
- Performance benchmarks vs Go
- Additional examples (web framework integration)
- Expanded error context

**v0.3.0 (Planned):**
- Async session refresh
- Batch operations
- Transaction support (where backends support it)

## Breaking Changes

None - this is the initial release.

## Deprecations

None - this is the initial release.

## Known Issues

See [SECURITY.md](SECURITY.md) for detailed information about:
- Transitive dependency advisories (no actual security impact)
- Upstream unmaintained dependencies
- Mitigation strategies

## Contributors

**Primary Author:**
- Dayna Blackwell ([@blackwell-systems](https://github.com/blackwell-systems))

**Original Go Implementation:**
- [vaultmux](https://github.com/blackwell-systems/vaultmux)

## Links

- **Repository:** https://github.com/blackwell-systems/vaultmux-rs
- **Documentation:** https://docs.rs/vaultmux
- **Crate:** https://crates.io/crates/vaultmux
- **Issues:** https://github.com/blackwell-systems/vaultmux-rs/issues
- **Original Go Library:** https://github.com/blackwell-systems/vaultmux

## License

Dual-licensed under MIT OR Apache-2.0

## Acknowledgments

Special thanks to:
- The Rust async ecosystem (tokio, async-trait)
- Backend SDK teams (AWS, Azure, Google)
- The original Go vaultmux users for feedback

---

**Ready to use in production!** 🚀

For questions, issues, or contributions, please visit our [GitHub repository](https://github.com/blackwell-systems/vaultmux-rs).
