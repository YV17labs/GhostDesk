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

    async def fake_capture_png(region=None, scale=None):
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
    async def counting_capture_png(region=None, scale=None):
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
    async def changing_capture_png(region=None, scale=None):
        nonlocal call_count
        call_count += 1
        if call_count == 1:
            return changing
        return stable

    with patch(f"{CAPTURE}.capture_png", side_effect=changing_capture_png):
        result = await screen_shot(stabilize=True)
        assert call_count >= 2
        assert hasattr(result, "data")


async def test_screen_shot_quality_shrinks_payload(_mock_deps):
    """Lower quality produces smaller webp bytes than higher quality."""
    # Use a non-trivial image so quality actually matters.
    img = Image.new("RGB", (256, 256))
    for x in range(256):
        for y in range(256):
            img.putpixel((x, y), ((x * 7) % 256, (y * 11) % 256, ((x + y) * 13) % 256))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    noisy_png = buf.getvalue()

    async def fake_capture(region=None, scale=None):
        return noisy_png

    with patch(f"{CAPTURE}.capture_png", side_effect=fake_capture):
        hi = await screen_shot(stabilize=False, quality=95)
        lo = await screen_shot(stabilize=False, quality=20)
        assert len(lo.data) < len(hi.data)


async def test_screen_shot_invalid_quality_raises(_mock_deps):
    with pytest.raises(ValueError):
        await screen_shot(quality=0)
    with pytest.raises(ValueError):
        await screen_shot(quality=101)


async def test_screen_shot_invalid_scale_raises(_mock_deps):
    with pytest.raises(ValueError):
        await screen_shot(scale=0)
    with pytest.raises(ValueError):
        await screen_shot(scale=-1.0)


async def test_screen_shot_forwards_scale_to_capture_png():
    """The scale arg reaches grim via capture_png."""
    tiny_png = _make_tiny_png()
    seen = []

    async def recording_capture_png(region=None, scale=None):
        seen.append(scale)
        return tiny_png

    with patch(f"{CAPTURE}.capture_png", side_effect=recording_capture_png):
        await screen_shot(stabilize=False, scale=0.5)
        assert seen == [0.5]


def test_reencode_quality_affects_output():
    """_reencode honors the quality knob for webp."""
    img = Image.new("RGB", (128, 128))
    for x in range(128):
        for y in range(128):
            img.putpixel((x, y), ((x * 5) % 256, (y * 9) % 256, ((x ^ y) * 3) % 256))
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    src = buf.getvalue()
    hi = _reencode(src, "webp", quality=95)
    lo = _reencode(src, "webp", quality=20)
    assert len(lo) < len(hi)
