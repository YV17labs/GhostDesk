# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk.input.feedback — post-action visual zone polling."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.input.feedback import (
    ZONE_SIZE,
    _zone_region,
    capture_before,
    poll_for_change,
)
from ghostdesk.screen._shared import SCREEN_HEIGHT, SCREEN_WIDTH, Region

MODULE = "ghostdesk.input.feedback"


# --- _zone_region ---

def test_zone_region_centre():
    """Region is centred on the point, clamped to screen."""
    r = _zone_region(640, 512)
    assert r.x == 640 - ZONE_SIZE // 2
    assert r.y == 512 - ZONE_SIZE // 2
    assert r.width == ZONE_SIZE
    assert r.height == ZONE_SIZE


def test_zone_region_top_left_clamp():
    """Region clamps to (0, 0) near the top-left corner."""
    r = _zone_region(10, 10)
    assert r.x == 0
    assert r.y == 0


def test_zone_region_bottom_right_clamp():
    """Region clamps near the bottom-right corner."""
    r = _zone_region(SCREEN_WIDTH - 5, SCREEN_HEIGHT - 5)
    assert r.x + r.width <= SCREEN_WIDTH
    assert r.y + r.height <= SCREEN_HEIGHT


# --- capture_before ---

async def test_capture_before_returns_region_and_bytes():
    """capture_before returns the region and the raw PNG bytes."""
    fake_png = b"fake-screenshot"
    with patch(f"{MODULE}.capture_png", new_callable=AsyncMock, return_value=fake_png):
        region, before = await capture_before(400, 300)

    assert isinstance(region, Region)
    assert before == fake_png


# --- poll_for_change ---

async def test_poll_detects_change():
    """poll_for_change returns changed=True when the zone image differs."""
    region = Region(0, 0, ZONE_SIZE, ZONE_SIZE)

    with patch(f"{MODULE}.capture_png", new_callable=AsyncMock, return_value=b"after"):
        result = await poll_for_change(region, b"before", timeout=1.0, interval=0.05)

    assert result["changed"] is True
    assert result["reaction_time_ms"] > 0


async def test_poll_timeout_no_change():
    """poll_for_change returns changed=False when zone never changes."""
    region = Region(0, 0, ZONE_SIZE, ZONE_SIZE)

    with patch(f"{MODULE}.capture_png", new_callable=AsyncMock, return_value=b"same"):
        result = await poll_for_change(region, b"same", timeout=0.2, interval=0.05)

    assert result["changed"] is False


async def test_poll_detects_change_after_delay():
    """poll_for_change detects a change that happens after a few polls."""
    region = Region(0, 0, ZONE_SIZE, ZONE_SIZE)

    call_count = 0

    async def changing_capture(r):
        nonlocal call_count
        call_count += 1
        return b"before" if call_count < 3 else b"after"

    with patch(f"{MODULE}.capture_png", side_effect=changing_capture):
        result = await poll_for_change(region, b"before", timeout=1.0, interval=0.05)

    assert result["changed"] is True
    assert call_count >= 3
