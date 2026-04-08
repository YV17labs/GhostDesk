# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.screen.capture."""

import struct
import zlib
from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.screen._shared import Region
from ghostdesk.screen.capture import screenshot

SHARED = "ghostdesk.screen._shared"
CAPTURE = "ghostdesk.screen.capture"


def _make_tiny_png() -> bytes:
    """Create a minimal valid 1x1 white PNG."""
    ihdr_data = struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0)
    ihdr_crc = struct.pack(">I", zlib.crc32(b"IHDR" + ihdr_data) & 0xFFFFFFFF)
    ihdr = struct.pack(">I", 13) + b"IHDR" + ihdr_data + ihdr_crc
    raw = zlib.compress(b"\x00\xff\xff\xff")
    idat_crc = struct.pack(">I", zlib.crc32(b"IDAT" + raw) & 0xFFFFFFFF)
    idat = struct.pack(">I", len(raw)) + b"IDAT" + raw + idat_crc
    iend_crc = struct.pack(">I", zlib.crc32(b"IEND") & 0xFFFFFFFF)
    iend = struct.pack(">I", 0) + b"IEND" + iend_crc
    return b"\x89PNG\r\n\x1a\n" + ihdr + idat + iend


@pytest.fixture(autouse=True)
def _mock_deps():
    """Patch all external dependencies of screen module."""
    tiny_png = _make_tiny_png()

    async def fake_capture_png(region=None):
        return tiny_png

    with (
        patch(f"{CAPTURE}.capture_png", side_effect=fake_capture_png),
        patch(f"{CAPTURE}.get_cursor_position", new_callable=AsyncMock, return_value=(100, 200)),
        patch(f"{CAPTURE}.get_open_windows", new_callable=AsyncMock, return_value=[]),
    ):
        yield


async def test_screenshot_returns_image_and_metadata(_mock_deps):
    result = await screenshot()
    assert isinstance(result, list)
    assert len(result) == 2
    assert hasattr(result[0], "data")
    meta = result[1]
    assert "screen" in meta
    assert "cursor" in meta
    assert "windows" in meta
    assert "region" in meta
    assert meta["cursor"] == {"x": 100, "y": 200}


async def test_screenshot_with_region(_mock_deps):
    result = await screenshot(region=Region(10, 20, 300, 400))
    assert isinstance(result, list)
    assert len(result) == 2
    assert hasattr(result[0], "data")


async def test_screenshot_default_format_is_png(_mock_deps):
    result = await screenshot()
    assert result[0]._format == "png"


async def test_screenshot_webp_format(_mock_deps):
    result = await screenshot(format="webp")
    assert result[0]._format == "webp"
