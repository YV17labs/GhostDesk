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


def _make_tiny_png(color_value: int = 0xFF) -> bytes:
    """Create a minimal valid 1x1 PNG with specified color."""
    ihdr_data = struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0)
    ihdr_crc = struct.pack(">I", zlib.crc32(b"IHDR" + ihdr_data) & 0xFFFFFFFF)
    ihdr = struct.pack(">I", 13) + b"IHDR" + ihdr_data + ihdr_crc
    # Create RGB pixel data with specified color value
    raw = zlib.compress(bytes([0, color_value, color_value, color_value]))
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


async def test_screenshot_default_format_is_webp(_mock_deps):
    result = await screenshot()
    assert result[0]._format == "webp"


async def test_screenshot_webp_format(_mock_deps):
    result = await screenshot(format="webp")
    assert result[0]._format == "webp"


def test_reencode_webp_format():
    """Test _reencode function with webp format (exercises lines 77-80)."""
    from ghostdesk.screen.capture import _reencode

    tiny_png = _make_tiny_png()
    result = _reencode(tiny_png, "webp")

    # Verify the result is valid WebP (starts with "RIFF" signature)
    assert result[:4] == b"RIFF", "Invalid WebP header"
    # Verify it's larger than just headers
    assert len(result) > 12


def test_reencode_png_format():
    """Test _reencode function with png format (returns unchanged)."""
    from ghostdesk.screen.capture import _reencode

    tiny_png = _make_tiny_png()
    result = _reencode(tiny_png, "png")

    # PNG format should return the original bytes unchanged
    assert result == tiny_png


async def test_screenshot_stabilize_disabled():
    """Test that stabilize=False skips stabilization check."""
    tiny_png = _make_tiny_png()

    # Mock capture_png to track call count
    call_count = 0
    async def counting_capture_png(region=None):
        nonlocal call_count
        call_count += 1
        return tiny_png

    with (
        patch(f"{CAPTURE}.capture_png", side_effect=counting_capture_png),
        patch(f"{CAPTURE}.get_cursor_position", new_callable=AsyncMock, return_value=(100, 200)),
        patch(f"{CAPTURE}.get_open_windows", new_callable=AsyncMock, return_value=[]),
    ):
        result = await screenshot(stabilize=False)
        # With stabilize=False, capture_png should only be called once
        assert call_count == 1
        assert isinstance(result, list)
        assert len(result) == 2


async def test_screenshot_stabilize_enabled_detects_change():
    """Test that stabilize=True detects when page is unstable."""
    stable_png = _make_tiny_png(color_value=0xFF)  # White
    changing_png = _make_tiny_png(color_value=0x80)  # Gray

    call_count = 0
    async def changing_capture_png(region=None):
        nonlocal call_count
        call_count += 1
        # First call returns changing PNG, second and onwards return stable PNG
        if call_count == 1:
            return changing_png
        return stable_png

    with (
        patch(f"{CAPTURE}.capture_png", side_effect=changing_capture_png),
        patch(f"{CAPTURE}.get_cursor_position", new_callable=AsyncMock, return_value=(100, 200)),
        patch(f"{CAPTURE}.get_open_windows", new_callable=AsyncMock, return_value=[]),
    ):
        result = await screenshot(stabilize=True)
        # Should have captured at least twice (first different, then stable)
        assert call_count >= 2
        assert isinstance(result, list)
        assert len(result) == 2
