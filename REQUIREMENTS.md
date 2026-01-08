# Vaultmux-rs Requirements

## Rust Version

- **Minimum**: Rust 1.75 (for most backends)
- **Recommended**: Rust 1.88+ (for AWS SDK backend)

### Backend-Specific Requirements

| Backend | Min Rust Version | Reason |
|---------|------------------|--------|
| Mock | 1.75 | Core features only |
| pass | 1.75 | No SDK dependencies |
| Bitwarden | 1.75 | No SDK dependencies |
| 1Password | 1.75 | No SDK dependencies |
| **AWS Secrets Manager** | **1.88** | AWS SDK v0.39+ requires 1.88 |
| GCP Secret Manager | 1.75 | google-secretmanager1 compatible |
| Azure Key Vault | 1.75 | azure_security_keyvault compatible |
| Windows Cred Mgr | 1.75 | windows crate compatible |

## Upgrading Rust

```bash
# Update to latest stable
rustup update stable

# Or install specific version
rustup install 1.88.0
rustup default 1.88.0
```

## Building Without AWS

If you're on Rust < 1.88, you can build without the AWS backend:

```bash
# Default features (mock only)
cargo build

# With CLI backends
cargo build --features "pass,bitwarden"

# All except AWS
cargo build --features "mock,pass,bitwarden"
```

## Building With AWS (Rust 1.88+)

```bash
# Enable AWS backend
cargo build --features "aws"

# Or all backends
cargo build --features "full"
```

## External Tool Requirements

### CLI Backends

| Backend | Required Tools | Installation |
|---------|---------------|--------------|
| **pass** | `pass`, `gpg` | `apt install pass gnupg` (Debian/Ubuntu)<br>`brew install pass gnupg` (macOS) |
| **Bitwarden** | `bw` | `npm install -g @bitwarden/cli` |
| **1Password** | `op` | https://1password.com/downloads/command-line/ |

### SDK Backends

| Backend | Required | Notes |
|---------|----------|-------|
| **AWS** | AWS credentials | Environment vars, ~/.aws/credentials, or IAM role |
| **GCP** | GCP credentials | Application Default Credentials (ADC) |
| **Azure** | Azure credentials | Azure CLI login or managed identity |

## Development Requirements

```bash
# Install development tools
cargo install cargo-watch  # File watching
cargo install cargo-edit    # Dependency management
cargo install cargo-audit   # Security audits
```

## Testing Requirements

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
# Requires backends to be installed and configured
VAULTMUX_TEST_PASS=1 cargo test --features pass -- --ignored
VAULTMUX_TEST_BITWARDEN=1 cargo test --features bitwarden -- --ignored
VAULTMUX_TEST_AWS=1 cargo test --features aws -- --ignored
```
