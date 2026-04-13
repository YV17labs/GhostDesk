# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""MCP call-tool middleware: coordinate normalisation, argument coercion, and call logging.

Every coordinate the LLM sends is in the normalised space of the
configured vision-language model (0–1000 for Qwen-VL by default).  This
middleware converts them to real screen pixels before any tool sees
them, and converts outputs back to that space.
"""

from __future__ import annotations

import logging
import time

from mcp.server.fastmcp import FastMCP
from mcp.server.fastmcp.exceptions import ToolError

from ghostdesk._coords import (
    get_model_space,
    is_enabled as _coords_enabled,
    region_to_model,
    region_to_pixels,
    to_model,
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

    No-op when coordinate normalisation is disabled via
    ``GHOSTDESK_MODEL_SPACE=0`` (Claude, GPT-4o, etc.).
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


def _normalise_output_coords(result: object) -> object:
    """Convert pixel coordinates in tool output back to model space.

    Handles the metadata dict returned by screenshot() which contains
    cursor position, window geometries, and screen/region info.
    No-op when coordinate normalisation is disabled.
    """
    if not _coords_enabled() or not isinstance(result, list):
        return result

    ms = get_model_space()

    for item in result:
        if not isinstance(item, dict):
            continue

        cursor = item.get("cursor")
        if isinstance(cursor, dict):
            cursor["x"], cursor["y"] = to_model(cursor["x"], cursor["y"])

        if isinstance(item.get("screen"), dict):
            item["screen"] = {"width": ms, "height": ms}

        region = item.get("region")
        if isinstance(region, dict) and all(k in region for k in ("x", "y", "width", "height")):
            region["x"], region["y"], region["width"], region["height"] = region_to_model(
                region["x"], region["y"], region["width"], region["height"],
            )

        windows = item.get("windows")
        if isinstance(windows, list):
            for w in windows:
                if not isinstance(w, dict):
                    continue
                if "x" in w and "y" in w:
                    w["x"], w["y"] = to_model(w["x"], w["y"])
                if "width" in w and "height" in w:
                    _, _, w["width"], w["height"] = region_to_model(
                        w.get("x", 0), w.get("y", 0), w["width"], w["height"],
                    )

    return result


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
        try:
            result = await original_call_tool(name, arguments)
            result = _normalise_output_coords(result)
            elapsed = time.monotonic() - t0
            logger.info("%s(%s) → OK (%.1fs)", name, args_str, elapsed)
            return result
        except ToolError as e:
            elapsed = time.monotonic() - t0
            msg = str(e)
            if "validation error" in msg.lower():
                msg = (
                    f"Invalid arguments for {name}. You sent: {args_str}. "
                    "Each parameter must be passed separately with the correct type."
                )
            logger.error("%s(%s) → ERROR (%.1fs): %s", name, args_str, elapsed, msg)
            raise ToolError(msg) from e
        except Exception:
            elapsed = time.monotonic() - t0
            logger.exception("%s(%s) → ERROR (%.1fs)", name, args_str, elapsed)
            raise

    mcp._mcp_server.call_tool(validate_input=False)(_call_tool)
