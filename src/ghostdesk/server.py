# Copyright (c) 2026 YV17 — MIT License
"""GhostDesk — MCP server entry point."""

import os

from mcp.server.fastmcp import FastMCP

from ghostdesk._logging import configure_logging, install_call_logging
from ghostdesk.instructions import INSTRUCTIONS
from ghostdesk.tools import accessibility, clipboard, devices, shell


def create_app(port: int | None = None) -> FastMCP:
    """Create and configure the MCP server instance.

    Args:
        port: Listening port. Defaults to the PORT env var, or 3000.
    """
    if port is None:
        port = int(os.environ.get("PORT", "3000"))

    mcp = FastMCP("ghostdesk", instructions=INSTRUCTIONS, host="0.0.0.0", port=port)

    devices.register(mcp)
    accessibility.register(mcp)
    shell.register(mcp)
    clipboard.register(mcp)

    install_call_logging(mcp)

    return mcp


def main() -> None:
    """Start the MCP server with Streamable HTTP transport."""
    os.environ.setdefault("LC_ALL", "en_US.utf8")
    os.environ.setdefault("LANG", "en_US.utf8")
    configure_logging()
    create_app().run(transport="streamable-http")


if __name__ == "__main__":
    main()
