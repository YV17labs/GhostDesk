# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""GhostDesk — MCP server entry point.

Auth ≡ TLS. The server has exactly two postures, decided at boot by whether
an operator mounted a cert+key at ``/etc/ghostdesk/tls/server.{crt,key}``
(or overrode the paths via ``GHOSTDESK_TLS_CERT`` / ``GHOSTDESK_TLS_KEY``):

* **Cert mounted** — uvicorn serves ``https://`` and an ASGI middleware
  rejects any request missing ``Authorization: Bearer <GHOSTDESK_AUTH_TOKEN>``
  with a constant-time comparison. ``GHOSTDESK_AUTH_TOKEN`` is mandatory in
  this posture; ``docker/init/entrypoint.sh`` refuses to boot without it.

* **No cert** — uvicorn serves plain ``http://`` and no application-level
  gate is installed. Shipping a static bearer token over cleartext would
  be security theater (no rotation, no per-user identity), so we
  intentionally leave the surface open and expect the operator to either
  mount a cert or keep the port on a trusted loopback / devcontainer
  forward. See SECURITY.md § Authentication for the full rationale.
"""

import hmac
import logging
import os
from pathlib import Path

import uvicorn
from mcp.server.fastmcp import FastMCP

from ghostdesk._coords import model_space_var
from ghostdesk._icons import GHOSTDESK_ICONS
from ghostdesk._lifespan import lifespan
from ghostdesk._logging import configure_logging
from ghostdesk._middleware import install_middleware
from ghostdesk.instructions import INSTRUCTIONS
from ghostdesk import apps, clipboard, input, resources, screen


_DEFAULT_TLS_CERT = "/etc/ghostdesk/tls/server.crt"
_DEFAULT_TLS_KEY = "/etc/ghostdesk/tls/server.key"

logger = logging.getLogger("ghostdesk")


def create_app(port: int | None = None, host: str | None = None) -> FastMCP:
    """Create and configure the MCP server instance.

    Args:
        port: Listening port. Defaults to the GHOSTDESK_PORT env var, or 3000.
        host: Bind address. Defaults to the GHOSTDESK_HOST env var, or
            ``127.0.0.1``. The container entrypoint exports
            ``GHOSTDESK_HOST=0.0.0.0`` so the MCP endpoint is reachable
            outside the container; standalone invocations stay on
            loopback by default per MCP transports spec.
    """
    if port is None:
        port = int(os.environ.get("GHOSTDESK_PORT", "3000"))
    if host is None:
        host = os.environ.get("GHOSTDESK_HOST", "127.0.0.1")

    mcp = FastMCP(
        "ghostdesk",
        instructions=INSTRUCTIONS,
        icons=GHOSTDESK_ICONS,
        lifespan=lifespan,
        host=host,
        port=port,
    )

    screen.register(mcp)
    input.register(mcp)
    apps.register(mcp)
    clipboard.register(mcp)
    resources.register(mcp)

    install_middleware(mcp)

    return mcp


def _resolve_tls_paths() -> tuple[str, str] | None:
    """Return (cert, key) if operator-mounted TLS files are present, else None.

    Paths are read from GHOSTDESK_TLS_CERT / GHOSTDESK_TLS_KEY, defaulting to
    /etc/ghostdesk/tls/server.{crt,key}. Both files must exist and be
    non-empty; otherwise the server runs in plain-HTTP / no-auth dev posture.
    """
    cert = Path(os.environ.get("GHOSTDESK_TLS_CERT", _DEFAULT_TLS_CERT))
    key = Path(os.environ.get("GHOSTDESK_TLS_KEY", _DEFAULT_TLS_KEY))
    if cert.is_file() and cert.stat().st_size > 0 and key.is_file() and key.stat().st_size > 0:
        return (str(cert), str(key))
    return None


def _find_header(scope: dict, name: bytes) -> bytes | None:
    """Return the raw bytes value of the first matching header, or None."""
    for hname, hvalue in scope.get("headers", ()):
        if hname == name:
            return hvalue
    return None


def _model_space_middleware(app):
    """ASGI middleware: bind ``GhostDesk-Model-Space`` to ``model_space_var``.

    ``ContextVar`` is per-task, so concurrent requests with different
    headers stay isolated. Missing or malformed header → ``0``
    (pass-through, frontier-model default).
    """
    async def _wrapped(scope, receive, send):
        if scope["type"] != "http":
            return await app(scope, receive, send)

        value = 0
        raw = _find_header(scope, b"ghostdesk-model-space")
        if raw is not None:
            try:
                parsed = int(raw.decode("ascii", "ignore").strip())
                if parsed > 0:
                    value = parsed
            except ValueError:
                pass

        token = model_space_var.set(value)
        try:
            return await app(scope, receive, send)
        finally:
            model_space_var.reset(token)

    return _wrapped


def _origin_middleware(app, allowed_origins: frozenset[str]):
    """ASGI middleware: reject browser requests with a non-allowlisted ``Origin``.

    MCP transports spec § Streamable HTTP requires servers to validate the
    ``Origin`` header to prevent DNS rebinding attacks. Non-browser clients
    (Claude Desktop, the Anthropic SDK, ``curl``) do not send ``Origin``,
    so requests without the header pass through. Requests *with* an Origin
    must match the allow-list (``GHOSTDESK_ALLOWED_ORIGINS``, comma-separated)
    or the request is rejected with HTTP 403. Lifespan events pass through.
    """
    async def _wrapped(scope, receive, send):
        if scope["type"] != "http":
            return await app(scope, receive, send)

        raw = _find_header(scope, b"origin")
        origin = raw.decode("ascii", "ignore").strip() if raw is not None else ""

        if origin and origin not in allowed_origins:
            await send({
                "type": "http.response.start",
                "status": 403,
                "headers": [(b"content-type", b"text/plain; charset=utf-8")],
            })
            await send({"type": "http.response.body", "body": b"forbidden origin\n"})
            return

        return await app(scope, receive, send)

    return _wrapped


def _bearer_auth_middleware(app, expected_token: str):
    """ASGI middleware: require ``Authorization: Bearer <expected_token>``.

    Constant-time comparison via ``hmac.compare_digest`` to avoid leaking
    token length or prefix through response timing. Lifespan events pass
    through untouched so uvicorn startup/shutdown still works.
    """
    expected_bytes = expected_token.encode("utf-8")

    async def _send_401(send) -> None:
        await send({
            "type": "http.response.start",
            "status": 401,
            "headers": [
                (b"content-type", b"text/plain; charset=utf-8"),
                (b"www-authenticate", b'Bearer realm="ghostdesk"'),
            ],
        })
        await send({"type": "http.response.body", "body": b"unauthorized\n"})

    async def _wrapped(scope, receive, send):
        if scope["type"] != "http":
            return await app(scope, receive, send)

        provided = _find_header(scope, b"authorization")
        if provided is None or not provided.startswith(b"Bearer "):
            return await _send_401(send)

        token = provided[len(b"Bearer "):].strip()
        if not hmac.compare_digest(token, expected_bytes):
            return await _send_401(send)

        return await app(scope, receive, send)

    return _wrapped


def _parse_allowed_origins(raw: str) -> frozenset[str]:
    """Parse a comma-separated origin list into a frozenset, dropping blanks."""
    return frozenset(s.strip() for s in raw.split(",") if s.strip())


def main() -> None:
    """Start the MCP server with Streamable HTTP transport."""
    configure_logging()
    app = create_app()

    tls = _resolve_tls_paths()
    asgi_app = app.streamable_http_app()
    asgi_app = _model_space_middleware(asgi_app)

    allowed_origins = _parse_allowed_origins(
        os.environ.get("GHOSTDESK_ALLOWED_ORIGINS", "")
    )
    asgi_app = _origin_middleware(asgi_app, allowed_origins)
    if allowed_origins:
        logger.info(
            "mcp-server: Origin allow-list active (%s)",
            ", ".join(sorted(allowed_origins)),
        )
    else:
        logger.info(
            "mcp-server: no Origin allow-list — non-browser clients only "
            "(any browser request with an Origin header will be rejected). "
            "Set GHOSTDESK_ALLOWED_ORIGINS to allow specific browser origins."
        )

    ssl_certfile: str | None = None
    ssl_keyfile: str | None = None

    if tls is not None:
        ssl_certfile, ssl_keyfile = tls
        token = os.environ.get("GHOSTDESK_AUTH_TOKEN", "")
        if not token:
            # entrypoint.sh § 3 already enforces this; belt-and-braces for
            # non-container invocations (e.g. uv run ghostdesk locally with
            # a cert mounted but no token exported).
            raise SystemExit(
                "GHOSTDESK_AUTH_TOKEN is required when TLS is enabled "
                f"(cert={ssl_certfile})"
            )
        asgi_app = _bearer_auth_middleware(asgi_app, token)
        logger.info("mcp-server: TLS enabled, bearer-token auth required")
    else:
        logger.warning(
            "mcp-server: no TLS cert — serving plain HTTP with NO authentication. "
            "This is the intended dev posture; deploy behind a cert + "
            "GHOSTDESK_AUTH_TOKEN for any non-loopback exposure."
        )

    config = uvicorn.Config(
        asgi_app,
        host=app.settings.host,
        port=app.settings.port,
        log_level=app.settings.log_level.lower(),
        ssl_certfile=ssl_certfile,
        ssl_keyfile=ssl_keyfile,
    )
    uvicorn.Server(config).run()


if __name__ == "__main__":
    main()
