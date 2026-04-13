# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.screen.capture."""

import io
from unittest.mock import AsyncMock, patch

import pytest
from PIL import Image

from ghostdesk.screen._shared import Region
from ghostdesk.screen.capture import _reencode, screenshot

CAPTURE = "ghostdesk.screen.capture"


def _make_tiny_png(color: tuple[int, int, int] = (255, 255, 255)) -> bytes:
    """Minimal valid 1x1 PNG."""
    img = Image.new("RGB", (1, 1), color)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


@pytest.fixture(autouse=True)
def _mock_deps():
    """Patch all external dependencies of the capture module."""
    tiny_png = _make_tiny_png()

    async def fake_capture_png(region=None):
        return tiny_png

    with (
        patch(f"{CAPTURE}.capture_png", side_effect=fake_capture_png),
        patch(f"{CAPTURE}.get_cursor_position", return_value=(100, 200)),
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


async def test_screenshot_default_format_is_webp(_mock_deps):
    result = await screenshot()
    assert result[0]._format == "webp"


async def test_screenshot_png_format(_mock_deps):
    result = await screenshot(format="png")
    assert result[0]._format == "png"


def test_reencode_webp_format():
    """_reencode with webp re-encodes the PNG bytes."""
    tiny_png = _make_tiny_png()
    result = _reencode(tiny_png, "webp")
    assert result[:4] == b"RIFF"


def test_reencode_png_format_passthrough():
    """_reencode with png returns the original bytes unchanged."""
    tiny_png = _make_tiny_png()
    result = _reencode(tiny_png, "png")
    assert result == tiny_png


async def test_screenshot_stabilize_disabled():
    """stabilize=False calls capture_png exactly once."""
    tiny_png = _make_tiny_png()

    call_count = 0
    async def counting_capture_png(region=None):
        nonlocal call_count
        call_count += 1
        return tiny_png

    with (
        patch(f"{CAPTURE}.capture_png", side_effect=counting_capture_png),
        patch(f"{CAPTURE}.get_cursor_position", return_value=(100, 200)),
        patch(f"{CAPTURE}.get_open_windows", new_callable=AsyncMock, return_value=[]),
    ):
        result = await screenshot(stabilize=False)
        assert call_count == 1
        assert len(result) == 2


async def test_screenshot_stabilize_waits_for_stable_frame():
    """stabilize=True keeps polling until two captures match."""
    stable = _make_tiny_png((255, 255, 255))
    changing = _make_tiny_png((0, 0, 0))

    call_count = 0
    async def changing_capture_png(region=None):
        nonlocal call_count
        call_count += 1
        if call_count == 1:
            return changing
        return stable

    with (
        patch(f"{CAPTURE}.capture_png", side_effect=changing_capture_png),
        patch(f"{CAPTURE}.get_cursor_position", return_value=(100, 200)),
        patch(f"{CAPTURE}.get_open_windows", new_callable=AsyncMock, return_value=[]),
    ):
        result = await screenshot(stabilize=True)
        assert call_count >= 2
        assert len(result) == 2
