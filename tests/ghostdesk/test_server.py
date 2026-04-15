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


def test_main_calls_create_app_and_runs():
    """main() calls configure_logging(), create_app(), and runs the server."""
    from ghostdesk.server import main

    with (
        patch("ghostdesk.server.configure_logging") as mock_logging,
        patch("ghostdesk.server.create_app") as mock_create,
    ):
        # Mock the app and its run method
        mock_app = mock_create.return_value
        mock_app.run = MagicMock()

        main()

        # Verify configure_logging was called
        mock_logging.assert_called_once()
        # Verify create_app was called with no arguments
        mock_create.assert_called_once_with()
        # Verify run was called with correct transport
        mock_app.run.assert_called_once_with(transport="streamable-http")


def test_main_with_port_env_var():
    """main() respects GHOSTDESK_PORT env var when calling create_app."""
    from ghostdesk.server import main

    with (
        patch("ghostdesk.server.configure_logging"),
        patch("ghostdesk.server.create_app") as mock_create,
        patch.dict("os.environ", {"GHOSTDESK_PORT": "5000"}),
    ):
        mock_app = mock_create.return_value
        mock_app.run = MagicMock()

        main()

        # create_app is called with no arguments, but GHOSTDESK_PORT env var
        # affects the port used internally in create_app
        mock_create.assert_called_once_with()
        mock_app.run.assert_called_once_with(transport="streamable-http")


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
