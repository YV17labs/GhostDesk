# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.input.keyboard."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.input.keyboard import _type_char, _typing_delays, press_key, type_text

MODULE = "ghostdesk.input.keyboard"

_FEEDBACK_RESULT = {"changed": True, "reaction_time_ms": 200}


@pytest.fixture(autouse=True)
def _mock_deps():
    with (
        patch(f"{MODULE}.run", new_callable=AsyncMock) as mock_run,
        patch(f"{MODULE}._typing_delays", return_value=[0.01, 0.01, 0.01]) as mock_delays,
        patch(f"{MODULE}.get_cursor_position", new_callable=AsyncMock, return_value=(100, 200)) as mock_pos,
        patch(f"{MODULE}.capture_before", new_callable=AsyncMock, return_value=(None, b"h")) as mock_cap,
        patch(f"{MODULE}.poll_for_change", new_callable=AsyncMock, return_value=_FEEDBACK_RESULT) as mock_poll,
    ):
        yield mock_run, mock_delays, mock_pos, mock_cap, mock_poll


# --- _type_char ---

async def test_type_char_newline(_mock_deps):
    mock_run, *_ = _mock_deps
    await _type_char("\n")
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "Return"])


async def test_type_char_tab(_mock_deps):
    mock_run, *_ = _mock_deps
    await _type_char("\t")
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "Tab"])


async def test_type_char_ascii(_mock_deps):
    mock_run, *_ = _mock_deps
    await _type_char("a")
    mock_run.assert_awaited_once_with(["xdotool", "type", "--clearmodifiers", "--delay", "0", "a"])


async def test_type_char_non_ascii(_mock_deps):
    mock_run, *_ = _mock_deps
    await _type_char("\u00e8")  # e-grave
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "U00E8"])


async def test_type_char_emoji(_mock_deps):
    mock_run, *_ = _mock_deps
    await _type_char("\U0001f600")  # grinning face
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "U1F600"])


# --- type_text ---

async def test_type_text_humanize(_mock_deps):
    mock_run, mock_delays, mock_pos, _, _ = _mock_deps
    result = await type_text("abc", delay_ms=50, humanize=True)
    mock_pos.assert_awaited_once()
    mock_delays.assert_called_once_with("abc", base_delay_ms=50)
    assert mock_run.await_count == 3  # one per char
    assert result["action"] == "Typed 3 characters"
    assert result["screen_changed"] is True


async def test_type_text_no_humanize(_mock_deps):
    mock_run, mock_delays, _, _, _ = _mock_deps
    result = await type_text("ab", humanize=False)
    mock_delays.assert_not_called()
    assert mock_run.await_count == 2
    assert result["action"] == "Typed 2 characters"


async def test_type_text_empty(_mock_deps):
    mock_run, _, _, _, _ = _mock_deps
    result = await type_text("", humanize=False)
    mock_run.assert_not_awaited()
    assert result["action"] == "Typed 0 characters"


async def test_type_text_humanize_empty(_mock_deps):
    _, mock_delays, _, _, _ = _mock_deps
    mock_delays.return_value = []
    result = await type_text("", delay_ms=50, humanize=True)
    assert result["action"] == "Typed 0 characters"


async def test_type_text_mixed_chars(_mock_deps):
    """Verify each character dispatches correctly through _type_char."""
    mock_run, mock_delays, _, _, _ = _mock_deps
    mock_delays.return_value = [0.001, 0.001]
    await type_text("a\n", humanize=True)
    mock_run.assert_any_await(["xdotool", "type", "--clearmodifiers", "--delay", "0", "a"])
    mock_run.assert_any_await(["xdotool", "key", "--clearmodifiers", "Return"])


async def test_type_text_no_change(_mock_deps):
    _, _, _, _, mock_poll = _mock_deps
    mock_poll.return_value = {"changed": False, "reaction_time_ms": 2000}
    result = await type_text("hello", humanize=False)
    assert result["screen_changed"] is False


# --- press_key ---

async def test_press_key(_mock_deps):
    mock_run, _, mock_pos, _, _ = _mock_deps
    result = await press_key("ctrl+c")
    mock_pos.assert_awaited_once()
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "ctrl+c"])
    assert result["action"] == "Pressed ctrl+c"
    assert result["screen_changed"] is True


async def test_press_key_single_key(_mock_deps):
    mock_run, *_ = _mock_deps
    result = await press_key("Return")
    mock_run.assert_awaited_once_with(["xdotool", "key", "--clearmodifiers", "Return"])
    assert result["action"] == "Pressed Return"


async def test_press_key_no_change(_mock_deps):
    _, _, _, _, mock_poll = _mock_deps
    mock_poll.return_value = {"changed": False, "reaction_time_ms": 2000}
    result = await press_key("ctrl+c")
    assert result["screen_changed"] is False


# --- _typing_delays ---

import random


class TestTypingDelays:
    """Tests for _typing_delays."""

    def test_length_matches_text(self):
        assert len(_typing_delays("hello")) == 5

    def test_empty_string(self):
        assert _typing_delays("") == []

    def test_all_delays_positive(self):
        random.seed(42)
        for d in _typing_delays("some text with punctuation! and spaces."):
            assert d >= 0.01

    def test_spaces_produce_longer_delays(self):
        random.seed(42)
        space_delays, letter_delays = [], []
        for _ in range(500):
            ds = _typing_delays("a b", base_delay_ms=50)
            letter_delays.append(ds[0])
            space_delays.append(ds[1])
            letter_delays.append(ds[2])
        assert sum(space_delays) / len(space_delays) > sum(letter_delays) / len(letter_delays)

    def test_punctuation_produces_longer_delays(self):
        random.seed(42)
        punct_delays, letter_delays = [], []
        for _ in range(500):
            ds = _typing_delays("a.", base_delay_ms=50)
            letter_delays.append(ds[0])
            punct_delays.append(ds[1])
        assert sum(punct_delays) / len(punct_delays) > sum(letter_delays) / len(letter_delays)

    def test_custom_base_delay(self):
        random.seed(42)
        slow = _typing_delays("abc", base_delay_ms=200)
        random.seed(42)
        fast = _typing_delays("abc", base_delay_ms=50)
        for s, f in zip(slow, fast):
            assert s > f

    def test_all_punctuation_chars_handled(self):
        random.seed(42)
        delays = _typing_delays(".,;:!?\n")
        assert len(delays) == 7
        for d in delays:
            assert d >= 0.01
