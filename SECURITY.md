# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

Please report security vulnerabilities by emailing security@blackwellsystems.com or by opening a private security advisory on GitHub.

## Known Issues

### Third-Party Dependencies

The following security advisories affect transitive dependencies from third-party backend SDKs:

#### GCP Backend (`google-secretmanager1` crate)

**RUSTSEC-2025-0066: google-apis-common is unmaintained**
- **Severity**: Informational
- **Status**: Acknowledged, waiting for official Google Rust SDK
- **Impact**: The `google-apis-common` crate is no longer maintained. Google recommends migrating to their official Rust bindings.
- **Mitigation**: The GCP backend functionality is not affected. This is purely an informational warning about future maintenance.
- **Action**: Will migrate to official Google Cloud Rust SDK when it provides equivalent functionality.

**RUSTSEC-2024-0421: idna Punycode vulnerability (CVE-2024-12224)**
- **Severity**: Privilege escalation (conditional)
- **Status**: Acknowledged, indirect dependency
- **Affected**: `idna` 0.1.5 (via `url` 1.7.2 via `google-apis-common`)
- **Impact**: Theoretical privilege escalation when hostname comparison is part of authentication **AND** attacker controls DNS/TLS certificates **AND** application relies on idna processing for security decisions.
- **Mitigation**: vaultmux does not use hostname comparison for privilege checks. GCP credentials are obtained via OAuth2 and verified by Google's infrastructure, not by idna processing.
- **Action**: Waiting for upstream `google-secretmanager1` to update dependencies.

#### Other Unmaintained Dependencies

The following are informational warnings about transitive dependencies:

- **RUSTSEC-2024-0384**: `instant` crate is unmaintained (transitive dependency)
  - No known vulnerabilities, functionality complete and stable
  - Waiting for upstream crates to migrate

- **RUSTSEC-2024-0436**: `paste` crate no longer maintained (transitive dependency)
  - No known vulnerabilities, functionality complete and stable
  - Waiting for upstream crates to migrate

- **RUSTSEC-2025-0134**: `rustls-pemfile` is unmaintained (transitive dependencies)
  - Users should migrate to `rustls-pki-types` (done in rustls itself)
  - Waiting for upstream crates to complete migration

## Security Best Practices

When using vaultmux:

1. **Credentials**: Never commit secrets to source control
2. **Session Caching**: Session cache files are created with 0600 permissions (user-only)
3. **Input Validation**: All item names are validated to prevent command injection
4. **TLS**: Cloud backends use TLS for all API communication
5. **Updates**: Run `cargo update` regularly to get security patches

## Vulnerability Disclosure Timeline

We aim to:
- Acknowledge reports within 48 hours
- Provide initial assessment within 1 week
- Release patches for confirmed vulnerabilities within 2 weeks
- Credit reporters (unless anonymity requested)
