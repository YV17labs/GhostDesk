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

**Auth ≡ TLS.** GhostDesk has exactly two postures, decided at boot by whether the operator mounted a cert+key at `/etc/ghostdesk/tls/server.{crt,key}`:

- **Cert mounted → prod posture.** Every exposed surface runs TLS *and* authenticated: `wss://` + bearer-token on the MCP server, `wss://` on `websockify` with password auth delegated down to wayvnc (RFB security type 2, single password prompt in the noVNC overlay). `GHOSTDESK_AUTH_TOKEN` and `GHOSTDESK_VNC_PASSWORD` are mandatory — the container refuses to boot without them.
- **No cert → dev posture.** Every exposed surface runs plain and **unauthenticated**. This is the devcontainer shape: the ports are reachable only via the IDE's localhost-scoped forward, so shipping a static bearer token or VNC password over cleartext would add no real defense (no rotation, no per-user identity, no per-request revocation). We intentionally disable the application-level gate in this posture rather than give a false sense of security; setting `GHOSTDESK_AUTH_TOKEN` or `GHOSTDESK_VNC_PASSWORD` without a cert logs a warning and is ignored.

The threat boundary the container itself is responsible for defending:

| In scope (product) | Out of scope (deployment) |
|---|---|
| TLS termination on every exposed port when a cert is mounted | Rate limiting, brute-force protection |
| Mandatory bearer-token auth on the MCP server and single-password auth at the wayvnc layer (RFB type 2) — **when TLS is on** | Per-user identity / SSO / OIDC / MFA on noVNC |
| Read-only application code under the runtime user | WAF, IP allow-listing |
| Container isolation (filesystem, process, network namespaces) | Network segmentation between agents |
| wayvnc hard-pinned to `127.0.0.1` (loopback only, inside the container's netns) | Trusted cert / TLS termination on the client-facing edge |
| Password challenge/response on wayvnc, wrapped by `wss://` TLS end-to-end from the browser to `websockify` on :6080 | Session recording & audit trail aggregation |

If your deployment exposes GhostDesk without a proxy and an authenticated edge, that is a deployment vulnerability, not a product one. The sections below document exactly what the product does and does not guarantee so operators can close the gap deliberately.

## Transport Security

GhostDesk's transport model is driven by a single switch: **is a cert mounted at `/etc/ghostdesk/tls/server.{crt,key}`?** The answer flips every exposed surface between *plain + unauthenticated* (dev) and *TLS + authenticated* (prod). The product does not generate or manage a TLS trust chain on its own.

- **Port 6080 — `websockify` / noVNC.** With a cert mounted, [`docker/services/websockify/run.sh`](docker/services/websockify/run.sh) starts `websockify` with `--cert`, `--key`, `--ssl-only`, serving `https://` + `wss://`. Without a cert it serves plain `http://` + `ws://`. websockify is the transport bridge only — authentication lives in wayvnc below.
- **Port 3000 — MCP server.** With a cert mounted, [`src/ghostdesk/server.py`](src/ghostdesk/server.py) runs uvicorn with `ssl_certfile` / `ssl_keyfile` and installs an ASGI middleware that rejects any request missing `Authorization: Bearer <GHOSTDESK_AUTH_TOKEN>` (constant-time compare via `hmac.compare_digest`). Without a cert, it serves plain HTTP with **no authentication gate** — the intended dev posture, described in [Authentication](#authentication) below.
- **Port 5900 — wayvnc.** Hard-pinned to `127.0.0.1` inside the container's netns by [`docker/init/entrypoint.sh`](docker/init/entrypoint.sh); `GHOSTDESK_VNC_ADDRESS` overrides are ignored. Under TLS, wayvnc is configured with `enable_auth=true` + `allow_broken_crypto=true` + `relax_encryption=true` + `password=${GHOSTDESK_VNC_PASSWORD}` (no username) so it advertises **RFB security type 2** (classic VNC Auth). noVNC 1.6 handles this natively and shows a single-password prompt in its overlay. The DES challenge/response used by RFB type 2 is cryptographically weak on its own — the upstream maintainer explicitly labels it "broken crypto" and it is only enabled because noVNC 1.6 still does not interoperate with wayvnc's modern security types (VeNCrypt X509Plain / RSA-AES). Confidentiality is provided end-to-end by the `wss://` envelope on the websockify leg; the DES flow is only an authentication token carried inside that tunnel. Because this configuration depends on a code path that only lives in wayvnc `master` (commit 1497397fc4, 2026-04-06), GhostDesk ships wayvnc built from pinned source — see [`docker/base/Dockerfile`](docker/base/Dockerfile) `vnc-builder` stage. Replace with a tag once upstream releases one that contains the commit.

### Cert provisioning

Mount a real cert over the default path in prod:

```yaml
volumes:
  - /etc/letsencrypt/live/agent.example.com/fullchain.pem:/etc/ghostdesk/tls/server.crt:ro
  - /etc/letsencrypt/live/agent.example.com/privkey.pem:/etc/ghostdesk/tls/server.key:ro
```

Both `websockify-run` and the MCP server detect the files at startup and enable TLS automatically. Rotation is handled by whatever writes the mounted paths (cert-manager, Let's Encrypt cron, your PKI); a `docker restart` picks up the new cert. Alternatively, terminate TLS on a reverse proxy / cloud LB in front of the container and leave both ports plain — that is the canonical deployment shape.

### Dev mode

In the devcontainer, no cert is mounted. `websockify` and MCP both serve plain HTTP inside the container with no application-level auth, and the IDE's port-forwarding layer (VS Code, GitHub Codespaces) wraps ports 6080 and 3000 in HTTPS with its own public cert on the client-facing leg. VS Code / Codespaces also scopes the host-side bind to the developer's machine, so the plain-HTTP leg between the IDE port-forward proxy and the container stays inside the trusted dev environment — it never hits the LAN.

If you want a locally-trusted cert for direct `https://localhost:6080` access (no IDE port-forward in between), [`mkcert`](https://github.com/FiloSottile/mkcert) is the recommended tool. It installs a local CA into your host trust store and issues short-lived leaf certs that your browser accepts without warnings:

```bash
mkcert -install
mkcert -cert-file ./tls/server.crt -key-file ./tls/server.key localhost 127.0.0.1 ::1
```

Then mount `./tls/server.crt` and `./tls/server.key` at `/etc/ghostdesk/tls/server.{crt,key}` in your compose file (same shape as the prod example above) and restart the container. GhostDesk does **not** generate a self-signed cert on your behalf — unmanaged self-signed certs train operators to click through browser warnings, and `mkcert` solves the same problem without that tradeoff.

## Authentication

**Auth is gated on TLS.** The presence of a mounted cert at `/etc/ghostdesk/tls/server.{crt,key}` is the single switch that arms (or disarms) both credentials:

| Surface | TLS off (dev) | TLS on (prod) |
|---|---|---|
| MCP server — port 3000 | Plain HTTP, **no auth** | `https://` + `Authorization: Bearer <GHOSTDESK_AUTH_TOKEN>` required on every request (constant-time compare) |
| wayvnc — port 5900 (via `websockify` on 6080) | Plain RFB on loopback, `enable_auth=false` | `wss://` on websockify + RFB security type 2 password challenge inside wayvnc (`GHOSTDESK_VNC_PASSWORD`, no username) |

### Prod posture (cert mounted)

`GHOSTDESK_AUTH_TOKEN` and `GHOSTDESK_VNC_PASSWORD` are **mandatory**. The container refuses to boot if either is missing:

```
entrypoint: FATAL GHOSTDESK_AUTH_TOKEN is required when TLS is enabled (cert mounted at /etc/ghostdesk/tls/server.crt)
entrypoint: FATAL GHOSTDESK_VNC_PASSWORD is required when TLS is enabled (cert mounted at /etc/ghostdesk/tls/server.crt)
```

The MCP server has a belt-and-braces check that raises `SystemExit` with the same message for non-container invocations (e.g. `uv run ghostdesk` locally with a cert mounted but no token exported). See [`src/ghostdesk/server.py`](src/ghostdesk/server.py) `main()`.

### Dev posture (no cert)

Both secrets are **ignored**, with a warning logged at boot if they are set anyway:

```
entrypoint: WARN GHOSTDESK_AUTH_TOKEN is set but TLS is off — ignored (no point shipping a static token over cleartext)
entrypoint: WARN GHOSTDESK_VNC_PASSWORD is set but TLS is off — ignored (wayvnc auth is only enabled under TLS)
```

The rationale is deliberate: GhostDesk's credentials are **static shared secrets** with no rotation, no per-user identity, and no revocation story. Over a cleartext channel they buy no real defense against anyone who can observe the transport — they only paper over the dev surface with a thin veneer of "auth is on" that a `tcpdump` peels off in seconds. The alternatives are both worse:

1. **Require secrets unconditionally**, as earlier builds did. This trains operators to paste placeholder tokens into committed compose files and forget they are there when the container moves to a public host.
2. **Enforce bearer auth even without TLS**, then document "it's fine because loopback." That's correct today but fragile — a small refactor or a misread `ports:` stanza is enough to leak a cleartext token to the LAN.

Instead, auth is bound to the posture that actually protects it. If you want auth, you get TLS with it; if you don't have TLS, the surface is honest about being open, and the operator is expected to constrain reachability some other way (devcontainer forward, Unix socket, loopback bind, reverse proxy). There is no middle ground and no "disable auth" toggle — just mount a cert or don't.

### The noVNC deployment contract

When TLS is on, the in-container VNC password is defense in depth, not a substitute for a proper front door. Port 6080 **must still not be exposed in clear on the public internet**, and in the dev posture (no cert, no VNC password) it must not be exposed anywhere except a trusted loopback forward. Any of the following satisfy the contract for the prod posture:

- **Reverse proxy with Basic Auth / OAuth2-proxy** (Caddy, nginx, Traefik, Apache) — the canonical prod setup. Pair with a real TLS cert on the edge.
- **Identity-aware proxy** — Cloudflare Access, Tailscale Funnel + ACLs, Pomerium, GCP IAP, AWS ALB + Cognito. The most robust option, recommended for multi-operator deployments.
- **Private network only** — Tailscale, WireGuard, VPC peering. Port 6080 is never reachable from outside the private network.
- **Devcontainer / Codespaces port forward** — the IDE port-forwarding layer gates access with your GitHub / VS Code identity and terminates TLS with its own public cert. Intended dev posture, satisfied out-of-the-box.

Why a front door on top of the in-container password? Two reasons: (1) the in-container password is a **single shared credential** with no per-user identity, audit trail, or revocation story — a reverse proxy with OAuth2-proxy gives you all three; (2) TLS termination on the client-facing leg is still the operator's job (the in-container TLS path is optional and defaults off). Exposing port 6080 directly to the internet — even with the VNC password enabled — is a **deployment vulnerability**, not a product one.

## Secrets Handling & Rotation

Both secrets are provided as plain environment variables, and only consulted in the prod posture (cert mounted). The recommended wiring depends on your orchestrator:

- **Kubernetes** — `valueFrom.secretKeyRef` pointing at a `Secret`. The value never appears in `kubectl describe pod` (only the reference does). With encryption-at-rest enabled on etcd (a baseline SOC2 / ISO 27001 / PCI control), the secret is protected at every stage.
- **Docker / compose** — inject via `environment:` backed by a git-ignored `.env` file, Docker secrets, Vault agent templating, or External Secrets Operator. Never inline secrets in a committed compose file.
- **Local dev** — the devcontainer ships **no** `GHOSTDESK_AUTH_TOKEN` / `GHOSTDESK_VNC_PASSWORD` in [`.devcontainer/docker-compose.yml`](.devcontainer/docker-compose.yml), because no cert is mounted there and auth is disabled on purpose. If you want to exercise the prod code path locally, mount an `mkcert`-issued cert (see [Transport Security § Dev mode](#dev-mode)) and `export GHOSTDESK_AUTH_TOKEN=...` + `export GHOSTDESK_VNC_PASSWORD=...` before bringing the container up.

No `*_FILE` indirection is supported. The rationale: on Kubernetes with `secretKeyRef`, env vars are already at parity with file mounts for audit purposes (both result in the value living in the pod's env; neither shows up in the pod spec). Maintaining a second code path for `*_FILE` bought nothing on the platform operators actually run, and made the API surface larger. TLS material stays on disk because PEM files are the natural format for OpenSSL-based libraries and cert-manager / Let's Encrypt rotation writes files, not env vars.

### Rotation

- **MCP auth token** — update the `Secret` / env source, `kubectl rollout restart` or `docker restart`. The new token becomes active on the next container boot. Clients must be updated in the same window.
- **VNC password** — same procedure. `entrypoint.sh` re-renders `~/.config/wayvnc/config` at every boot, so the new password is picked up on the next container restart without touching the image.
- **TLS cert** — see [Transport Security](#transport-security) above.

There is no support for hot-reloading credentials without a restart. This is deliberate: the blast radius of a container restart is a few seconds of unavailability for a single agent, which is acceptable and easier to reason about than a live credential swap.

## Image Hardening

### Read-only application code

In the prod image (`docker/base/Dockerfile`), after `uv sync` installs the Python venv at `/opt/ghostdesk/.venv`:

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

- **Single shared VNC credential.** `GHOSTDESK_VNC_PASSWORD` is one password shared by every viewer — the container has no notion of per-user identity on the noVNC surface. For per-user audit trail and revocation, front port 6080 with an identity-aware proxy; see [The noVNC deployment contract](#the-novnc-deployment-contract).
- **Single shared MCP token.** Same story for `GHOSTDESK_AUTH_TOKEN`: one static bearer, no rotation, no per-client identity. Treat it as a coarse gate on a trusted hop, not a user-facing credential.
- **No hot reload.** Credential and cert rotation require a container restart.
- **TLS termination is the operator's responsibility.** The container serves plain HTTP *and disables auth* on 6080 and 3000 unless a cert is mounted at `/etc/ghostdesk/tls/server.{crt,key}`. Deploying either port in that state outside a trusted loopback forward is a deployment vulnerability, not a product one — see [Authentication](#authentication).
- **Supply-chain trust on the base image.** `:base` is `ubuntu:24.04` + a fixed set of apt packages pinned to distribution repositories. Upstream CVEs in those packages are your responsibility to track (Dependabot + Trivy on your registry, or pull `:base` regularly to pick up base-image rebuilds). Supply-chain provenance is attached to every published image (SLSA provenance + SBOM, verifiable with `cosign verify-attestation`).

## Contact

For sensitive security discussions, open a [private security advisory](../../security/advisories) on GitHub.

## Acknowledgments

We appreciate researchers and community members who responsibly report security vulnerabilities. Valid reports may be acknowledged in security advisories.
