# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.input.keyboard."""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.input.keyboard import (
    _normalize_chord,
    _normalize_token,
    key_press,
    key_type,
)

_FEEDBACK_RESULT = {"changed": True, "reaction_time_ms": 200}


@pytest.fixture
def _mock_deps():
    mock_wl = AsyncMock()

    with (
        patch("ghostdesk.input.keyboard.get_wayland_input", new=AsyncMock(return_value=mock_wl)),
        patch("ghostdesk.input.keyboard.get_cursor_position", return_value=(100, 200)) as mock_pos,
        patch("ghostdesk.input.keyboard.capture_before", new_callable=AsyncMock, return_value=(None, b"h")) as mock_cap,
        patch("ghostdesk.input.keyboard.poll_for_change", new_callable=AsyncMock, return_value=_FEEDBACK_RESULT) as mock_poll,
    ):
        yield mock_wl, mock_pos, mock_cap, mock_poll


# --- key_type ---

async def test_key_type_plain_ascii(_mock_deps):
    mock_wl, mock_pos, _, _ = _mock_deps
    result = await key_type("abc")
    mock_pos.assert_called_once()
    mock_wl.type_text.assert_awaited_once_with("abc")
    assert result["action"] == "Typed 3 characters"
    assert result["screen_changed"] is True


async def test_key_type_empty(_mock_deps):
    mock_wl, *_ = _mock_deps
    result = await key_type("")
    mock_wl.type_text.assert_awaited_once_with("")
    assert result["action"] == "Typed 0 characters"


async def test_key_type_with_newline(_mock_deps):
    """Newlines/tabs are handled inside WaylandInput.type_text, so
    key_type just forwards the string verbatim."""
    mock_wl, *_ = _mock_deps
    await key_type("a\nb")
    mock_wl.type_text.assert_awaited_once_with("a\nb")


async def test_key_type_with_tab(_mock_deps):
    mock_wl, *_ = _mock_deps
    await key_type("a\tb")
    mock_wl.type_text.assert_awaited_once_with("a\tb")


async def test_key_type_unicode(_mock_deps):
    """Unicode characters are forwarded to WaylandInput, which builds
    an on-the-fly XKB keymap with the needed keysyms."""
    mock_wl, *_ = _mock_deps
    await key_type("café")
    mock_wl.type_text.assert_awaited_once_with("café")


async def test_key_type_no_change(_mock_deps):
    _, _, _, mock_poll = _mock_deps
    mock_poll.return_value = {"changed": False, "reaction_time_ms": 2000}
    result = await key_type("hello")
    assert result["screen_changed"] is False


# --- key_press ---

async def test_key_press_ctrl_c(_mock_deps):
    mock_wl, mock_pos, _, _ = _mock_deps
    result = await key_press("Ctrl+c")
    mock_pos.assert_called_once()
    mock_wl.press_chord.assert_awaited_once_with(["leftctrl", "c"])
    assert result["action"] == "Pressed Ctrl+c"
    assert result["screen_changed"] is True


async def test_key_press_single_key(_mock_deps):
    mock_wl, *_ = _mock_deps
    await key_press("Return")
    mock_wl.press_chord.assert_awaited_once_with(["enter"])


async def test_key_press_tab_alias(_mock_deps):
    """Bare 'Tab' normalises to the internal ``tab`` name."""
    mock_wl, *_ = _mock_deps
    await key_press("Tab")
    mock_wl.press_chord.assert_awaited_once_with(["tab"])


async def test_key_press_three_token_combo(_mock_deps):
    mock_wl, *_ = _mock_deps
    await key_press("Ctrl+Shift+Tab")
    mock_wl.press_chord.assert_awaited_once_with(["leftctrl", "leftshift", "tab"])


async def test_key_press_alt_f4(_mock_deps):
    mock_wl, *_ = _mock_deps
    await key_press("Alt+F4")
    mock_wl.press_chord.assert_awaited_once_with(["leftalt", "f4"])


async def test_key_press_no_change(_mock_deps):
    _, _, _, mock_poll = _mock_deps
    mock_poll.return_value = {"changed": False, "reaction_time_ms": 2000}
    result = await key_press("Ctrl+c")
    assert result["screen_changed"] is False


# --- _normalize_token / _normalize_chord ---


class TestNormalizeToken:
    def test_modifiers(self):
        assert _normalize_token("ctrl") == "leftctrl"
        assert _normalize_token("Control") == "leftctrl"
        assert _normalize_token("alt") == "leftalt"
        assert _normalize_token("shift") == "leftshift"
        assert _normalize_token("super") == "leftmeta"
        assert _normalize_token("win") == "leftmeta"

    def test_named_keys(self):
        assert _normalize_token("Return") == "enter"
        assert _normalize_token("Escape") == "esc"
        assert _normalize_token("BackSpace") == "backspace"
        assert _normalize_token("Tab") == "tab"
        assert _normalize_token("Page_Up") == "pageup"
        assert _normalize_token("PageDown") == "pagedown"

    def test_function_keys(self):
        assert _normalize_token("F1") == "f1"
        assert _normalize_token("F12") == "f12"

    def test_arrow_keys(self):
        assert _normalize_token("Left") == "left"
        assert _normalize_token("Right") == "right"

    def test_single_char_passthrough(self):
        assert _normalize_token("a") == "a"
        assert _normalize_token("5") == "5"


class TestNormalizeChord:
    def test_simple_chord(self):
        assert _normalize_chord("Ctrl+c") == ["leftctrl", "c"]

    def test_three_tokens(self):
        assert _normalize_chord("Ctrl+Shift+Tab") == ["leftctrl", "leftshift", "tab"]

    def test_alt_f4(self):
        assert _normalize_chord("Alt+F4") == ["leftalt", "f4"]

    def test_single_token(self):
        assert _normalize_chord("Return") == ["enter"]
