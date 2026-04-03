# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.screen.cursor — cursor drawing on PNGs."""

import io

from PIL import Image

from ghostdesk.screen.cursor import draw_cursor


def _make_test_png(width: int = 50, height: int = 50, color: str = "red") -> bytes:
    """Create a small solid-color PNG for testing."""
    img = Image.new("RGB", (width, height), color)
    buf = io.BytesIO()
    img.save(buf, format="PNG")
    return buf.getvalue()


class TestDrawCursor:
    """Tests for draw_cursor."""

    def test_default_format_is_png(self):
        """draw_cursor should return PNG by default."""
        png = _make_test_png()
        result = draw_cursor(png, 25, 25)

        img = Image.open(io.BytesIO(result))
        assert img.format == "PNG"

    def test_webp_format(self):
        """draw_cursor should return WebP when requested."""
        png = _make_test_png()
        result = draw_cursor(png, 25, 25, output_format="webp")

        img = Image.open(io.BytesIO(result))
        assert img.format == "WEBP"

    def test_webp_quality(self):
        """Higher quality WebP should produce larger output."""
        png = _make_test_png(100, 100, "blue")
        low = draw_cursor(png, 50, 50, output_format="webp", quality=10)
        high = draw_cursor(png, 50, 50, output_format="webp", quality=100)
        assert len(high) > len(low)

    def test_invalid_format_falls_back_to_png(self):
        """draw_cursor should fall back to PNG for unknown formats."""
        png = _make_test_png()
        result = draw_cursor(png, 25, 25, output_format="bmp")  # type: ignore[arg-type]
        img = Image.open(io.BytesIO(result))
        assert img.format == "PNG"

    def test_quality_clamped(self):
        """draw_cursor should clamp quality to 1-100."""
        png = _make_test_png(100, 100, "blue")
        # quality=200 should be clamped to 100, quality=-5 to 1
        high = draw_cursor(png, 50, 50, output_format="webp", quality=200)
        low = draw_cursor(png, 50, 50, output_format="webp", quality=-5)
        # Both should produce valid images without error
        Image.open(io.BytesIO(high))
        Image.open(io.BytesIO(low))

    def test_preserves_dimensions(self):
        """Output image should have the same dimensions as input."""
        png = _make_test_png(80, 60)
        result = draw_cursor(png, 40, 30)

        img = Image.open(io.BytesIO(result))
        assert img.size == (80, 60)

    def test_output_is_rgb(self):
        """Output image should be in RGB mode (converted from RGBA)."""
        png = _make_test_png()
        result = draw_cursor(png, 25, 25)

        img = Image.open(io.BytesIO(result))
        assert img.mode == "RGB"

    def test_modifies_center_pixels(self):
        """Drawing a cursor should modify pixels at the cursor position."""
        png = _make_test_png(50, 50, "blue")
        original = Image.open(io.BytesIO(png))
        original_pixel = original.getpixel((25, 25))

        result = draw_cursor(png, 25, 25, output_format="png")
        modified = Image.open(io.BytesIO(result))
        modified_pixel = modified.getpixel((25, 25))
        assert modified_pixel != original_pixel

    def test_custom_color(self):
        """draw_cursor should accept a custom color parameter."""
        png = _make_test_png(50, 50, "black")
        result = draw_cursor(png, 25, 25, color=(0, 255, 0, 255), output_format="png")

        img = Image.open(io.BytesIO(result))
        # Center pixel should have been modified (green cursor on black bg)
        pixel = img.getpixel((25, 25))
        # Green channel should be dominant at the center dot
        assert pixel[1] > pixel[0]  # green > red

    def test_custom_size(self):
        """draw_cursor with larger size should affect more pixels."""
        png = _make_test_png(100, 100, "white")

        small = draw_cursor(png, 50, 50, size=10, output_format="png")
        large = draw_cursor(png, 50, 50, size=40, output_format="png")

        # The large cursor image should differ from the small one
        # Check pixels far from center that only the large cursor reaches
        small_img = Image.open(io.BytesIO(small))
        large_img = Image.open(io.BytesIO(large))

        # At distance 18px from center — only the large crosshair reaches
        far_pixel_small = small_img.getpixel((50, 32))
        far_pixel_large = large_img.getpixel((50, 32))
        assert far_pixel_small != far_pixel_large

    def test_cursor_at_edge(self):
        """draw_cursor should not crash when cursor is at the image edge."""
        png = _make_test_png(50, 50)
        # Should not raise even at corners
        result = draw_cursor(png, 0, 0)
        assert len(result) > 0

        result = draw_cursor(png, 49, 49)
        assert len(result) > 0

    def test_returns_bytes(self):
        """draw_cursor should return bytes, not a BytesIO or Image."""
        png = _make_test_png()
        result = draw_cursor(png, 10, 10)
        assert isinstance(result, bytes)
