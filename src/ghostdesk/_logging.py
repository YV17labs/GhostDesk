# Copyright (c) 2026 YV17 — MIT License
"""Logging setup and MCP call-tool middleware."""

import logging
import time

from mcp.server.fastmcp import FastMCP
from mcp.server.fastmcp.exceptions import ToolError

logger = logging.getLogger("ghostdesk")

_LOG_FMT = "%(asctime)s  %(levelname)-5s  %(name)s  %(message)s"
_LOG_DATEFMT = "%H:%M:%S"


def configure_logging() -> None:
    """Unify log format across the app, MCP SDK, and uvicorn."""
    import uvicorn.config

    handler = logging.StreamHandler()
    handler.setFormatter(logging.Formatter(_LOG_FMT, datefmt=_LOG_DATEFMT))

    root = logging.getLogger()
    root.handlers = [handler]
    root.setLevel(logging.INFO)

    fmt_cfg = {"fmt": _LOG_FMT, "datefmt": _LOG_DATEFMT}
    if "formatters" not in uvicorn.config.LOGGING_CONFIG:
        logger.warning(
            "uvicorn.config.LOGGING_CONFIG missing 'formatters' key; "
            "log format may not be unified across all output"
        )
    else:
        for formatter in uvicorn.config.LOGGING_CONFIG["formatters"].values():
            formatter.update(fmt_cfg)


def install_call_logging(mcp: FastMCP) -> None:
    """Wrap call_tool to log every invocation with name, args, and duration.

    FastMCP's _setup_handlers captures a reference to the original call_tool
    before our registration code runs. We use the lowlevel
    _mcp_server.call_tool(validate_input=False) decorator to properly hook
    the internal call path. This is a deliberate trade-off: accessing
    _mcp_server (private) buys us correct behavior without relying on
    unstable public APIs.
    """
    original_call_tool = mcp.call_tool

    async def _logged_call_tool(name: str, arguments: dict) -> object:
        def format_args() -> str:
            return ", ".join(
                f"{k}={repr(v)[:80]}"
                for k, v in arguments.items()
            )

        t0 = time.monotonic()
        try:
            result = await original_call_tool(name, arguments)
            elapsed = time.monotonic() - t0
            logger.info("%s(%s) → OK (%.1fs)", name, format_args(), elapsed)
            return result
        except ToolError as e:
            elapsed = time.monotonic() - t0
            msg = str(e)
            if "validation error" in msg.lower():
                # Replace verbose Pydantic trace with a short message.
                msg = f"Invalid arguments for {name}. You sent: {format_args()}. Each parameter must be passed separately with the correct type."
            logger.error("%s(%s) → ERROR (%.1fs): %s", name, format_args(), elapsed, msg)
            raise ToolError(msg) from e
        except Exception:
            elapsed = time.monotonic() - t0
            logger.exception("%s(%s) → ERROR (%.1fs)", name, format_args(), elapsed)
            raise

    mcp._mcp_server.call_tool(validate_input=False)(_logged_call_tool)
