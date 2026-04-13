# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
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
