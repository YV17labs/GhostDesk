# Copyright (c) 2026 YV17 — MIT License
"""Shell exec tool — run commands and capture output."""

import asyncio
import os
import signal

from mcp.server.fastmcp import FastMCP

_KILL_DRAIN_TIMEOUT = 5  # seconds to drain pipes after killing a process group


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


def register(mcp: FastMCP) -> None:
    """Register the exec tool."""
    mcp.tool(name="exec")(shell_exec)
