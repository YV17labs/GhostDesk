# Copyright (c) 2026 YV17 — MIT License
"""Shell execution and application launcher tools."""

import asyncio
import os
import shlex
import signal

from mcp.server.fastmcp import FastMCP

_KILL_DRAIN_TIMEOUT = 5  # seconds to drain pipes after killing a process group


def register(mcp: FastMCP) -> None:
    """Register exec, launch, and wait tools on the MCP server."""

    @mcp.tool(name="exec")
    async def shell_exec(command: str, timeout_seconds: float = 30.0) -> dict[str, str | int]:
        """Execute a shell command and return its output.

        Use for any command where you need the text result: listing files,
        installing packages, checking processes, running scripts, etc.

        Do NOT use this for GUI applications — use launch() instead.

        Args:
            command: The bash command to execute (e.g. 'ls -la /tmp').
            timeout_seconds: Maximum execution time in seconds. Defaults to 30.
        """
        proc = await asyncio.create_subprocess_exec(
            "bash", "-c", command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            process_group=0,
        )
        try:
            stdout, stderr = await asyncio.wait_for(
                proc.communicate(), timeout=timeout_seconds,
            )
        except asyncio.TimeoutError:
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            try:
                stdout, stderr = await asyncio.wait_for(
                    proc.communicate(), timeout=_KILL_DRAIN_TIMEOUT,
                )
            except asyncio.TimeoutError:
                # Grandchild escaped the process group and holds pipes open.
                # Force-close pipes and return what we have.
                for stream in (proc.stdout, proc.stderr):
                    if stream:
                        stream.close()
                stdout, stderr = b"", b""
            return {
                "stdout": stdout.decode(errors="replace"),
                "stderr": f"Command timed out after {timeout_seconds}s\n"
                          f"{stderr.decode(errors='replace')}",
                "returncode": -1,
            }

        return {
            "stdout": stdout.decode(errors="replace"),
            "stderr": stderr.decode(errors="replace"),
            "returncode": proc.returncode if proc.returncode is not None else 0,
        }

    @mcp.tool()
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

    @mcp.tool()
    async def wait(milliseconds: int) -> str:
        """Wait for a specified number of milliseconds.

        Useful to let animations finish, pages load, or UI transitions complete
        before taking a screenshot or performing the next action.

        Args:
            milliseconds: Duration to wait.
        """
        await asyncio.sleep(milliseconds / 1000.0)
        return f"Waited {milliseconds}ms"
