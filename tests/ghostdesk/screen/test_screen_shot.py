# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk.screen.screen_shot."""

import io
from unittest.mock import patch

import pytest
from PIL import Image

from ghostdesk.screen._shared import Region
from ghostdesk.screen.screen_shot import _reencode, screen_shot

CAPTURE = "ghostdesk.screen.screen_shot"


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

    with patch(f"{CAPTURE}.capture_png", side_effect=fake_capture_png):
        yield


async def test_screen_shot_returns_image(_mock_deps):
    result = await screen_shot()
    assert hasattr(result, "data")


async def test_screen_shot_with_region(_mock_deps):
    result = await screen_shot(region=Region(10, 20, 300, 400))
    assert hasattr(result, "data")


async def test_screen_shot_default_format_is_webp(_mock_deps):
    result = await screen_shot()
    assert result._format == "webp"


async def test_screen_shot_png_format(_mock_deps):
    result = await screen_shot(format="png")
    assert result._format == "png"


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


async def test_screen_shot_stabilize_disabled():
    """stabilize=False calls capture_png exactly once."""
    tiny_png = _make_tiny_png()

    call_count = 0
    async def counting_capture_png(region=None):
        nonlocal call_count
        call_count += 1
        return tiny_png

    with patch(f"{CAPTURE}.capture_png", side_effect=counting_capture_png):
        result = await screen_shot(stabilize=False)
        assert call_count == 1
        assert hasattr(result, "data")


async def test_screen_shot_stabilize_waits_for_stable_frame():
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

    with patch(f"{CAPTURE}.capture_png", side_effect=changing_capture_png):
        result = await screen_shot(stabilize=True)
        assert call_count >= 2
        assert hasattr(result, "data")
