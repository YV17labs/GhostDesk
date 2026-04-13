# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.screen._shared utilities."""

import io
import struct
import zlib
from unittest.mock import AsyncMock, patch

import pytest
from PIL import Image

from ghostdesk.screen._shared import (
    Region,
    build_metadata,
    capture_png,
    save_image_bytes,
    screens_stable,
)

SHARED = "ghostdesk.screen._shared"


def _create_png_with_text_metadata(width: int, height: int, color: tuple[int, int, int], text: str) -> bytes:
    """Create a PNG with a tEXt metadata chunk (different bytes, identical pixels)."""
    img = Image.new("RGB", (width, height), color)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    png_bytes = buf.getvalue()

    iend_pos = len(png_bytes) - 12
    text_data = f"test\x00{text}".encode("latin-1")
    text_chunk = struct.pack(">I", len(text_data)) + b"tEXt" + text_data
    text_crc = zlib.crc32(b"tEXt" + text_data) & 0xFFFFFFFF
    text_chunk += struct.pack(">I", text_crc)
    return png_bytes[:iend_pos] + text_chunk + png_bytes[iend_pos:]


def _make_tiny_png(color: tuple[int, int, int] = (255, 255, 255)) -> bytes:
    """Minimal valid 1x1 PNG."""
    img = Image.new("RGB", (1, 1), color)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


def _make_png_with_size(width: int, height: int, color: tuple[int, int, int] = (255, 255, 255)) -> bytes:
    """Valid PNG of the requested size."""
    img = Image.new("RGB", (width, height), color)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestSaveImageBytes:
    """Tests for save_image_bytes function."""

    def test_save_image_bytes_webp_format(self):
        """WebP encoding with RGB conversion."""
        img = Image.new("RGBA", (10, 10), (255, 0, 0, 255))
        result = save_image_bytes(img, fmt="webp")
        assert result[:4] == b"RIFF"
        loaded = Image.open(io.BytesIO(result))
        assert loaded.format == "WEBP"
        assert loaded.mode == "RGB"

    def test_save_image_bytes_png_format(self):
        """PNG encoding without conversion."""
        img = Image.new("RGB", (10, 10), (0, 255, 0))
        result = save_image_bytes(img, fmt="png")
        assert result[:8] == b"\x89PNG\r\n\x1a\n"
        loaded = Image.open(io.BytesIO(result))
        assert loaded.format == "PNG"

    def test_save_image_bytes_png_default_format(self):
        """Default format is PNG."""
        img = Image.new("RGB", (5, 5), (100, 100, 100))
        result = save_image_bytes(img)
        assert result[:8] == b"\x89PNG\r\n\x1a\n"

    def test_save_image_bytes_preserves_dimensions(self):
        """Encoded images keep their original dimensions."""
        width, height = 42, 24
        img = Image.new("RGB", (width, height), (128, 128, 128))

        webp_bytes = save_image_bytes(img, fmt="webp")
        assert Image.open(io.BytesIO(webp_bytes)).size == (width, height)

        png_bytes = save_image_bytes(img, fmt="png")
        assert Image.open(io.BytesIO(png_bytes)).size == (width, height)


class TestScreensStable:
    """Tests for screens_stable function."""

    def test_screens_stable_identical_bytes(self):
        png_bytes = _make_tiny_png()
        assert screens_stable(png_bytes, png_bytes) is True

    def test_screens_stable_different_sizes(self):
        img1 = _make_png_with_size(10, 10)
        img2 = _make_png_with_size(20, 20)
        assert screens_stable(img1, img2) is False

    def test_screens_stable_no_diff(self):
        """Same pixels, different bytes (via tEXt metadata)."""
        png1 = _create_png_with_text_metadata(5, 5, (100, 100, 100), "v1")
        png2 = _create_png_with_text_metadata(5, 5, (100, 100, 100), "v2")
        assert png1 != png2
        assert screens_stable(png1, png2) is True

    def test_screens_stable_small_diff(self):
        """1-pixel diff in a 100x100 image is below the 0.005 threshold."""
        img1 = Image.new("RGB", (100, 100), (255, 255, 255))
        img2 = Image.new("RGB", (100, 100), (255, 255, 255))
        img2.load()[0, 0] = (0, 0, 0)

        b1 = io.BytesIO(); img1.save(b1, format="PNG")
        b2 = io.BytesIO(); img2.save(b2, format="PNG")
        assert screens_stable(b1.getvalue(), b2.getvalue()) is True

    def test_screens_stable_large_diff(self):
        """10x10 diff in a 100x100 image exceeds the threshold."""
        img1 = Image.new("RGB", (100, 100), (255, 255, 255))
        img2 = Image.new("RGB", (100, 100), (255, 255, 255))
        pixels2 = img2.load()
        for x in range(10):
            for y in range(10):
                pixels2[x, y] = (0, 0, 0)

        b1 = io.BytesIO(); img1.save(b1, format="PNG")
        b2 = io.BytesIO(); img2.save(b2, format="PNG")
        assert screens_stable(b1.getvalue(), b2.getvalue()) is False


class TestCapturePng:
    """Tests for capture_png — single grim invocation."""

    @pytest.mark.asyncio
    async def test_capture_png_success_without_region(self):
        """grim is invoked with no -g flag when region is None."""
        expected = _make_tiny_png()

        with patch(
            f"{SHARED}.asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
        ) as mock_exec:
            mock_proc = AsyncMock()
            mock_proc.communicate.return_value = (expected, b"")
            mock_proc.returncode = 0
            mock_exec.return_value = mock_proc

            result = await capture_png(None)

            assert result == expected
            args = mock_exec.call_args[0]
            assert args == ("grim", "-t", "png", "-")

    @pytest.mark.asyncio
    async def test_capture_png_success_with_region(self):
        """grim is invoked with ``-g 'X,Y WxH'`` geometry."""
        expected = _make_tiny_png()
        region = Region(x=10, y=20, width=300, height=400)

        with patch(
            f"{SHARED}.asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
        ) as mock_exec:
            mock_proc = AsyncMock()
            mock_proc.communicate.return_value = (expected, b"")
            mock_proc.returncode = 0
            mock_exec.return_value = mock_proc

            result = await capture_png(region)

            assert result == expected
            args = mock_exec.call_args[0]
            assert args == ("grim", "-t", "png", "-g", "10,20 300x400", "-")

    @pytest.mark.asyncio
    async def test_capture_png_failure_with_error_message(self):
        """Non-zero returncode surfaces stderr as RuntimeError."""
        error_msg = "Error: output not found"

        with patch(
            f"{SHARED}.asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
        ) as mock_exec:
            mock_proc = AsyncMock()
            mock_proc.communicate.return_value = (b"", error_msg.encode())
            mock_proc.returncode = 1
            mock_exec.return_value = mock_proc

            with pytest.raises(RuntimeError) as exc_info:
                await capture_png(None)
            assert str(exc_info.value) == error_msg

    @pytest.mark.asyncio
    async def test_capture_png_failure_empty_stderr(self):
        """Empty stderr falls back to a default message."""
        with patch(
            f"{SHARED}.asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
        ) as mock_exec:
            mock_proc = AsyncMock()
            mock_proc.communicate.return_value = (b"", b"")
            mock_proc.returncode = 1
            mock_exec.return_value = mock_proc

            with pytest.raises(RuntimeError) as exc_info:
                await capture_png(None)
            assert str(exc_info.value) == "grim failed"


class TestBuildMetadata:
    """Tests for build_metadata function."""

    def test_build_metadata_without_region(self):
        result = build_metadata(cx=100, cy=200, windows=[])
        assert result["screen"]["width"] == 1280
        assert result["screen"]["height"] == 1024
        assert result["region"] == {"x": 0, "y": 0, "width": 1280, "height": 1024}
        assert result["cursor"] == {"x": 100, "y": 200}
        assert result["windows"] == []

    def test_build_metadata_with_region(self):
        region = Region(x=50, y=75, width=640, height=512)
        result = build_metadata(cx=300, cy=400, windows=[], region=region)
        assert result["region"] == {"x": 50, "y": 75, "width": 640, "height": 512}
        assert result["cursor"] == {"x": 300, "y": 400}

    def test_build_metadata_with_windows(self):
        windows = [
            {"app": "foot", "title": "term", "x": 0, "y": 0, "width": 640, "height": 480},
        ]
        result = build_metadata(cx=100, cy=100, windows=windows)
        assert result["windows"] == windows


class TestRegion:
    """Tests for Region dataclass."""

    def test_region_creation(self):
        region = Region(x=10, y=20, width=100, height=200)
        assert (region.x, region.y, region.width, region.height) == (10, 20, 100, 200)

    def test_region_equality(self):
        r1 = Region(x=10, y=20, width=100, height=200)
        r2 = Region(x=10, y=20, width=100, height=200)
        r3 = Region(x=15, y=20, width=100, height=200)
        assert r1 == r2
        assert r1 != r3
