# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk.resources.clipboard."""

from unittest.mock import AsyncMock, patch

from ghostdesk.resources.clipboard import clipboard_resource
from ghostdesk.server import create_app


async def test_clipboard_resource_returns_plain_text():
    """The clipboard resource forwards the raw clipboard_get() string."""
    with patch(
        "ghostdesk.resources.clipboard.clipboard_get",
        new_callable=AsyncMock,
        return_value="hello world",
    ):
        assert await clipboard_resource() == "hello world"


async def test_create_app_registers_clipboard_resource():
    """create_app() exposes the ghostdesk://clipboard URI."""
    app = create_app(port=9999)
    assert "ghostdesk://clipboard" in app._resource_manager._resources
