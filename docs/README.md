# Vaultmux Documentation

Complete documentation for vaultmux - unified secret management for Rust.

## Documentation Index

### Getting Started

- **[User Guide](user-guide.md)** - Complete guide to using vaultmux
  - Installation and setup
  - Core concepts
  - Backend configuration
  - Common patterns
  - Best practices

- **[API Reference](api-reference.md)** - Detailed API documentation
  - Core traits
  - Types and enums
  - Factory functions
  - Error handling
  - Backend-specific details

### Additional Resources

- **[README](../README.md)** - Quick start and project overview
- **[Examples](../examples/)** - Working code examples
- **[CONTRIBUTING](../CONTRIBUTING.md)** - Contribution guidelines

## Quick Links

### By Use Case

**I want to...**

- **Get started quickly** → [Quick Start](user-guide.md#quick-start)
- **Configure AWS Secrets Manager** → [AWS Configuration](user-guide.md#aws-secrets-manager)
- **Test my application** → [Testing Guide](user-guide.md#testing)
- **Handle errors properly** → [Error Handling](user-guide.md#error-handling)
- **Switch backends easily** → [Environment-Based Selection](user-guide.md#environment-based-backend-selection)
- **Look up a method** → [API Reference](api-reference.md)

### By Backend

- [Pass (password-store)](user-guide.md#pass-unix-password-store)
- [Bitwarden](user-guide.md#bitwarden)
- [1Password](user-guide.md#1password)
- [AWS Secrets Manager](user-guide.md#aws-secrets-manager)
- [Google Cloud Secret Manager](user-guide.md#google-cloud-secret-manager)
- [Azure Key Vault](user-guide.md#azure-key-vault)
- [Windows Credential Manager](user-guide.md#windows-credential-manager)
- [Mock (testing)](api-reference.md#mockbackend)

## Documentation Philosophy

This documentation follows these principles:

1. **Complete** - Every feature is documented
2. **Practical** - Focus on real-world usage
3. **Current** - Updated with each release
4. **Accessible** - Examples for all experience levels

## Contributing to Docs

Found an error? Have a suggestion? Documentation improvements are always welcome!

- Typos and corrections: Submit a PR directly
- New sections: Open an issue to discuss first
- Examples: PRs with working code are appreciated

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

## Version

This documentation is for vaultmux v0.1.0.

Latest docs always available at: https://docs.rs/vaultmux
