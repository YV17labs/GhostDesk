# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.devices.keyboard."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.tools.devices.keyboard import _type_char, press_key, type_text

MODULE = "ghostdesk.tools.devices.keyboard"


@pytest.fixture(autouse=True)
def _mock_deps():
    with (
        patch(f"{MODULE}.run", new_callable=AsyncMock) as mock_run,
        patch(f"{MODULE}.typing_delays", return_value=[0.01, 0.01, 0.01]) as mock_delays,
    ):
        yield mock_run, mock_delays


# --- _type_char ---

async def test_type_char_newline(_mock_deps):
    mock_run, _ = _mock_deps
    await _type_char("\n")
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "Return"])


async def test_type_char_tab(_mock_deps):
    mock_run, _ = _mock_deps
    await _type_char("\t")
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "Tab"])


async def test_type_char_ascii(_mock_deps):
    mock_run, _ = _mock_deps
    await _type_char("a")
    mock_run.assert_awaited_once_with(["xdotool", "type", "--clearmodifiers", "--delay", "0", "a"])


async def test_type_char_non_ascii(_mock_deps):
    mock_run, _ = _mock_deps
    await _type_char("\u00e8")  # e-grave
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "U00E8"])


async def test_type_char_emoji(_mock_deps):
    mock_run, _ = _mock_deps
    await _type_char("\U0001f600")  # grinning face
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "U1F600"])


# --- type_text ---

async def test_type_text_humanize(_mock_deps):
    mock_run, mock_delays = _mock_deps
    result = await type_text("abc", delay_ms=50, humanize=True)
    mock_delays.assert_called_once_with("abc", base_delay_ms=50)
    assert mock_run.await_count == 3  # one per char
    assert result == "Typed 3 characters"


async def test_type_text_no_humanize(_mock_deps):
    mock_run, mock_delays = _mock_deps
    result = await type_text("ab", humanize=False)
    mock_delays.assert_not_called()
    assert mock_run.await_count == 2
    assert result == "Typed 2 characters"


async def test_type_text_empty(_mock_deps):
    mock_run, _ = _mock_deps
    result = await type_text("", humanize=False)
    mock_run.assert_not_awaited()
    assert result == "Typed 0 characters"


async def test_type_text_humanize_empty(_mock_deps):
    _, mock_delays = _mock_deps
    mock_delays.return_value = []
    result = await type_text("", delay_ms=50, humanize=True)
    assert result == "Typed 0 characters"


async def test_type_text_mixed_chars(_mock_deps):
    """Verify each character dispatches correctly through _type_char."""
    mock_run, mock_delays = _mock_deps
    mock_delays.return_value = [0.001, 0.001]
    await type_text("a\n", humanize=True)
    # 'a' -> xdotool type, '\n' -> xdotool key Return
    mock_run.assert_any_await(["xdotool", "type", "--clearmodifiers", "--delay", "0", "a"])
    mock_run.assert_any_await(["xdotool", "key", "--clearmodifiers", "Return"])


# --- press_key ---

async def test_press_key(_mock_deps):
    mock_run, _ = _mock_deps
    result = await press_key("ctrl+c")
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "ctrl+c"])
    assert result == "Pressed ctrl+c"


async def test_press_key_single_key(_mock_deps):
    mock_run, _ = _mock_deps
    result = await press_key("Return")
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "Return"])
    assert result == "Pressed Return"
