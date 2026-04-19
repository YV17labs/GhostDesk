# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Static invariants for ``ghostdesk.input._wayland``.

The module itself opens a live Wayland connection at import via
``WaylandInput.get()`` — we don't exercise that here. These tests guard
the pure-data tables that drive wire-format behaviour, since a bad
entry would silently invert a scroll direction on every compositor.
"""

from ghostdesk.input._wayland import (
    _AXIS_HORIZONTAL,
    _AXIS_VERTICAL,
    _SCROLL_VECTORS,
    _WHEEL_STEP_FIXED,
)


def _sign(n: int) -> int:
    return (n > 0) - (n < 0)


def test_scroll_vectors_cover_all_directions():
    assert set(_SCROLL_VECTORS) == {"up", "down", "left", "right"}


def test_scroll_vectors_sign_consistency():
    """wl_pointer requires sign(value) == sign(discrete) in a frame.

    Mismatched signs let the compositor pick one side as authoritative
    and silently invert the scroll (Firefox reads ``discrete``).
    """
    for direction, (_, value, discrete) in _SCROLL_VECTORS.items():
        assert _sign(value) == _sign(discrete), (
            f"{direction!r}: value={value} and discrete={discrete} "
            "have opposite signs"
        )
        assert discrete != 0, f"{direction!r}: discrete must be non-zero"


def test_scroll_vectors_axis_mapping():
    assert _SCROLL_VECTORS["up"][0] == _AXIS_VERTICAL
    assert _SCROLL_VECTORS["down"][0] == _AXIS_VERTICAL
    assert _SCROLL_VECTORS["left"][0] == _AXIS_HORIZONTAL
    assert _SCROLL_VECTORS["right"][0] == _AXIS_HORIZONTAL


def test_scroll_vectors_direction_intuition():
    """"up"/"left" reveal content above/before — negative wl_fixed motion."""
    assert _SCROLL_VECTORS["up"][1] == -_WHEEL_STEP_FIXED
    assert _SCROLL_VECTORS["down"][1] == _WHEEL_STEP_FIXED
    assert _SCROLL_VECTORS["left"][1] == -_WHEEL_STEP_FIXED
    assert _SCROLL_VECTORS["right"][1] == _WHEEL_STEP_FIXED
