# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.tools.devices.screen."""

import struct
import tempfile
import zlib
from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.devices.screen import screenshot

MODULE = "ghostdesk.tools.devices.screen"


def _make_tiny_png() -> bytes:
    """Create a minimal valid 1x1 white PNG."""
    # IHDR
    ihdr_data = struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0)
    ihdr_crc = struct.pack(">I", zlib.crc32(b"IHDR" + ihdr_data) & 0xFFFFFFFF)
    ihdr = struct.pack(">I", 13) + b"IHDR" + ihdr_data + ihdr_crc
    # IDAT
    raw = zlib.compress(b"\x00\xff\xff\xff")
    idat_crc = struct.pack(">I", zlib.crc32(b"IDAT" + raw) & 0xFFFFFFFF)
    idat = struct.pack(">I", len(raw)) + b"IDAT" + raw + idat_crc
    # IEND
    iend_crc = struct.pack(">I", zlib.crc32(b"IEND") & 0xFFFFFFFF)
    iend = struct.pack(">I", 0) + b"IEND" + iend_crc
    return b"\x89PNG\r\n\x1a\n" + ihdr + idat + iend


@pytest.fixture()
def tiny_png(tmp_path):
    """Write a tiny PNG to a temp file and return its bytes."""
    return _make_tiny_png()


@pytest.fixture(autouse=True)
def _mock_deps(tiny_png):
    """Patch all external dependencies of screen module."""
    async def fake_run(cmd):
        """Write a tiny PNG when maim is called; return geometry for xdotool."""
        if cmd[0] == "maim":
            path = cmd[-1]
            with open(path, "wb") as f:
                f.write(tiny_png)
            return ""
        if cmd[0] == "xdotool":
            return "1920 1080"
        return ""

    with (
        patch(f"{MODULE}.run", side_effect=fake_run) as mock_run,
        patch(f"{MODULE}.get_cursor_position", new_callable=AsyncMock, return_value=(100, 200)) as mock_pos,
        patch(f"{MODULE}.get_clickables", new_callable=AsyncMock, return_value=[]) as mock_click,
        patch(f"{MODULE}.draw_cursor", return_value=tiny_png) as mock_draw,
    ):
        yield mock_run, mock_pos, mock_click, mock_draw


# --- screenshot ---

async def test_screenshot_full(_mock_deps):
    mock_run, mock_pos, mock_click, mock_draw = _mock_deps
    result = await screenshot()
    assert isinstance(result, list)
    assert len(result) == 2
    meta = result[1]
    assert meta["cursor"] == {"x": 100, "y": 200}
    assert "apps" in meta
    mock_draw.assert_called_once()


async def test_screenshot_region(_mock_deps):
    mock_run, mock_pos, mock_click, mock_draw = _mock_deps
    result = await screenshot(x=10, y=20, width=300, height=400)
    assert isinstance(result, list)
    # cursor coords should be adjusted for region offset
    meta = result[1]
    assert meta["cursor"] == {"x": 90, "y": 180}  # 100-10, 200-20
    # draw_cursor should receive adjusted coords + format kwargs
    mock_draw.assert_called_once_with(
        _make_tiny_png(), 90, 180,
        output_format="png", quality=80,
    )


async def test_screenshot_partial_region_treated_as_fullscreen(_mock_deps):
    """If only some region args are given, it should be full screen."""
    _, _, _, mock_draw = _mock_deps
    result = await screenshot(x=10, y=20)  # width/height missing
    meta = result[1]
    # No offset adjustment — treated as full screen
    assert meta["cursor"] == {"x": 100, "y": 200}


async def test_screenshot_cleans_up_tempfile(_mock_deps):
    """The temp PNG should be deleted after screenshot."""
    import os

    # Just verify no leftover tempfiles — the function uses try/finally
    before = set(os.listdir(tempfile.gettempdir()))
    await screenshot()
    after = set(os.listdir(tempfile.gettempdir()))
    # No new .png files should remain
    new_pngs = {f for f in (after - before) if f.endswith(".png")}
    assert len(new_pngs) == 0


async def test_screenshot_default_format_is_png(_mock_deps):
    _, _, _, mock_draw = _mock_deps
    await screenshot()
    mock_draw.assert_called_once()
    _, kwargs = mock_draw.call_args
    assert kwargs.get("output_format") == "png"
    assert kwargs.get("quality") == 80


async def test_screenshot_webp_format(_mock_deps):
    _, _, _, mock_draw = _mock_deps
    await screenshot(output_format="webp")
    _, kwargs = mock_draw.call_args
    assert kwargs.get("output_format") == "webp"


async def test_screenshot_custom_quality(_mock_deps):
    _, _, _, mock_draw = _mock_deps
    await screenshot(quality=50)
    _, kwargs = mock_draw.call_args
    assert kwargs.get("quality") == 50

