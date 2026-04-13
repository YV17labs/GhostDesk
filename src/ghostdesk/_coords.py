# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Coordinate conversion between the LLM's normalised space and real pixels.

Some vision-language models (Qwen3-VL family) work in a fixed virtual
space (typically 1000x1000) regardless of the actual screen resolution.
Others (Claude, GPT-4o) use native pixel coordinates directly. This
module provides a configurable conversion layer controlled by the
``GHOSTDESK_MODEL_SPACE`` environment variable.

Values:
    - ``1000`` (default): Qwen-VL style 0-1000 normalised space
    - ``0`` or ``none``: disabled, pass-through (for Claude/GPT-4o)
    - any positive integer: custom normalised space (e.g. 256, 512)
"""

from __future__ import annotations

import os


def _parse_model_space() -> int:
    raw = os.environ.get("GHOSTDESK_MODEL_SPACE", "1000").strip().lower()
    if raw in ("0", "none", "off", "disabled", "false", ""):
        return 0
    try:
        return max(0, int(raw))
    except ValueError:
        return 1000


# Screen size — single source of truth for the project.
SCREEN_WIDTH = int(os.environ.get("GHOSTDESK_SCREEN_WIDTH", "1280"))
SCREEN_HEIGHT = int(os.environ.get("GHOSTDESK_SCREEN_HEIGHT", "1024"))

# Normalised coordinate space the LLM operates in. 0 means pass-through.
MODEL_SPACE = _parse_model_space()


def get_model_space() -> int:
    """Return the live ``MODEL_SPACE`` value.

    Reads the module attribute each call so tests can monkeypatch it.
    """
    return MODEL_SPACE


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
