# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk._lifespan — FastMCP lifespan Wayland warm-up."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk._lifespan import lifespan
from ghostdesk.server import create_app


async def test_lifespan_warms_up_wayland_on_enter():
    """Entering the lifespan binds the Wayland input singleton before yielding."""
    with patch("ghostdesk._lifespan.get_wayland_input", new_callable=AsyncMock) as mock_get:
        async with lifespan(object()) as ctx:
            mock_get.assert_awaited_once()
            assert ctx == {}


async def test_lifespan_propagates_warm_up_failure():
    """Warm-up failure propagates out of the lifespan."""
    class _BoomError(RuntimeError):
        pass

    with patch(
        "ghostdesk._lifespan.get_wayland_input",
        new_callable=AsyncMock,
        side_effect=_BoomError("missing zwlr_virtual_pointer_manager_v1"),
    ):
        with pytest.raises(_BoomError):
            async with lifespan(object()):
                pytest.fail("lifespan should not reach the yield")


async def test_create_app_wires_lifespan():
    """create_app() wires the lifespan into FastMCP."""
    app = create_app(port=9999)
    assert app.settings.lifespan is lifespan
