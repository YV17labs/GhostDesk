# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""MCP call-tool middleware: argument coercion and call logging."""

import logging
import time

from mcp.server.fastmcp import FastMCP
from mcp.server.fastmcp.exceptions import ToolError

logger = logging.getLogger("ghostdesk")


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
        args_str = ", ".join(f"{k}={repr(v)[:80]}" for k, v in arguments.items())

        t0 = time.monotonic()
        try:
            result = await original_call_tool(name, arguments)
            elapsed = time.monotonic() - t0
            logger.info("%s(%s) → OK (%.1fs)", name, args_str, elapsed)
            return result
        except ToolError as e:
            elapsed = time.monotonic() - t0
            msg = str(e)
            if "validation error" in msg.lower():
                msg = f"Invalid arguments for {name}. You sent: {args_str}. Each parameter must be passed separately with the correct type."
            logger.error("%s(%s) → ERROR (%.1fs): %s", name, args_str, elapsed, msg)
            raise ToolError(msg) from e
        except Exception:
            elapsed = time.monotonic() - t0
            logger.exception("%s(%s) → ERROR (%.1fs)", name, args_str, elapsed)
            raise

    mcp._mcp_server.call_tool(validate_input=False)(_call_tool)
