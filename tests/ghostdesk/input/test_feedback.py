# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk.input.feedback — full-screen post-action polling."""

import io
from unittest.mock import AsyncMock, patch

import pytest
from PIL import Image

from ghostdesk.input.feedback import (
    FEEDBACK_SCALE,
    capture_before,
    poll_for_change,
)

MODULE = "ghostdesk.input.feedback"


def _png_bytes() -> bytes:
    """Minimal valid PNG so poll_for_change can decode the baseline."""
    buf = io.BytesIO()
    Image.new("RGB", (4, 4), color=(0, 0, 0)).save(buf, format="PNG")
    return buf.getvalue()


# --- capture_before ---

async def test_capture_before_returns_bytes():
    """capture_before returns the raw PNG bytes from a scaled grim capture."""
    fake_png = b"fake-screenshot"
    with patch(f"{MODULE}.capture_png", new_callable=AsyncMock, return_value=fake_png) as mock_cap:
        before = await capture_before()

    assert before == fake_png
    mock_cap.assert_awaited_once_with(scale=FEEDBACK_SCALE)


# --- poll_for_change ---

async def test_poll_detects_change():
    """poll_for_change returns changed=True when diff_against_rgb says so."""
    with (
        patch(f"{MODULE}.capture_png", new_callable=AsyncMock, return_value=b"after"),
        patch(f"{MODULE}.diff_against_rgb", return_value=True),
    ):
        result = await poll_for_change(_png_bytes(), timeout=1.0, interval=0.05)

    assert result["changed"] is True
    assert result["reaction_time_ms"] > 0


async def test_poll_timeout_no_change():
    """poll_for_change returns changed=False when diff_against_rgb stays false."""
    with (
        patch(f"{MODULE}.capture_png", new_callable=AsyncMock, return_value=b"same"),
        patch(f"{MODULE}.diff_against_rgb", return_value=False),
    ):
        result = await poll_for_change(_png_bytes(), timeout=0.2, interval=0.05)

    assert result["changed"] is False


async def test_poll_detects_change_after_delay():
    """poll_for_change detects a change that happens after a few polls."""
    call_count = 0

    def differs(_baseline, _b):
        nonlocal call_count
        call_count += 1
        return call_count >= 3

    with (
        patch(f"{MODULE}.capture_png", new_callable=AsyncMock, return_value=b"x"),
        patch(f"{MODULE}.diff_against_rgb", side_effect=differs),
    ):
        result = await poll_for_change(_png_bytes(), timeout=1.0, interval=0.05)

    assert result["changed"] is True
    assert call_count >= 3


async def test_poll_uses_scaled_capture():
    """poll_for_change re-captures using the same FEEDBACK_SCALE as capture_before."""
    with (
        patch(f"{MODULE}.capture_png", new_callable=AsyncMock, return_value=b"x") as mock_cap,
        patch(f"{MODULE}.diff_against_rgb", return_value=True),
    ):
        await poll_for_change(_png_bytes(), timeout=1.0, interval=0.05)

    mock_cap.assert_awaited_with(scale=FEEDBACK_SCALE)
