# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 1.0.x   | :white_check_mark: |
| < 1.0   | :x:                |

## Reporting a Vulnerability

We take security seriously. If you discover a security vulnerability in Codebuddy, please report it privately:

1. **DO NOT** open a public GitHub issue
2. Email security issues to: security@goobits.com
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

## Running Security Audits

Check current security status:

```bash
# Run security audit
cargo audit

# Check for warnings
cargo audit --deny warnings

# Update advisory database
cargo audit --update-only
```

**Recommendation**: Run `cargo audit` before each release and monthly for production deployments.

### Interpreting Results

- **Critical/High**: Address immediately before deploying
- **Medium**: Review and plan fix in next release
- **Low/Warning**: Document as known issue if not applicable
- **Unmaintained dependencies**: Evaluate replacement options

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
