# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Async subprocess runner for system commands (swaymsg, grim, wl-paste, etc.)."""

import asyncio


async def run(cmd: list[str], timeout: float = 10.0) -> str:
    """Execute a command asynchronously and return its stdout.

    Args:
        cmd: Command and arguments as a list (no shell expansion).
        timeout: Maximum seconds to wait before killing the process.

    Raises:
        RuntimeError: If the command exits with a non-zero return code.
        TimeoutError: If the command exceeds the timeout.
    """
    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )
    try:
        stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout)
    except asyncio.TimeoutError:
        proc.kill()
        await proc.communicate()
        raise TimeoutError(f"Command timed out after {timeout}s: {cmd}")

    if proc.returncode != 0:
        raise RuntimeError(stderr.decode().strip() or f"Command failed with code {proc.returncode}: {cmd}")

    return stdout.decode().strip()
