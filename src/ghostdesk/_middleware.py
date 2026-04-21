# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""MCP call-tool middleware: coordinate normalisation, argument coercion, and call logging.

When the client sends ``GhostDesk-Model-Space: <N>`` on the request,
rescales the LLM's normalised coordinates to screen pixels before any
tool sees them. Otherwise pass-through.
"""

from __future__ import annotations

import logging
import time

from mcp.server.fastmcp import FastMCP
from mcp.server.fastmcp.exceptions import ToolError

from ghostdesk._coords import (
    is_enabled as _coords_enabled,
    region_to_pixels,
    to_pixels,
)

logger = logging.getLogger("ghostdesk")

# (input_key, output_key) pairs that carry xy coordinates in tool arguments.
_XY_PAIRS: tuple[tuple[str, str], ...] = (
    ("x", "y"),
    ("from_x", "from_y"),
    ("to_x", "to_y"),
)


def _coerce_xy_args(arguments: dict) -> dict:
    """Fix a common LLM mistake where both coordinates are packed into x.

    The LLM sometimes sends x='383, 22' instead of x=383, y=22.
    When x is a string containing a comma, we split it and return corrected arguments.
    """
    x = arguments.get("x")
    if not isinstance(x, str) or "," not in x:
        return arguments
    parts = x.split(",", 1)
    try:
        x_int = int(parts[0].strip())
        y_int = int(parts[1].strip())
    except ValueError:
        return arguments
    fixed = dict(arguments)
    fixed["x"] = x_int
    fixed["y"] = y_int
    return fixed


def _normalise_input_coords(arguments: dict) -> dict:
    """Convert model coordinates to screen pixels for all known xy pairs.

    No-op when no ``GhostDesk-Model-Space`` header was sent (frontier
    models like Claude, GPT-4o, Gemini emit native pixels).
    """
    if not _coords_enabled():
        return arguments
    args = dict(arguments)

    for kx, ky in _XY_PAIRS:
        if kx in args and ky in args:
            args[kx], args[ky] = to_pixels(int(args[kx]), int(args[ky]))

    region = args.get("region")
    if isinstance(region, dict) and all(k in region for k in ("x", "y", "width", "height")):
        px, py, pw, ph = region_to_pixels(
            int(region["x"]), int(region["y"]),
            int(region["width"]), int(region["height"]),
        )
        args["region"] = {"x": px, "y": py, "width": pw, "height": ph}

    return args


def install_middleware(mcp: FastMCP) -> None:
    """Wrap call_tool with argument coercion and call logging.

    FastMCP's _setup_handlers captures a reference to the original call_tool
    before our registration code runs. We use the lowlevel
    _mcp_server.call_tool(validate_input=False) decorator to properly hook
    the internal call path. This is a deliberate trade-off: accessing
    _mcp_server (private) buys us correct behavior without relying on
    unstable public APIs.
    """
    original_call_tool = mcp.call_tool

    async def _call_tool(name: str, arguments: dict) -> object:
        arguments = _coerce_xy_args(arguments)
        arguments = _normalise_input_coords(arguments)
        args_str = ", ".join(f"{k}={repr(v)[:80]}" for k, v in arguments.items())

        t0 = time.monotonic()
        elapsed_ms = lambda: int((time.monotonic() - t0) * 1000)
        try:
            result = await original_call_tool(name, arguments)
            logger.info("%s(%s) → OK (%d ms)", name, args_str, elapsed_ms())
            return result
        except ToolError as e:
            msg = str(e)
            if "validation error" in msg.lower():
                msg = (
                    f"Invalid arguments for {name}. You sent: {args_str}. "
                    "Each parameter must be passed separately with the correct type."
                )
            logger.error("%s(%s) → ERROR (%d ms): %s", name, args_str, elapsed_ms(), msg)
            raise ToolError(msg) from e
        except Exception:
            logger.exception("%s(%s) → ERROR (%d ms)", name, args_str, elapsed_ms())
            raise

    mcp._mcp_server.call_tool(validate_input=False)(_call_tool)
