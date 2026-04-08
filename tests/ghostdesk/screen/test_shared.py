# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.screen._shared utilities."""

import io
import struct
import zlib
from unittest.mock import AsyncMock, patch

import pytest
from PIL import Image, ImageChops

from ghostdesk.screen._shared import (
    Region,
    _maim,
    _screens_stable,
    build_metadata,
    save_image_bytes,
)

SHARED = "ghostdesk.screen._shared"


def _create_png_with_text_metadata(width: int, height: int, color: tuple[int, int, int], text: str) -> bytes:
    """Create a PNG with a tEXt metadata chunk.

    This creates different byte contents while maintaining identical pixel data,
    useful for testing image comparison logic.
    """
    img = Image.new("RGB", (width, height), color)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    png_bytes = buf.getvalue()

    # Find IEND position (last 12 bytes)
    iend_pos = len(png_bytes) - 12

    # Create tEXt chunk: keyword\0text
    text_data = f"test\x00{text}".encode("latin-1")
    text_chunk = struct.pack(">I", len(text_data))  # length
    text_chunk += b"tEXt"  # type
    text_chunk += text_data
    # Calculate CRC
    text_crc = zlib.crc32(b"tEXt" + text_data) & 0xFFFFFFFF
    text_chunk += struct.pack(">I", text_crc)

    # Insert before IEND
    return png_bytes[:iend_pos] + text_chunk + png_bytes[iend_pos:]


def _make_tiny_png(color: tuple[int, int, int] = (255, 255, 255)) -> bytes:
    """Create a minimal valid 1x1 PNG with specified color.

    Args:
        color: RGB tuple for the pixel color (default white).

    Returns:
        Valid PNG bytes for a 1x1 image.
    """
    r, g, b = color
    ihdr_data = struct.pack(">IIBBBBB", 1, 1, 8, 2, 0, 0, 0)
    ihdr_crc = struct.pack(">I", zlib.crc32(b"IHDR" + ihdr_data) & 0xFFFFFFFF)
    ihdr = struct.pack(">I", 13) + b"IHDR" + ihdr_data + ihdr_crc
    raw = zlib.compress(bytes([0, r, g, b]))
    idat_crc = struct.pack(">I", zlib.crc32(b"IDAT" + raw) & 0xFFFFFFFF)
    idat = struct.pack(">I", len(raw)) + b"IDAT" + raw + idat_crc
    iend_crc = struct.pack(">I", zlib.crc32(b"IEND") & 0xFFFFFFFF)
    iend = struct.pack(">I", 0) + b"IEND" + iend_crc
    return b"\x89PNG\r\n\x1a\n" + ihdr + idat + iend


def _make_png_with_size(width: int, height: int, color: tuple[int, int, int] = (255, 255, 255)) -> bytes:
    """Create a valid PNG with specified dimensions.

    Args:
        width: Image width in pixels.
        height: Image height in pixels.
        color: RGB tuple for the pixel color (default white).

    Returns:
        Valid PNG bytes for an image of specified size.
    """
    img = Image.new("RGB", (width, height), color)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestSaveImageBytes:
    """Tests for save_image_bytes function."""

    def test_save_image_bytes_webp_format(self):
        """Test WebP encoding with RGB conversion and method=4."""
        # Create a test image (RGBA to exercise convert)
        img = Image.new("RGBA", (10, 10), (255, 0, 0, 255))

        result = save_image_bytes(img, fmt="webp")

        # Verify result is valid WebP (RIFF signature)
        assert result[:4] == b"RIFF", "Invalid WebP header"
        assert len(result) > 12

        # Verify the saved image can be loaded
        loaded = Image.open(io.BytesIO(result))
        assert loaded.format == "WEBP"
        assert loaded.mode == "RGB"  # WebP conversion to RGB worked

    def test_save_image_bytes_png_format(self):
        """Test PNG encoding without RGB conversion."""
        # Create test image in RGB
        img = Image.new("RGB", (10, 10), (0, 255, 0))

        result = save_image_bytes(img, fmt="png")

        # Verify result is valid PNG
        assert result[:8] == b"\x89PNG\r\n\x1a\n", "Invalid PNG header"

        # Verify the saved image can be loaded
        loaded = Image.open(io.BytesIO(result))
        assert loaded.format == "PNG"
        assert loaded.mode == "RGB"

    def test_save_image_bytes_png_default_format(self):
        """Test that PNG is the default format when not specified."""
        img = Image.new("RGB", (5, 5), (100, 100, 100))

        result = save_image_bytes(img)

        assert result[:8] == b"\x89PNG\r\n\x1a\n", "Default format should be PNG"

    def test_save_image_bytes_preserves_dimensions(self):
        """Test that encoded images preserve original dimensions."""
        width, height = 42, 24
        img = Image.new("RGB", (width, height), (128, 128, 128))

        # Test WebP
        webp_bytes = save_image_bytes(img, fmt="webp")
        webp_img = Image.open(io.BytesIO(webp_bytes))
        assert webp_img.size == (width, height)

        # Test PNG
        png_bytes = save_image_bytes(img, fmt="png")
        png_img = Image.open(io.BytesIO(png_bytes))
        assert png_img.size == (width, height)


class TestScreensStable:
    """Tests for _screens_stable function."""

    def test_screens_stable_identical_bytes(self):
        """Test when byte content is identical (line 79-80)."""
        png_bytes = _make_tiny_png()

        assert _screens_stable(png_bytes, png_bytes) is True

    def test_screens_stable_different_sizes(self):
        """Test when images have different dimensions (line 83-84)."""
        img1 = _make_png_with_size(10, 10)
        img2 = _make_png_with_size(20, 20)

        assert _screens_stable(img1, img2) is False

    def test_screens_stable_no_diff(self):
        """Test when ImageChops.difference returns None (line 86-87)."""
        # Create two PNG bytes with identical pixel content but different metadata
        # This bypasses the early return at line 79-80
        png1 = _create_png_with_text_metadata(5, 5, (100, 100, 100), "v1")
        png2 = _create_png_with_text_metadata(5, 5, (100, 100, 100), "v2")

        # Verify bytes are different
        assert png1 != png2

        # When pixels are identical, getbbox() should return None
        result = _screens_stable(png1, png2)

        assert result is True

    def test_screens_stable_small_diff(self):
        """Test when difference area is below threshold (line 91-92)."""
        # Create a 100x100 image (10000 pixels total)
        # Threshold is 0.005, so max diff area is 50 pixels
        # Create a 1x1 diff (1 pixel, well below threshold)
        img1 = Image.new("RGB", (100, 100), (255, 255, 255))
        img2 = Image.new("RGB", (100, 100), (255, 255, 255))

        # Manually modify one pixel in img2
        pixels2 = img2.load()
        pixels2[0, 0] = (0, 0, 0)

        buf1 = io.BytesIO()
        buf2 = io.BytesIO()
        img1.save(buf1, format="PNG")
        img2.save(buf2, format="PNG")

        assert _screens_stable(buf1.getvalue(), buf2.getvalue()) is True

    def test_screens_stable_large_diff(self):
        """Test when difference area exceeds threshold."""
        # Create a 100x100 image (10000 pixels total)
        # Threshold is 0.005, so max diff area is 50 pixels
        # Create a 10x10 diff (100 pixels, above threshold)
        img1 = Image.new("RGB", (100, 100), (255, 255, 255))
        img2 = Image.new("RGB", (100, 100), (255, 255, 255))

        # Modify a 10x10 area in img2
        pixels2 = img2.load()
        for x in range(10):
            for y in range(10):
                pixels2[x, y] = (0, 0, 0)

        buf1 = io.BytesIO()
        buf2 = io.BytesIO()
        img1.save(buf1, format="PNG")
        img2.save(buf2, format="PNG")

        assert _screens_stable(buf1.getvalue(), buf2.getvalue()) is False

    def test_screens_stable_edge_case_at_threshold(self):
        """Test difference area exactly at the stability threshold."""
        # 1000x1000 image = 1,000,000 pixels
        # Threshold 0.005 = 5,000 pixels max diff
        # Create a diff of exactly 50x100 = 5,000 pixels
        img1 = Image.new("RGB", (1000, 1000), (255, 255, 255))
        img2 = Image.new("RGB", (1000, 1000), (255, 255, 255))

        pixels2 = img2.load()
        for x in range(50):
            for y in range(100):
                pixels2[x, y] = (0, 0, 0)

        buf1 = io.BytesIO()
        buf2 = io.BytesIO()
        img1.save(buf1, format="PNG")
        img2.save(buf2, format="PNG")

        # At threshold, should return False (strictly less than)
        assert _screens_stable(buf1.getvalue(), buf2.getvalue()) is False


class TestMaim:
    """Tests for _maim function."""

    @pytest.mark.asyncio
    async def test_maim_success_without_region(self):
        """Test successful maim execution without region."""
        expected_stdout = _make_tiny_png()

        with patch(
            f"{SHARED}.asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
        ) as mock_exec:
            mock_proc = AsyncMock()
            mock_proc.communicate.return_value = (expected_stdout, b"")
            mock_proc.returncode = 0
            mock_exec.return_value = mock_proc

            result = await _maim(None)

            assert result == expected_stdout
            mock_exec.assert_called_once()
            args = mock_exec.call_args[0]
            assert args == ("maim", "--format=png")

    @pytest.mark.asyncio
    async def test_maim_success_with_region(self):
        """Test successful maim execution with region."""
        expected_stdout = _make_tiny_png()
        region = Region(x=10, y=20, width=300, height=400)

        with patch(
            f"{SHARED}.asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
        ) as mock_exec:
            mock_proc = AsyncMock()
            mock_proc.communicate.return_value = (expected_stdout, b"")
            mock_proc.returncode = 0
            mock_exec.return_value = mock_proc

            result = await _maim(region)

            assert result == expected_stdout
            args = mock_exec.call_args[0]
            assert args == ("maim", "--format=png", "-g", "300x400+10+20")

    @pytest.mark.asyncio
    async def test_maim_failure_with_error_message(self):
        """Test maim failure when returncode is non-zero."""
        error_msg = "Error: display not available"

        with patch(
            f"{SHARED}.asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
        ) as mock_exec:
            mock_proc = AsyncMock()
            mock_proc.communicate.return_value = (b"", error_msg.encode())
            mock_proc.returncode = 1
            mock_exec.return_value = mock_proc

            with pytest.raises(RuntimeError) as exc_info:
                await _maim(None)

            assert str(exc_info.value) == error_msg

    @pytest.mark.asyncio
    async def test_maim_failure_empty_stderr(self):
        """Test maim failure with empty stderr (line 68)."""
        with patch(
            f"{SHARED}.asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
        ) as mock_exec:
            mock_proc = AsyncMock()
            mock_proc.communicate.return_value = (b"", b"")
            mock_proc.returncode = 1
            mock_exec.return_value = mock_proc

            with pytest.raises(RuntimeError) as exc_info:
                await _maim(None)

            assert str(exc_info.value) == "maim failed"


class TestBuildMetadata:
    """Tests for build_metadata function."""

    def test_build_metadata_without_region(self):
        """Test metadata building with default full screen region."""
        result = build_metadata(cx=100, cy=200, windows=[])

        assert result["screen"]["width"] == 1280
        assert result["screen"]["height"] == 1024
        assert result["region"]["x"] == 0
        assert result["region"]["y"] == 0
        assert result["region"]["width"] == 1280
        assert result["region"]["height"] == 1024
        assert result["cursor"]["x"] == 100
        assert result["cursor"]["y"] == 200
        assert result["windows"] == []

    def test_build_metadata_with_region(self):
        """Test metadata building with custom region."""
        region = Region(x=50, y=75, width=640, height=512)
        result = build_metadata(cx=300, cy=400, windows=[], region=region)

        assert result["region"]["x"] == 50
        assert result["region"]["y"] == 75
        assert result["region"]["width"] == 640
        assert result["region"]["height"] == 512
        assert result["cursor"]["x"] == 300
        assert result["cursor"]["y"] == 400

    def test_build_metadata_with_windows(self):
        """Test metadata building with window list."""
        windows = [
            {"name": "window1", "x": 0, "y": 0},
            {"name": "window2", "x": 640, "y": 0},
        ]
        result = build_metadata(cx=100, cy=100, windows=windows)

        assert len(result["windows"]) == 2
        assert result["windows"] == windows


class TestRegion:
    """Tests for Region dataclass."""

    def test_region_creation(self):
        """Test Region dataclass creation."""
        region = Region(x=10, y=20, width=100, height=200)

        assert region.x == 10
        assert region.y == 20
        assert region.width == 100
        assert region.height == 200

    def test_region_equality(self):
        """Test Region equality comparison."""
        region1 = Region(x=10, y=20, width=100, height=200)
        region2 = Region(x=10, y=20, width=100, height=200)
        region3 = Region(x=15, y=20, width=100, height=200)

        assert region1 == region2
        assert region1 != region3
