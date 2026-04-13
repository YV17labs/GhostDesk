# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk._cursor — internal cursor position tracking."""

import ghostdesk._cursor as cursor_module
from ghostdesk._cursor import get_cursor_position, set_cursor_position


def _reset_state() -> None:
    cursor_module._cursor_x = None
    cursor_module._cursor_y = None


def test_default_position_is_screen_center():
    """get_cursor_position() returns the screen center on first call."""
    _reset_state()
    x, y = get_cursor_position()
    assert x == 640   # 1280 / 2
    assert y == 512   # 1024 / 2


def test_set_then_get_roundtrip():
    """set_cursor_position() is the source of truth after being called."""
    _reset_state()
    set_cursor_position(100, 200)
    x, y = get_cursor_position()
    assert x == 100
    assert y == 200


def test_set_cursor_overrides_previous():
    """set_cursor_position() replaces earlier values."""
    _reset_state()
    set_cursor_position(50, 50)
    set_cursor_position(400, 300)
    x, y = get_cursor_position()
    assert x == 400
    assert y == 300


def test_set_cursor_coerces_to_int():
    """set_cursor_position() coerces inputs to int."""
    _reset_state()
    set_cursor_position(123.7, 456.2)
    x, y = get_cursor_position()
    assert x == 123
    assert y == 456
