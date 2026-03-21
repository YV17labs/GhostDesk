# Copyright (c) 2026 YV17 — MIT License
"""Shell launch tool — fire-and-forget GUI application launcher."""

import asyncio
import shlex

from mcp.server.fastmcp import FastMCP


async def launch(command: str) -> str:
    """Launch a GUI application. Returns immediately, no output is captured.

    Use for any graphical application: browsers, editors, media players, etc.
    After launching, use wait() then screenshot() to see the result.

    To open a website: launch("firefox https://example.com")
    Firefox opens URLs in new tabs automatically when already running.

    Args:
        command: The command to launch (e.g. "firefox https://linkedin.com",
                 "gedit /tmp/notes.txt", "vlc video.mp4").
    """
    try:
        parts = shlex.split(command)
    except ValueError as e:
        return f"Invalid command syntax: {e}"
    if not parts:
        return "No command provided"

    try:
        await asyncio.create_subprocess_exec(
            *parts,
            stdout=asyncio.subprocess.DEVNULL,
            stderr=asyncio.subprocess.DEVNULL,
            process_group=0,
        )
    except FileNotFoundError:
        return f"Command not found: {parts[0]}"

    return f"Launched: {command}"


def register(mcp: FastMCP) -> None:
    """Register the launch tool."""
    mcp.tool()(launch)
