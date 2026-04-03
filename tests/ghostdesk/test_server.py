# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.server — MCP server entry point."""

import importlib
import sys
from unittest.mock import patch

import pytest

from ghostdesk.server import create_app


async def test_create_app_returns_fastmcp():
    """create_app() returns a FastMCP instance."""
    from mcp.server.fastmcp import FastMCP

    app = create_app(port=9999)

    assert isinstance(app, FastMCP)


async def test_create_app_has_11_tools():
    """create_app() registers exactly 11 tools."""
    app = create_app(port=9999)

    tools = app._tool_manager._tools
    assert len(tools) == 11, f"Expected 11 tools, got {len(tools)}: {sorted(tools.keys())}"


async def test_create_app_custom_port():
    """create_app() accepts a custom port."""
    app = create_app(port=4567)

    assert app.settings.port == 4567


async def test_create_app_default_port_from_env():
    """create_app() reads PORT from environment when no port is given."""
    with patch.dict("os.environ", {"PORT": "8080"}):
        app = create_app()

    assert app.settings.port == 8080


async def test_create_app_default_port_fallback():
    """create_app() defaults to port 3000 when PORT env var is unset."""
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
