# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Static invariants for ``ghostdesk.input._wayland``.

The module itself opens a live Wayland connection at import via
``WaylandInput.get()`` — we don't exercise that here. These tests guard
the pure-data tables that drive wire-format behaviour, since a bad
entry would silently invert a scroll direction on every compositor.
"""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.input._wayland import (
    _AXIS_HORIZONTAL,
    _AXIS_VERTICAL,
    _SCROLL_VECTORS,
    _WHEEL_STEP_FIXED,
    WaylandInput,
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


# -- drag gesture --------------------------------------------------
#
# GtkGestureDrag arms only when it sees a persisting press with motion
# events arriving *while it is held*. Bundling press → motion → release
# into one atomic roundtrip makes GTK see a plain click and breaks real
# drag-and-drop (moving a selection, a file, a tab). Text selection
# happens to survive the bundle because GtkTextView tracks raw
# button+position, which is why regressions here are silent.

@pytest.fixture
def _drag_sequence():
    """Drive ``WaylandInput.drag`` with move/button primitives mocked.

    Yields the ordered list of ``(name, args)`` calls so a test can
    assert the exact sequence WaylandInput emits.
    """
    with (
        patch.object(WaylandInput, "move", new_callable=AsyncMock) as m_move,
        patch.object(WaylandInput, "button_down", new_callable=AsyncMock) as m_down,
        patch.object(WaylandInput, "button_up", new_callable=AsyncMock) as m_up,
        patch("asyncio.sleep", new_callable=AsyncMock),  # keep the test fast
    ):
        calls: list[tuple[str, tuple]] = []
        m_move.side_effect = lambda *a: calls.append(("move", a))
        m_down.side_effect = lambda *a: calls.append(("button_down", a))
        m_up.side_effect = lambda *a: calls.append(("button_up", a))
        yield calls


async def test_drag_does_not_bundle_press_and_release(_drag_sequence):
    """At least one motion must sit between the press and the release.

    Guards against a revert to a single-send() bundle where press and
    release arrive in the same compositor frame — that shape is
    indistinguishable from a click and silently breaks real DnD.
    """
    wl = WaylandInput.__new__(WaylandInput)  # skip the live Wayland ctor
    await wl.drag(10, 20, 100, 200)

    names = [name for name, _ in _drag_sequence]
    press = names.index("button_down")
    release = names.index("button_up")
    motions_between = names[press + 1 : release].count("move")
    assert motions_between >= 2, (
        "drag must emit intermediate motion events while the button is "
        f"held; got sequence: {names}"
    )


async def test_drag_hits_source_and_destination_coordinates(_drag_sequence):
    """First move goes to (from_x, from_y); last move lands on (to_x, to_y)."""
    wl = WaylandInput.__new__(WaylandInput)
    await wl.drag(10, 20, 100, 200)

    moves = [args for name, args in _drag_sequence if name == "move"]
    assert moves[0] == (10, 20)
    assert moves[-1] == (100, 200)


async def test_drag_button_is_held_across_motions(_drag_sequence):
    """Exactly one press and one release, press strictly before release."""
    wl = WaylandInput.__new__(WaylandInput)
    await wl.drag(10, 20, 100, 200)

    names = [name for name, _ in _drag_sequence]
    assert names.count("button_down") == 1
    assert names.count("button_up") == 1
    assert names.index("button_down") < names.index("button_up")


async def test_drag_respects_button_argument(_drag_sequence):
    """The requested button is forwarded to both press and release."""
    wl = WaylandInput.__new__(WaylandInput)
    await wl.drag(10, 20, 100, 200, button="right")

    for name, args in _drag_sequence:
        if name in ("button_down", "button_up"):
            assert args == ("right",)
