# Copyright (c) 2026 YV17 — MIT License
"""Human-like mouse movement and typing — Bézier curves, easing, jitter."""

import asyncio
import math
import random

from ghostdesk.utils.xdotool import run


def _bezier_point(t: float, p0: tuple, p1: tuple, p2: tuple, p3: tuple) -> tuple[int, int]:
    """Compute a point on a cubic Bézier curve at parameter t."""
    u = 1 - t
    x = u**3 * p0[0] + 3 * u**2 * t * p1[0] + 3 * u * t**2 * p2[0] + t**3 * p3[0]
    y = u**3 * p0[1] + 3 * u**2 * t * p1[1] + 3 * u * t**2 * p2[1] + t**3 * p3[1]
    return (int(x), int(y))


def _ease_in_out(t: float) -> float:
    """Sinusoidal ease-in-out: slow start, fast middle, slow end."""
    return 0.5 * (1 - math.cos(math.pi * t))


def _generate_trajectory(
    from_x: int, from_y: int, to_x: int, to_y: int,
) -> list[tuple[int, int]]:
    """Generate a human-like trajectory using a cubic Bézier curve."""
    distance = math.hypot(to_x - from_x, to_y - from_y)
    steps = max(15, int(distance / 4))
    spread = max(30, distance * 0.25)

    # Random control points offset from the straight line
    cp1 = (
        from_x + (to_x - from_x) * 0.3 + random.gauss(0, spread * 0.3),
        from_y + (to_y - from_y) * 0.3 + random.gauss(0, spread * 0.3),
    )
    cp2 = (
        from_x + (to_x - from_x) * 0.7 + random.gauss(0, spread * 0.3),
        from_y + (to_y - from_y) * 0.7 + random.gauss(0, spread * 0.3),
    )

    # Tiny jitter on target (±2px) to simulate human imprecision
    jitter_x = random.randint(-2, 2)
    jitter_y = random.randint(-2, 2)
    target = (to_x + jitter_x, to_y + jitter_y)

    p0 = (from_x, from_y)
    points = []
    for i in range(steps + 1):
        t = _ease_in_out(i / steps)
        points.append(_bezier_point(t, p0, cp1, cp2, target))

    # Deduplicate consecutive identical points
    unique = [points[0]]
    for p in points[1:]:
        if p != unique[-1]:
            unique.append(p)

    return unique


async def human_move(from_x: int, from_y: int, to_x: int, to_y: int) -> None:
    """Move mouse along a Bézier curve with variable speed."""
    trajectory = _generate_trajectory(from_x, from_y, to_x, to_y)

    for px, py in trajectory:
        await run(["xdotool", "mousemove", str(px), str(py)])
        await asyncio.sleep(random.uniform(0.003, 0.009))

    # Final snap to exact target (remove jitter)
    await run(["xdotool", "mousemove", str(to_x), str(to_y)])


async def get_cursor_position() -> tuple[int, int]:
    """Return current cursor (x, y) from xdotool."""
    output = await run(["xdotool", "getmouselocation"])
    # Format: "x:123 y:456 screen:0 window:789"
    parts = dict(p.split(":") for p in output.split())
    return int(parts["x"]), int(parts["y"])


def typing_delays(text: str, base_delay_ms: int = 50) -> list[float]:
    """Generate human-like per-character delays (in seconds).

    Slower after spaces and punctuation, variable within words.
    """
    delays: list[float] = []
    for char in text:
        if char == " ":
            ms = random.gauss(base_delay_ms * 1.5, base_delay_ms * 0.3)
        elif char in ".,;:!?\n":
            ms = random.gauss(base_delay_ms * 2.5, base_delay_ms * 0.5)
        else:
            ms = random.gauss(base_delay_ms, base_delay_ms * 0.2)
        delays.append(max(10, ms) / 1000.0)
    return delays
