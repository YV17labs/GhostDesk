# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk.server — MCP server entry point."""

import importlib
import sys
from unittest.mock import MagicMock, patch

import pytest

from ghostdesk.server import create_app


async def test_create_app_returns_fastmcp():
    """create_app() returns a FastMCP instance."""
    from mcp.server.fastmcp import FastMCP

    app = create_app(port=9999)

    assert isinstance(app, FastMCP)


async def test_create_app_has_expected_tools():
    """create_app() registers the expected number of tools."""
    app = create_app(port=9999)

    tools = app._tool_manager._tools
    assert len(tools) == 12, f"Expected 12 tools, got {len(tools)}: {sorted(tools.keys())}"


@pytest.mark.filterwarnings("ignore::RuntimeWarning:pydantic.fields")
async def test_create_app_custom_port():
    """create_app() accepts a custom port."""
    app = create_app(port=4567)

    assert app.settings.port == 4567


async def test_create_app_default_port_from_env():
    """create_app() reads GHOSTDESK_PORT from environment when no port is given."""
    with patch.dict("os.environ", {"GHOSTDESK_PORT": "8080"}):
        app = create_app()

    assert app.settings.port == 8080


async def test_create_app_default_port_fallback():
    """create_app() defaults to port 3000 when GHOSTDESK_PORT env var is unset."""
    with patch.dict("os.environ", {}, clear=True):
        app = create_app()

    assert app.settings.port == 3000


async def test_import_server_no_side_effect():
    """Importing ghostdesk.server does NOT create a server as a side-effect."""
    # Remove module from cache so re-import is fresh
    mod_name = "ghostdesk.server"
    saved = sys.modules.pop(mod_name, None)
    try:
        with patch("ghostdesk.server.create_app") as mock_create:
            # Re-import the module
            importlib.import_module(mod_name)
            # create_app should not be called at import time
            mock_create.assert_not_called()
    finally:
        if saved is not None:
            sys.modules[mod_name] = saved


def _mock_uvicorn_main_run() -> tuple:
    """Patch the uvicorn server so main() returns immediately.

    Returns the (uvicorn_config, uvicorn_server) mocks so callers can
    assert on the host/port/ssl arguments uvicorn was invoked with.
    """
    server_mock = MagicMock()
    server_mock.return_value.run = MagicMock()
    config_mock = MagicMock()
    return config_mock, server_mock


def test_main_calls_create_app_and_runs():
    """main() calls configure_logging(), create_app(), and starts uvicorn."""
    from ghostdesk.server import main

    config_mock, server_mock = _mock_uvicorn_main_run()
    with (
        patch("ghostdesk.server.configure_logging") as mock_logging,
        patch("ghostdesk.server.create_app") as mock_create,
        patch("ghostdesk.server._resolve_tls_paths", return_value=None),
        patch("ghostdesk.server.uvicorn.Config", config_mock),
        patch("ghostdesk.server.uvicorn.Server", server_mock),
    ):
        mock_create.return_value.streamable_http_app.return_value = MagicMock()
        mock_create.return_value.settings.host = "127.0.0.1"
        mock_create.return_value.settings.port = 3000
        mock_create.return_value.settings.log_level = "INFO"

        main()

        mock_logging.assert_called_once()
        mock_create.assert_called_once_with()
        server_mock.return_value.run.assert_called_once()


def test_main_with_port_env_var():
    """main() respects GHOSTDESK_PORT env var when calling create_app."""
    from ghostdesk.server import main

    config_mock, server_mock = _mock_uvicorn_main_run()
    with (
        patch("ghostdesk.server.configure_logging"),
        patch("ghostdesk.server.create_app") as mock_create,
        patch("ghostdesk.server._resolve_tls_paths", return_value=None),
        patch("ghostdesk.server.uvicorn.Config", config_mock),
        patch("ghostdesk.server.uvicorn.Server", server_mock),
        patch.dict("os.environ", {"GHOSTDESK_PORT": "5000"}),
    ):
        mock_create.return_value.streamable_http_app.return_value = MagicMock()
        mock_create.return_value.settings.host = "127.0.0.1"
        mock_create.return_value.settings.port = 5000
        mock_create.return_value.settings.log_level = "INFO"

        main()

        # GHOSTDESK_PORT is read inside create_app(), which is mocked here;
        # the test asserts main() flows correctly with the env var set.
        mock_create.assert_called_once_with()
        server_mock.return_value.run.assert_called_once()


# ---------------------------------------------------------------------------
# create_app — host binding (GHOSTDESK_HOST)
# ---------------------------------------------------------------------------


async def test_create_app_default_host_is_loopback():
    """create_app() defaults to 127.0.0.1 per MCP transports spec."""
    with patch.dict("os.environ", {}, clear=True):
        app = create_app()
    assert app.settings.host == "127.0.0.1"


async def test_create_app_host_from_env():
    """create_app() reads GHOSTDESK_HOST from environment."""
    with patch.dict("os.environ", {"GHOSTDESK_HOST": "0.0.0.0"}):
        app = create_app()
    assert app.settings.host == "0.0.0.0"


async def test_create_app_host_explicit_arg_wins():
    """Explicit host argument overrides the env var."""
    with patch.dict("os.environ", {"GHOSTDESK_HOST": "0.0.0.0"}):
        app = create_app(host="192.0.2.1")
    assert app.settings.host == "192.0.2.1"


# ---------------------------------------------------------------------------
# _origin_middleware — DNS rebinding protection (MCP transports spec)
# ---------------------------------------------------------------------------


async def _drive_origin_middleware(headers, allowed_origins):
    """Drive the Origin middleware once and return (status, body)."""
    from ghostdesk.server import _origin_middleware

    inner_called = {"hit": False}

    async def inner_app(scope, receive, send):
        inner_called["hit"] = True
        await send({"type": "http.response.start", "status": 200, "headers": []})
        await send({"type": "http.response.body", "body": b"ok"})

    wrapped = _origin_middleware(inner_app, frozenset(allowed_origins))
    scope = {"type": "http", "headers": headers}

    captured: dict = {"status": None, "body": b""}

    async def send(msg):
        if msg["type"] == "http.response.start":
            captured["status"] = msg["status"]
        elif msg["type"] == "http.response.body":
            captured["body"] += msg.get("body", b"")

    await wrapped(scope, None, send)
    return captured["status"], captured["body"], inner_called["hit"]


async def test_origin_missing_passes_through():
    """Non-browser clients (no Origin header) are allowed through."""
    status, _, hit = await _drive_origin_middleware([], allowed_origins=[])
    assert status == 200
    assert hit is True


async def test_origin_allowed_passes_through():
    """An Origin in the allow-list is accepted."""
    status, _, hit = await _drive_origin_middleware(
        [(b"origin", b"https://app.example.com")],
        allowed_origins=["https://app.example.com"],
    )
    assert status == 200
    assert hit is True


async def test_origin_disallowed_rejected_403():
    """An Origin not in the allow-list is rejected with 403."""
    status, body, hit = await _drive_origin_middleware(
        [(b"origin", b"https://attacker.example.com")],
        allowed_origins=["https://app.example.com"],
    )
    assert status == 403
    assert b"forbidden" in body
    assert hit is False


async def test_origin_with_empty_allowlist_rejects_browser():
    """With no allow-list configured, any Origin header is rejected."""
    status, _, hit = await _drive_origin_middleware(
        [(b"origin", b"https://anything.example.com")],
        allowed_origins=[],
    )
    assert status == 403
    assert hit is False


async def test_origin_lifespan_event_passes_through():
    """Lifespan events (startup/shutdown) bypass Origin checks."""
    from ghostdesk.server import _origin_middleware

    inner_called = {"hit": False}

    async def inner_app(scope, receive, send):
        inner_called["hit"] = True

    wrapped = _origin_middleware(inner_app, frozenset())
    await wrapped({"type": "lifespan"}, None, None)
    assert inner_called["hit"] is True


def test_parse_allowed_origins():
    """Comma-separated list parses into a frozenset, stripping blanks."""
    from ghostdesk.server import _parse_allowed_origins

    assert _parse_allowed_origins("") == frozenset()
    assert _parse_allowed_origins("https://a.com") == frozenset(["https://a.com"])
    assert _parse_allowed_origins("https://a.com, https://b.com  ,") == frozenset(
        ["https://a.com", "https://b.com"]
    )


# ---------------------------------------------------------------------------
# _model_space_middleware — per-request GhostDesk-Model-Space header
# ---------------------------------------------------------------------------


async def _capture_model_space_during_request(headers):
    """Drive the middleware once and return the model space seen inside."""
    import asyncio

    from ghostdesk._coords import get_model_space
    from ghostdesk.server import _model_space_middleware

    seen: dict = {}

    async def inner_app(scope, receive, send):
        # Yield so concurrent requests can interleave inside the middleware.
        await asyncio.sleep(0)
        seen["value"] = get_model_space()

    wrapped = _model_space_middleware(inner_app)
    scope = {"type": "http", "headers": headers}
    await wrapped(scope, None, None)
    return seen["value"]


async def test_model_space_header_sets_per_request_value():
    value = await _capture_model_space_during_request(
        [(b"ghostdesk-model-space", b"1000")]
    )
    assert value == 1000


async def test_model_space_header_absent_defaults_to_zero():
    value = await _capture_model_space_during_request([])
    assert value == 0


async def test_model_space_header_concurrent_requests_isolated():
    """Two concurrent requests with different headers must not see each other."""
    import asyncio

    a, b = await asyncio.gather(
        _capture_model_space_during_request([(b"ghostdesk-model-space", b"1000")]),
        _capture_model_space_during_request([(b"ghostdesk-model-space", b"512")]),
    )
    assert (a, b) == (1000, 512)


async def test_model_space_header_resets_after_request():
    """The ContextVar override must not leak past the request boundary."""
    from ghostdesk._coords import get_model_space

    await _capture_model_space_during_request([(b"ghostdesk-model-space", b"1000")])
    assert get_model_space() == 0


async def test_model_space_header_invalid_value_defaults_to_zero():
    value = await _capture_model_space_during_request(
        [(b"ghostdesk-model-space", b"not-a-number")]
    )
    assert value == 0
