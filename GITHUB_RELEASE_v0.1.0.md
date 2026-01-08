## 🎉 vaultmux v0.1.0 - Initial Stable Release

Rust port of the Go vaultmux library with 100% feature parity.

### ✨ What's New

**Unified secret management interface for 8 vault backends:**
- CLI: pass, Bitwarden, 1Password
- Cloud: AWS Secrets Manager, GCP Secret Manager, Azure Key Vault
- Platform: Windows Credential Manager
- Testing: Mock backend

### 🚀 Key Features

✅ **Type Safety** - Compile-time guarantees via Rust's type system  
✅ **Memory Safety** - Zero data races, no GC overhead  
✅ **Async/Await** - Non-blocking I/O with tokio  
✅ **Feature Flags** - Optional backend compilation  
✅ **Session Caching** - Disk-based with secure permissions  
✅ **Input Validation** - Command injection prevention  
✅ **Rich Errors** - Full error context and chaining  

### 📦 Installation

```toml
[dependencies]
vaultmux = "0.1"
```

Or with specific backends:
```toml
vaultmux = { version = "0.1", features = ["aws", "gcp", "azure"] }
```

### 📚 Quick Start

```rust
use vaultmux::{factory, Backend, Config, BackendType};

#[tokio::main]
async fn main() -> vaultmux::Result<()> {
    let config = Config::new(BackendType::Pass).with_prefix("myapp");
    let mut backend = factory::new_backend(config)?;
    backend.init().await?;
    
    let session = backend.authenticate().await?;
    backend.create_item("api-key", "secret", &*session).await?;
    
    Ok(())
}
```

### 📊 Stats

- **4,500+ lines** of Rust code
- **73 tests** passing (53 unit + 20 doc)
- **6 examples** with real-world patterns
- **100% API documentation** coverage
- **4 CI/CD workflows** for quality assurance
- **Zero warnings** (clippy, rustdoc)

### 🔒 Security

All security advisories documented in [SECURITY.md](SECURITY.md). No actual vulnerabilities affecting vaultmux's security model.

### 📖 Documentation

- [README](README.md) - Overview and quick start
- [Examples](examples/README.md) - 6 runnable examples
- [API Docs](https://docs.rs/vaultmux) - Complete API reference
- [Security](SECURITY.md) - Security policy
- [Changelog](CHANGELOG.md) - Full version history

### ⚙️ Requirements

- **Rust:** 1.88.0 or later
- **Runtime:** tokio
- **Platform:** Linux, macOS, or Windows

### 🙏 Acknowledgments

Rust port by Dayna Blackwell (Blackwell Systems™)  
Original Go implementation: https://github.com/blackwell-systems/vaultmux

### 📝 License

Dual-licensed under MIT OR Apache-2.0

---

**Ready to use in production!** Report issues at https://github.com/blackwell-systems/vaultmux-rs/issues
