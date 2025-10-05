# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in Codebuddy, please report it privately:

1. **DO NOT** open a public GitHub issue
2. Email the maintainers at: [your-security-email@example.com]
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if available)

We will respond within 48 hours and provide a timeline for fixes.

## Security Best Practices

### For Users

1. **Always use HTTPS/WSS** for remote connections
2. **Enable JWT authentication** for production deployments
3. **Keep dependencies updated**: Run `cargo update` regularly
4. **Run security audits**: Use `cargo audit` before deploying

### For Contributors

1. **Run security checks** before submitting PRs:
   ```bash
   make audit
   cargo clippy -- -D warnings
   ```

2. **Review dependencies**:
   - Prefer well-maintained crates with security track records
   - Document any security-sensitive dependencies
   - Use `cargo-audit` to check for known vulnerabilities

3. **Follow secure coding practices**:
   - Avoid `unsafe` code unless absolutely necessary
   - Validate all inputs
   - Use structured error handling
   - Never log sensitive data

## Current Security Audit Status

Last audit: [Run `cargo audit` to check]

### Known Issues

#### RUSTSEC-2023-0071 (RSA Marvin Attack)
- **Severity**: Medium (5.9 CVSS)
- **Affected**: `rsa` crate via `jsonwebtoken`
- **Status**: **Not applicable to our use case**
- **Justification**: We only use HMAC (symmetric) JWT signing, not RSA operations
- **Action**: Monitor for jsonwebtoken updates

#### RUSTSEC-2024-0375 (atty unmaintained)
- **Severity**: Warning
- **Affected**: `atty` crate via `cb-client`
- **Status**: Tracked for replacement
- **Action**: Replace with `std::io::IsTerminal` (Rust 1.70+)

## Security Features

### Authentication
- JWT-based authentication with configurable expiry
- Support for project-scoped access tokens
- TLS/HTTPS support for encrypted transport

### Memory Safety
- Written in Rust for memory safety guarantees
- No `unsafe` code in core logic (1 occurrence total)
- Compile-time prevention of common vulnerabilities

### Input Validation
- Structured parameter validation for all MCP tools
- File path sanitization
- LSP message validation

### Audit Trail
- Structured logging for security events
- Request/response correlation via request IDs
- Configurable log levels for production monitoring

## Automated Security

### CI/CD Checks
```bash
# Run on every PR
cargo audit
cargo clippy -- -D warnings
cargo test --all-features
```

### Pre-commit Hooks
```bash
# Setup
git config core.hooksPath .git/hooks

# Add to .git/hooks/pre-commit
#!/bin/bash
cargo audit --deny warnings || exit 1
cargo clippy -- -D warnings || exit 1
```

## Security Updates

Subscribe to security advisories:
- [RustSec Advisory Database](https://rustsec.org/)
- [GitHub Security Advisories](https://github.com/goobits/codebuddy/security/advisories)

## License

This security policy is part of the Codebuddy project and follows the same MIT license.
