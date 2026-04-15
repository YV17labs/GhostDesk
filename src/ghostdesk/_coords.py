# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Coordinate conversion between the LLM's normalised space and real pixels.

The active model space is per-request: the HTTP layer reads the
``GhostDesk-Model-Space`` header on each MCP call and stores it in
``model_space_var``. ``0`` (default) means pass-through native pixels;
``1000`` selects Qwen-VL's 0-1000 normalised space; any positive integer
is a custom normalised space.
"""

from __future__ import annotations

import os
from contextvars import ContextVar

# Screen size — single source of truth for the project.
SCREEN_WIDTH = int(os.environ.get("GHOSTDESK_SCREEN_WIDTH", "1280"))
SCREEN_HEIGHT = int(os.environ.get("GHOSTDESK_SCREEN_HEIGHT", "1024"))

# Per-request model space set by the HTTP middleware from the
# ``GhostDesk-Model-Space`` header. ``0`` is pass-through.
model_space_var: ContextVar[int] = ContextVar("ghostdesk_model_space", default=0)


def get_model_space() -> int:
    """Return the active per-request model space."""
    return model_space_var.get()


def is_enabled() -> bool:
    """True when coordinate normalisation is active."""
    return get_model_space() > 0


def to_pixels(mx: int | float, my: int | float) -> tuple[int, int]:
    """Convert model coords → screen pixels. Pass-through when disabled."""
    ms = get_model_space()
    if ms == 0:
        return int(mx), int(my)
    return round(mx * SCREEN_WIDTH / ms), round(my * SCREEN_HEIGHT / ms)


def to_model(px: int | float, py: int | float) -> tuple[int, int]:
    """Convert screen pixels → model coords. Pass-through when disabled."""
    ms = get_model_space()
    if ms == 0:
        return int(px), int(py)
    return round(px * ms / SCREEN_WIDTH), round(py * ms / SCREEN_HEIGHT)


def region_to_pixels(mx: int, my: int, mw: int, mh: int) -> tuple[int, int, int, int]:
    """Convert a model-space region {x, y, width, height} → pixel region."""
    ms = get_model_space()
    if ms == 0:
        return mx, my, mw, mh
    return (
        round(mx * SCREEN_WIDTH / ms),
        round(my * SCREEN_HEIGHT / ms),
        round(mw * SCREEN_WIDTH / ms),
        round(mh * SCREEN_HEIGHT / ms),
    )


def region_to_model(px: int, py: int, pw: int, ph: int) -> tuple[int, int, int, int]:
    """Convert a pixel region → model-space region."""
    ms = get_model_space()
    if ms == 0:
        return px, py, pw, ph
    return (
        round(px * ms / SCREEN_WIDTH),
        round(py * ms / SCREEN_HEIGHT),
        round(pw * ms / SCREEN_WIDTH),
        round(ph * ms / SCREEN_HEIGHT),
    )
