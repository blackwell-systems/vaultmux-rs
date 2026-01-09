# Contributing to Vaultmux

Thank you for your interest in contributing to Vaultmux! This document provides guidelines and instructions for contributing.

## Code of Conduct

Be respectful, constructive, and professional in all interactions.

## How to Contribute

### Reporting Issues

- Search existing issues before creating a new one
- Provide clear reproduction steps
- Include relevant version information
- Attach logs or error messages when applicable

### Submitting Pull Requests

1. **Fork the repository** and create a feature branch
2. **Write tests** for new functionality
3. **Run the test suite**: `cargo test --all-features`
4. **Check formatting**: `cargo fmt`
5. **Run Clippy**: `cargo clippy --all-features -- -D warnings`
6. **Update documentation** as needed
7. **Commit with clear messages** following conventional commits format

### Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/vaultmux-rs
cd vaultmux-rs

# Install Rust (if needed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Run tests
cargo test --all-features

# Build documentation
cargo doc --all-features --open
```

## Code Guidelines

### Style

- Follow Rust standard style (`cargo fmt`)
- Pass all Clippy lints (`cargo clippy`)
- Add documentation for public APIs
- Include examples in doc comments

### Testing

- Write unit tests for new functionality
- Add integration tests for backend implementations
- Ensure all tests pass before submitting PR
- Aim for high test coverage on critical paths

### Documentation

- Document all public functions, structs, and traits
- Include usage examples in doc comments
- Update README.md for significant changes
- Add entries to CHANGELOG.md

## Backend Implementation

When adding a new backend:

1. Create a new module in `src/backends/`
2. Implement the `Backend` trait
3. Add feature flag to `Cargo.toml`
4. Register backend in `src/backends/mod.rs`
5. Add tests (unit and integration)
6. Update documentation and examples

### Backend Requirements

- Implement all `Backend` trait methods
- Call `validate_item_name()` before operations
- Handle errors appropriately
- Support session caching
- Write comprehensive tests

## Testing

### Unit Tests

```bash
# Run all unit tests
cargo test --lib --all-features

# Run tests for specific backend
cargo test --lib --features bitwarden
```

### Integration Tests

```bash
# AWS with LocalStack
docker run -d -p 4566:4566 localstack/localstack
cargo test --test integration_aws --features aws -- --ignored

# Other backends require actual credentials
cargo test --test integration_pass --features pass -- --ignored
```

### Examples

Test that examples compile and work:

```bash
cargo run --example basic_usage
cargo run --example aws_secrets --features aws
```

## Pull Request Process

1. **Create focused PRs** - One feature or fix per PR
2. **Write clear descriptions** - Explain what and why
3. **Link related issues** - Reference issue numbers
4. **Respond to feedback** - Address review comments
5. **Keep commits clean** - Squash or rebase if needed

### PR Checklist

- [ ] Tests pass locally
- [ ] Clippy passes with no warnings
- [ ] Code is formatted with `rustfmt`
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Examples work (if applicable)
- [ ] No unnecessary dependencies added

## Release Process

Releases are managed by maintainers:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag `vX.Y.Z`
4. Publish to crates.io
5. Create GitHub release

## Questions?

- Open an issue for questions
- Check existing documentation
- Review examples for usage patterns

## License

By contributing, you agree that your contributions will be licensed under the same terms as the project (MIT OR Apache-2.0).
