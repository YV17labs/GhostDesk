# Security Policy

## Reporting Security Vulnerabilities

If you discover a security vulnerability in GhostDesk, **please do not open a public issue**. Instead, please report it responsibly using GitHub's private vulnerability reporting feature.

### How to Report

1. **Via GitHub:** Visit the [Security tab](../../security/advisories) and click "Report a vulnerability"
2. **Description:** Include:
   - Type of vulnerability (e.g., XSS, SQL Injection, Authentication bypass)
   - Location in code (file, line number if possible)
   - Proof of concept or steps to reproduce
   - Potential impact
   - Suggested fix (if you have one)

### Response Timeline

| Severity | Examples | Patch timeline |
|----------|----------|----------------|
| **Critical** | Remote code execution, auth bypass, unauthorized data access | ASAP (within 48h) |
| **High** | Privilege escalation, session hijacking, DoS | 1-2 weeks |
| **Medium** | XSS, CSRF, weak cryptography | 1 month |
| **Low** | Minor info disclosure, user enumeration | Next release |

We aim to acknowledge reports within 24 hours and provide an initial assessment within 3-5 business days.

## Security Practices

- All code changes go through peer review
- Branch protection rules require status checks to pass
- Dependency vulnerabilities monitored via Dependabot
- Secrets scanning is enabled
- Never commit secrets, API keys, or credentials

## Supported Versions

| Version | Supported |
|---------|-----------|
| Latest  | ✅ Yes    |

Security fixes are provided for the current major version. Users are encouraged to upgrade to the latest version.

## Contact

For sensitive security discussions, open a [private security advisory](../../security/advisories) on GitHub.

## Acknowledgments

We appreciate researchers and community members who responsibly report security vulnerabilities. Valid reports may be acknowledged in security advisories.
