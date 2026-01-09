# Vaultmux v0.1.0 - Initial Stable Release

**Write once, run anywhere** - Unified async interface for password managers and cloud secret vaults.

## What is Vaultmux?

Vaultmux provides one Rust API that works with 8 different secret backends. Switch from pass to AWS Secrets Manager by changing one line of configuration.

```rust
// Works with any backend - pass, Bitwarden, AWS, GCP, Azure, etc.
let config = Config::new(BackendType::Pass).with_prefix("myapp");
let mut backend = factory::new_backend(config)?;
let session = backend.authenticate().await?;
backend.create_item("api-key", "value", &*session).await?;
```

## Supported Backends

- **CLI:** pass, Bitwarden, 1Password
- **Cloud:** AWS Secrets Manager, GCP Secret Manager, Azure Key Vault
- **Platform:** Windows Credential Manager
- **Testing:** Mock backend

## Key Features

+ **Unified API** - Single `Backend` trait for all vaults
+ **Async/await** - Built on tokio for non-blocking I/O
+ **Type safety** - Rust enums prevent typos and bugs
+ **Session caching** - Secure disk-based caching (0600 permissions)
+ **Feature flags** - Compile only what you need
+ **Input validation** - Command injection prevention
+ **Testable** - Mock backend for unit tests
+ **Cross-platform** - Linux, macOS, Windows

## What's Included

- 75+ tests (unit + integration + doc tests) - all passing
- AWS integration tests with LocalStack
- 6 runnable examples with real-world patterns
- Comprehensive user guide (docs/user-guide.md)
- Complete API reference (docs/api-reference.md)
- GitHub Actions CI/CD (4 workflows)
- Security policy and audit documentation

## Installation

\`\`\`toml
[dependencies]
vaultmux = { version = "0.1", features = ["bitwarden", "aws"] }
\`\`\`

Available features: \`mock\`, \`pass\`, \`bitwarden\`, \`onepassword\`, \`aws\`, \`gcp\`, \`azure\`, \`wincred\`, or \`full\` for all backends.

## Documentation

- **[User Guide](https://github.com/blackwell-systems/vaultmux-rs/blob/main/docs/user-guide.md)** - Setup, patterns, best practices
- **[API Reference](https://github.com/blackwell-systems/vaultmux-rs/blob/main/docs/api-reference.md)** - Complete API documentation
- **[Examples](https://github.com/blackwell-systems/vaultmux-rs/tree/main/examples)** - Real-world usage patterns
- **[docs.rs](https://docs.rs/vaultmux)** - Generated documentation

## Requirements

- Rust 1.88.0 or later (MSRV)
- Tokio async runtime
- Backend-specific requirements (CLI tools, cloud credentials, etc.)

## License

MIT OR Apache-2.0
