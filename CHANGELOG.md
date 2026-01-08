# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Complete Rust port of vaultmux with 8/8 backends implemented
- Mock backend for testing
- CLI backends: pass, Bitwarden, 1Password
- Cloud SDK backends: AWS Secrets Manager, GCP Secret Manager, Azure Key Vault
- Windows Credential Manager backend (PowerShell-based)
- Session caching with disk persistence (0600 permissions)
- Input validation to prevent command injection
- Factory pattern for backend registration
- Feature flags for optional backend compilation
- Comprehensive error handling with `VaultmuxError`
- GitHub Actions CI/CD workflows
  - Cross-platform testing (Linux, macOS, Windows)
  - Clippy linting and rustfmt formatting checks
  - Individual feature flag testing
  - Documentation generation
  - Security audit with cargo-audit
  - Automated dependency updates
  - Release automation with binary builds
  - Benchmark tracking

### Changed
- Async/await throughout (tokio runtime)
- Type-safe enums instead of string constants
- Compile-time feature flags for backends

### Documentation
- API documentation with rustdoc
- README with quick start guide
- Architecture documentation in RUST_PORT_PLAN.md
- Examples directory

## [0.1.0] - TBD

### Added
- Initial release with full feature parity to Go vaultmux
- All 8 backends implemented and tested
- 54 unit tests + 20 doc tests passing
- Cross-platform support (Linux, macOS, Windows)

[Unreleased]: https://github.com/blackwell-systems/vaultmux-rs/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/blackwell-systems/vaultmux-rs/releases/tag/v0.1.0
