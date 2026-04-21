# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Persistent Wayland input controller.

Opens a single Wayland connection at first use and reuses it for every
subsequent pointer/keyboard action. Two unstable protocol extensions are
required:

- ``zwlr_virtual_pointer_manager_v1`` — provided by any wlroots-based
  compositor (Sway, Hyprland, Wayfire, labwc, river, …).
- ``zwp_virtual_keyboard_manager_v1`` — provided by the same set plus a
  few others.

If either is missing the constructor raises a clear ``RuntimeError`` so
the MCP server fails fast and loudly at startup.

The keyboard path uses the same trick as ``wtype``: we build a minimal
XKB keymap on the fly where each keysym we need is assigned its own
dedicated keycode. This makes text entry layout-independent — the US
system keymap, a French AZERTY, or a Dvorak layout all produce the
exact same result, because we never touch the compositor's system
keymap at all.
"""

from __future__ import annotations

import asyncio
import os
import time
from typing import Callable, Literal

from pywayland.client import Display

Button = Literal["left", "middle", "right"]
ScrollDirection = Literal["up", "down", "left", "right"]

from ghostdesk._coords import SCREEN_HEIGHT, SCREEN_WIDTH
from ghostdesk.input._wl_bindings.virtual_keyboard_unstable_v1 import (
    ZwpVirtualKeyboardManagerV1,
)
from ghostdesk.input._wl_bindings.wayland import WlSeat
from ghostdesk.input._wl_bindings.wlr_virtual_pointer_unstable_v1 import (
    ZwlrVirtualPointerManagerV1,
)

# --- Linux input-event-codes.h -----------------------------------------

BTN_LEFT = 0x110
BTN_RIGHT = 0x111
BTN_MIDDLE = 0x112

_BUTTON_CODES: dict[str, int] = {
    "left": BTN_LEFT, "right": BTN_RIGHT, "middle": BTN_MIDDLE,
}

# --- wl_pointer / wl_keyboard enums ------------------------------------

_STATE_RELEASED = 0
_STATE_PRESSED = 1

_AXIS_VERTICAL = 0    # wl_pointer.axis.vertical_scroll
_AXIS_HORIZONTAL = 1  # wl_pointer.axis.horizontal_scroll

_AXIS_SOURCE_WHEEL = 0  # wl_pointer.axis_source.wheel

# wl_fixed_t uses 1/256 units per scroll unit; 10 units per notch is the
# value most wlroots clients send for a synthetic wheel tick.
_WHEEL_STEP_FIXED = 10 * 256

# (axis, wl_fixed motion value, discrete notch count).
# The wl_pointer protocol requires sign(value) == sign(discrete) within
# a frame; wlroots exposes both and Firefox trusts `discrete` for wheel
# sources, so a mismatch silently inverts the scroll direction.
_SCROLL_VECTORS: dict[str, tuple[int, int, int]] = {
    "up":    (_AXIS_VERTICAL,   -_WHEEL_STEP_FIXED, -1),
    "down":  (_AXIS_VERTICAL,    _WHEEL_STEP_FIXED,  1),
    "left":  (_AXIS_HORIZONTAL, -_WHEEL_STEP_FIXED, -1),
    "right": (_AXIS_HORIZONTAL,  _WHEEL_STEP_FIXED,  1),
}

_KEYMAP_FORMAT_XKB_V1 = 1

# --- X11 keysym table --------------------------------------------------
# Only the symbolic names we actually need — derived from X11/keysymdef.h.
# Key friendly-name → keysym int. Names mirror what ``keyboard.py``
# normalizes to (all lowercase, no underscores, ``leftctrl`` etc.).

_NAMED_KEYSYMS: dict[str, int] = {
    "leftctrl": 0xFFE3, "rightctrl": 0xFFE4,
    "leftshift": 0xFFE1, "rightshift": 0xFFE2,
    "leftalt": 0xFFE9, "rightalt": 0xFFEA,
    "leftmeta": 0xFFEB, "rightmeta": 0xFFEC,
    "enter": 0xFF0D,
    "esc": 0xFF1B,
    "backspace": 0xFF08,
    "tab": 0xFF09,
    "delete": 0xFFFF,
    "insert": 0xFF63,
    "home": 0xFF50,
    "end": 0xFF57,
    "pageup": 0xFF55,
    "pagedown": 0xFF56,
    "left": 0xFF51,
    "up": 0xFF52,
    "right": 0xFF53,
    "down": 0xFF54,
    "space": 0x0020,
}
_NAMED_KEYSYMS.update({f"f{i}": 0xFFBD + i for i in range(1, 13)})

# Whitespace characters that don't map cleanly to Unicode keysyms —
# they need their X11 functional keysyms instead.
_CHAR_KEYSYMS: dict[str, int] = {
    "\n": _NAMED_KEYSYMS["enter"],
    "\t": _NAMED_KEYSYMS["tab"],
}

# Keysym → XKB modifier bit mask. Modifier state is announced to the
# compositor via ``virtual_keyboard.modifiers(mask, …)`` rather than by
# pressing modifier keysyms as regular keys — that's what the protocol
# expects and it bypasses keymap interpretation quirks.
_MODIFIER_BITS: dict[int, int] = {
    0xFFE1: 0x01, 0xFFE2: 0x01,  # Shift_L / Shift_R  → Shift
    0xFFE3: 0x04, 0xFFE4: 0x04,  # Control_L / R      → Control
    0xFFE9: 0x08, 0xFFEA: 0x08,  # Alt_L / R          → Mod1
    0xFFEB: 0x40, 0xFFEC: 0x40,  # Super_L / R        → Mod4
}


def keysym_for(token: str) -> int:
    """Resolve a friendly key name or single character to its X11 keysym.

    Examples:
        ``keysym_for("leftctrl")`` → 0xFFE3
        ``keysym_for("a")``        → 0x61
        ``keysym_for("é")``        → 0xE9
        ``keysym_for("€")``        → 0x010020AC (Unicode keysym range)
    """
    low = token.lower()
    if low in _NAMED_KEYSYMS:
        return _NAMED_KEYSYMS[low]
    if len(token) != 1:
        raise ValueError(f"unknown key name: {token!r}")
    cp = ord(token)
    if cp < 0x100:
        return cp
    # X11 keysym convention for Unicode > latin1: codepoint | 0x01000000.
    return cp | 0x01000000


def _build_keymap(keysyms: list[int]) -> str:
    """Serialise ``keysyms`` to an XKB v1 text keymap.

    Slot ``i`` maps XKB keycode ``8 + i`` (= evdev keycode ``i``) to
    ``keysyms[i]`` as a ``ONE_LEVEL`` key: no shift/lock/modifier level
    switching. Modifier state is announced separately via
    ``virtual_keyboard.modifiers``, so no modifier_map is emitted here.
    """
    if not keysyms:
        # Empty keymaps are legal; the compositor just gets a no-op layout.
        return (
            'xkb_keymap {\n'
            '  xkb_keycodes "ghostdesk" { minimum = 8; maximum = 8; };\n'
            '  xkb_types "ghostdesk" {\n'
            '    virtual_modifiers Ghostdesk;\n'
            '    type "ONE_LEVEL" { modifiers = none; level_name[Level1] = "Any"; };\n'
            '  };\n'
            '  xkb_compatibility "ghostdesk" {};\n'
            '  xkb_symbols "ghostdesk" {};\n'
            '};\n'
        )

    last = 8 + len(keysyms) - 1
    kc_lines = "\n".join(f"    <K{i}> = {8 + i};" for i in range(len(keysyms)))
    sym_lines = "\n".join(
        f'    key <K{i}> {{ type = "ONE_LEVEL", symbols[Group1] = [ 0x{k:08x} ] }};'
        for i, k in enumerate(keysyms)
    )

    return (
        'xkb_keymap {\n'
        '  xkb_keycodes "ghostdesk" {\n'
        '    minimum = 8;\n'
        f'    maximum = {last};\n'
        f'{kc_lines}\n'
        '  };\n'
        '  xkb_types "ghostdesk" {\n'
        '    virtual_modifiers Ghostdesk;\n'
        '    type "ONE_LEVEL" {\n'
        '      modifiers = none;\n'
        '      level_name[Level1] = "Any";\n'
        '    };\n'
        '  };\n'
        '  xkb_compatibility "ghostdesk" {};\n'
        '  xkb_symbols "ghostdesk" {\n'
        f'{sym_lines}\n'
        '  };\n'
        '};\n'
    )


class WaylandInput:
    """Singleton input controller with a persistent Wayland connection."""

    _instance: "WaylandInput | None" = None
    _init_lock: asyncio.Lock = asyncio.Lock()

    def __init__(self) -> None:
        self._display: Display | None = None
        self._seat = None
        self._pointer_mgr = None
        self._keyboard_mgr = None
        self._pointer = None
        self._keyboard = None
        self._lock = asyncio.Lock()
        # keysym int → keycode index (position in the keymap slot list).
        self._keysyms: list[int] = []
        self._keysym_index: dict[int, int] = {}

    # -- bootstrap ------------------------------------------------------

    @classmethod
    async def get(cls) -> "WaylandInput":
        """Return the shared instance, connecting on first call."""
        if cls._instance is not None:
            return cls._instance
        async with cls._init_lock:
            if cls._instance is None:
                inst = cls()
                await asyncio.to_thread(inst._connect_sync)
                cls._instance = inst
        return cls._instance

    def _connect_sync(self) -> None:
        display = Display()
        display.connect()
        registry = display.get_registry()

        bound: dict[str, object | None] = {
            "seat": None, "pointer_mgr": None, "keyboard_mgr": None,
        }

        def on_global(reg, name, interface, version):
            if interface == "wl_seat":
                bound["seat"] = reg.bind(name, WlSeat, min(version, 7))
            elif interface == "zwlr_virtual_pointer_manager_v1":
                bound["pointer_mgr"] = reg.bind(
                    name, ZwlrVirtualPointerManagerV1, min(version, 2),
                )
            elif interface == "zwp_virtual_keyboard_manager_v1":
                bound["keyboard_mgr"] = reg.bind(
                    name, ZwpVirtualKeyboardManagerV1, min(version, 1),
                )

        registry.dispatcher["global"] = on_global
        display.roundtrip()

        missing = [k for k, v in bound.items() if v is None]
        if missing:
            display.disconnect()
            raise RuntimeError(
                "Wayland compositor is missing required protocols: "
                f"{missing}. Ghostdesk needs wl_seat, "
                "zwlr_virtual_pointer_manager_v1 (any wlroots compositor: "
                "Sway, Hyprland, Wayfire, labwc, river), and "
                "zwp_virtual_keyboard_manager_v1."
            )

        self._display = display
        self._seat = bound["seat"]
        self._pointer_mgr = bound["pointer_mgr"]
        self._keyboard_mgr = bound["keyboard_mgr"]

        self._pointer = self._pointer_mgr.create_virtual_pointer(self._seat)
        self._keyboard = self._keyboard_mgr.create_virtual_keyboard(self._seat)
        display.roundtrip()

        # Upload an empty keymap so the virtual keyboard is in a valid
        # state even before the first ``type_text`` call; real keymaps
        # get pushed lazily once we know which keysyms we need.
        self._upload_keymap_sync()

    # -- helpers --------------------------------------------------------

    @staticmethod
    def _now_ms() -> int:
        return int(time.time() * 1000) & 0xFFFFFFFF

    def _upload_keymap_sync(self) -> None:
        """Serialise the current keysym pool and push it to the compositor."""
        keymap_str = _build_keymap(self._keysyms).encode("utf-8") + b"\x00"
        size = len(keymap_str)
        fd = os.memfd_create("ghostdesk-keymap", os.MFD_CLOEXEC)
        try:
            os.write(fd, keymap_str)
            self._keyboard.keymap(_KEYMAP_FORMAT_XKB_V1, fd, size)
            self._display.roundtrip()
        finally:
            os.close(fd)

    def _ensure_keysyms_sync(self, keysyms: list[int]) -> list[int]:
        """Assign keycodes for any unseen keysym, re-uploading if needed.

        Returns the evdev keycodes (i.e. ``slot_index``) for each input
        keysym, in the same order.
        """
        changed = False
        for k in keysyms:
            if k not in self._keysym_index:
                self._keysym_index[k] = len(self._keysyms)
                self._keysyms.append(k)
                changed = True
        if changed:
            self._upload_keymap_sync()
        return [self._keysym_index[k] for k in keysyms]

    async def _run(self, send: Callable[[], None]) -> None:
        """Run ``send`` under the serialisation lock in a worker thread.

        ``send`` must leave the display in a consistent state (typically
        ending with ``self._display.roundtrip()``). Every public primitive
        goes through this helper so event ordering and thread-safety are
        handled in exactly one place.
        """
        async with self._lock:
            await asyncio.to_thread(send)

    def _motion(self, x: int, y: int) -> None:
        self._pointer.motion_absolute(
            self._now_ms(), x, y, SCREEN_WIDTH, SCREEN_HEIGHT,
        )
        self._pointer.frame()

    def _button(self, code: int, state: int) -> None:
        self._pointer.button(self._now_ms(), code, state)
        self._pointer.frame()

    # -- pointer primitives --------------------------------------------

    async def move(self, x: int, y: int) -> None:
        def send() -> None:
            self._motion(x, y)
            self._display.roundtrip()
        await self._run(send)

    async def button_down(self, button: Button = "left") -> None:
        code = _BUTTON_CODES[button]
        def send() -> None:
            self._button(code, _STATE_PRESSED)
            self._display.roundtrip()
        await self._run(send)

    async def button_up(self, button: Button = "left") -> None:
        code = _BUTTON_CODES[button]
        def send() -> None:
            self._button(code, _STATE_RELEASED)
            self._display.roundtrip()
        await self._run(send)

    async def click(self, button: Button = "left") -> None:
        code = _BUTTON_CODES[button]
        def send() -> None:
            self._button(code, _STATE_PRESSED)
            self._button(code, _STATE_RELEASED)
            self._display.roundtrip()
        await self._run(send)

    async def drag(
        self, from_x: int, from_y: int, to_x: int, to_y: int,
        button: Button = "left",
    ) -> None:
        """Press ``button`` at (from_x, from_y), drag to (to_x, to_y), release.

        Bundled into one thread hop + one roundtrip so the compositor sees
        a contiguous motion stream instead of four separate RPC waits.
        """
        code = _BUTTON_CODES[button]
        def send() -> None:
            self._motion(from_x, from_y)
            self._button(code, _STATE_PRESSED)
            self._motion(to_x, to_y)
            self._button(code, _STATE_RELEASED)
            self._display.roundtrip()
        await self._run(send)

    async def scroll(self, direction: ScrollDirection, amount: int) -> None:
        """One ``amount``-notch wheel scroll in ``direction``.

        The virtual-pointer ``axis_discrete`` request is self-contained:
        wlroots unpacks it into both the continuous ``delta`` and the
        ``delta_discrete`` step count, so no separate ``axis`` call is
        needed (it would just overwrite ``delta`` with the same value).
        """
        axis, value, discrete = _SCROLL_VECTORS[direction]

        def send() -> None:
            for _ in range(amount):
                t = self._now_ms()
                self._pointer.axis_source(_AXIS_SOURCE_WHEEL)
                self._pointer.axis_discrete(t, axis, value, discrete)
                self._pointer.frame()
            self._display.roundtrip()
        await self._run(send)

    # -- keyboard primitives -------------------------------------------

    async def type_text(self, text: str) -> None:
        """Press-and-release every character in ``text`` in order.

        Newlines become ``Return``, tabs become ``Tab``. Every other
        character is mapped via its Unicode keysym on the fly so the
        result is layout-independent.
        """
        if not text:
            return
        keysyms = [_CHAR_KEYSYMS.get(ch) or keysym_for(ch) for ch in text]

        def send() -> None:
            codes = self._ensure_keysyms_sync(keysyms)
            for kc in codes:
                self._keyboard.key(self._now_ms(), kc, _STATE_PRESSED)
                self._keyboard.key(self._now_ms(), kc, _STATE_RELEASED)
            self._display.roundtrip()
        await self._run(send)

    async def press_chord(self, tokens: list[str]) -> None:
        """Press a chord: modifiers held while non-modifier keys tap.

        Modifier tokens are aggregated into a bitmask and announced via
        ``virtual_keyboard.modifiers(mask, 0, 0, 0)``; non-modifier
        tokens are pressed and released while the mask is held.
        """
        if not tokens:
            return

        mask = 0
        non_mods: list[int] = []
        for t in tokens:
            k = keysym_for(t)
            bit = _MODIFIER_BITS.get(k)
            if bit is not None:
                mask |= bit
            else:
                non_mods.append(k)

        def send() -> None:
            codes = self._ensure_keysyms_sync(non_mods) if non_mods else []
            if mask:
                self._keyboard.modifiers(mask, 0, 0, 0)
            for kc in codes:
                self._keyboard.key(self._now_ms(), kc, _STATE_PRESSED)
                self._keyboard.key(self._now_ms(), kc, _STATE_RELEASED)
            if mask:
                self._keyboard.modifiers(0, 0, 0, 0)
            self._display.roundtrip()
        await self._run(send)


async def get_wayland_input() -> WaylandInput:
    """Convenience accessor used by ``input/mouse.py`` and ``input/keyboard.py``."""
    return await WaylandInput.get()
