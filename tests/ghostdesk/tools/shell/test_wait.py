# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.shell.wait — timed pause between actions."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.shell.wait import wait


async def test_wait_returns_message():
    """wait() returns a message indicating how long it waited."""
    with patch("asyncio.sleep", new_callable=AsyncMock) as mock_sleep:
        result = await wait(500)

        assert result == "Waited 500ms"


async def test_wait_calls_sleep_with_seconds():
    """wait() converts milliseconds to seconds for asyncio.sleep."""
    with patch("asyncio.sleep", new_callable=AsyncMock) as mock_sleep:
        await wait(1500)

        mock_sleep.assert_awaited_once_with(1.5)


async def test_wait_zero_milliseconds():
    """wait() handles zero milliseconds."""
    with patch("asyncio.sleep", new_callable=AsyncMock) as mock_sleep:
        result = await wait(0)

        assert result == "Waited 0ms"
        mock_sleep.assert_awaited_once_with(0.0)


async def test_wait_small_value():
    """wait() handles small millisecond values correctly."""
    with patch("asyncio.sleep", new_callable=AsyncMock) as mock_sleep:
        result = await wait(1)

        assert result == "Waited 1ms"
        mock_sleep.assert_awaited_once_with(0.001)
