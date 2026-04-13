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

---

## Threat Model

GhostDesk is a **single-tenant** service designed to run **behind a reverse proxy** on a trusted internal network or an identity-aware edge (Tailscale, Cloudflare Access, oauth2-proxy, Pomerium, etc.). It is **not designed to be exposed directly on the public internet**.

The threat boundary the container itself is responsible for defending:

| In scope (product) | Out of scope (deployment) |
|---|---|
| Transport encryption (TLS) on every exposed port | Rate limiting, brute-force protection |
| Mandatory authentication on MCP and VNC | SSO / OIDC / MFA |
| Secret material never leaves the container in env dumps | WAF, IP allow-listing |
| Read-only application code under the runtime user | Network segmentation between agents |
| Container isolation (filesystem, process, network namespaces) | Session recording & audit trail aggregation |
| Cert algorithm, length, and SAN meeting modern baselines | Cert trust chain provenance (CA / Let's Encrypt) |

If your deployment exposes GhostDesk without a proxy and an authenticated edge, that is a deployment vulnerability, not a product one. The sections below document exactly what the product does and does not guarantee so operators can close the gap deliberately.

## Transport Security

**TLS is mandatory on every network surface.** The product refuses to run without a certificate at `/etc/ghostdesk/tls/server.{crt,key}`.

- `websockify` serves `https://` and `wss://` only on port 6080 — there is no plain-HTTP fallback.
- `wayvnc` runs with `enable_auth=true` and a 2048-bit RSA key for **RSA-AES** transport encryption on the internal loopback hop (`websockify` ↔ `wayvnc`). Even the loopback traffic is encrypted and authenticated.
- The MCP server on port 3000 is served over HTTP by design: TLS termination and trust-chain handling belong to the reverse proxy in front of it. **Do not expose port 3000 directly.**

### Cert provisioning

At first boot, `docker/entrypoint.sh` generates a **self-signed** cert if none is mounted:

- Algorithm: **RSA-2048 + SHA-256**
- Validity: 10 years
- SAN: `DNS:localhost, IP:127.0.0.1, IP:::1`
- Subject: `CN=localhost, O=GhostDesk, OU=dev`

The self-signed cert is **cryptographically sound** — the browser warning is about trust-chain provenance (no CA signature), not algorithm weakness. This is appropriate for local dev only.

For production, mount a real cert over the default path:

```yaml
volumes:
  - /etc/letsencrypt/live/agent.example.com/fullchain.pem:/etc/ghostdesk/tls/server.crt:ro
  - /etc/letsencrypt/live/agent.example.com/privkey.pem:/etc/ghostdesk/tls/server.key:ro
```

The entrypoint detects the existing files and skips generation. Rotation is handled by whatever writes the mounted paths (cert-manager, Let's Encrypt cron, your PKI) — `docker restart` is enough for the new cert to take effect on the next websockify start.

## Authentication

Two independent credentials, one per surface:

| Credential | Used for | How it travels |
|---|---|---|
| `GHOSTDESK_AUTH_TOKEN` | MCP server on port 3000 | `Authorization: Bearer <token>` header on every request |
| `GHOSTDESK_VNC_PASSWORD` | wayvnc | RFB password auth over RSA-AES inside TLS |

Both are **mandatory** — the container refuses to boot if either is unset:

```
entrypoint: FATAL GHOSTDESK_AUTH_TOKEN is required (or provide GHOSTDESK_AUTH_TOKEN_FILE)
```

There is **no default credential**. There is **no "disable auth" toggle** — the dev and prod code paths are identical.

## Secrets Handling & Rotation

Secrets should be provided via the `*_FILE` convention, never as inline env vars on shared hosts:

```bash
GHOSTDESK_AUTH_TOKEN_FILE=/run/secrets/ghostdesk_auth_token
GHOSTDESK_VNC_PASSWORD_FILE=/run/secrets/ghostdesk_vnc_password
```

`entrypoint.sh` reads the file, exports the value for the current process tree, and lets the file permissions (set by Docker secrets / k8s Secrets / Vault) stay at mode 0400. This avoids three common leaks:

- Values never appear in `docker inspect` output.
- Values never appear in `/proc/*/environ` of other processes that might be readable by a sidecar.
- Values are never baked into an image layer or CI log.

### Rotation

- **MCP auth token** — update the mounted file, `docker restart`. The new token becomes active on the next container boot. Clients must be updated in the same window.
- **VNC password** — same procedure. `entrypoint.sh` re-renders `~/.config/wayvnc/config` at every boot, so the new password is picked up without touching the image.
- **TLS cert** — see [Transport Security](#transport-security) above.

There is no support for hot-reloading credentials without a restart. This is deliberate: the blast radius of a container restart is a few seconds of unavailability for a single agent, which is acceptable and easier to reason about than a live credential swap.

## Image Hardening

### Read-only application code

In the prod image (`Dockerfile.base`), after `uv sync` installs the Python venv at `/opt/ghostdesk/.venv`:

1. Source `.py` files are compiled to PEP 3147 sourceless `.pyc` (`compileall -b`) next to the originals.
2. All `.py` and `__pycache__` are deleted — only the flat `.pyc` remain.
3. The entire venv is `chown root:root` with `0555` on dirs and `0444` on files.

The runtime user (`agent`, UID 1000) can **import and execute** the code but **cannot modify it**. A compromised agent process cannot persist modifications to the product surface. Code tampering requires root inside the container, which in turn requires a container-escape CVE.

### Distribution image: `sudo NOPASSWD`

The `:latest` distribution image (`Dockerfile`) grants passwordless sudo to `agent`:

```
agent ALL=(ALL) NOPASSWD:ALL
```

**This is deliberate and scoped to the `:latest` tag** — the image is meant to be used as a playground where an LLM agent installs packages, runs debuggers, etc. It inherits the full Unix trust model of a single-user workstation.

**If you deploy GhostDesk into a production environment where the LLM agent should not be able to become root**, use the `:base` tag instead and build your own image without the sudo grant. The `:base` image has no sudo configured at all.

To remove sudo from the `:latest` image at runtime (useful for a one-off hardened container):

```bash
docker run ... ghcr.io/yv17labs/ghostdesk:latest \
  bash -c 'rm /etc/sudoers.d/agent && exec /usr/local/bin/entrypoint'
```

### Container-level isolation

- `cap_add: [SYS_ADMIN]` is documented as required by Electron apps (Chromium sandboxing via user namespaces). **Remove it if you don't run Electron apps** — it is the single largest capability grant in the default compose example and its removal materially shrinks the attack surface.
- No `--privileged`, no `/dev/*` mounts, no host network, no bind mounts to host paths other than the secrets files and (optionally) the TLS certs.
- The wallpaper, sway config, and supervisord config are baked into the image at build time, not mounted from the host.

## Known Limitations

- **Single shared VNC credential.** There is one username and one password per container, shared by every viewer. For per-user identity, audit trail, and revocation, use an identity-aware proxy in front of noVNC.
- **No hot reload.** Credential and cert rotation require a container restart.
- **Self-signed cert in dev triggers browser warnings.** This is a feature, not a bug — it signals to the developer that trust-chain setup is pending. Do not suppress the warning by disabling TLS.
- **Supply-chain trust on the base image.** `:base` is `ubuntu:24.04` + a fixed set of apt packages pinned to distribution repositories. Upstream CVEs in those packages are your responsibility to track (Dependabot + Trivy on your registry, or pull `:base` regularly to pick up base-image rebuilds). Supply-chain provenance is attached to every published image (SLSA provenance + SBOM, verifiable with `cosign verify-attestation`).

## Contact

For sensitive security discussions, open a [private security advisory](../../security/advisories) on GitHub.

## Acknowledgments

We appreciate researchers and community members who responsibly report security vulnerabilities. Valid reports may be acknowledged in security advisories.
