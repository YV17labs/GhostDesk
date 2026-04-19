# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Apps app_launch tool — GUI application launcher with process tracking."""

import asyncio
import shlex
import tempfile
from pathlib import Path

from mcp.server.fastmcp import Context

from ghostdesk.apps._desktop import known_executables

LOG_DIR = Path("/tmp/ghostdesk")

# Registry of PIDs launched by this session — checked by app_status.
_launched_pids: set[int] = set()


async def app_launch(command: str, ctx: Context | None = None) -> dict:
    """Launch a desktop GUI application and return its PID and log file path.

    Only applications listed by ``app_list()`` are accepted. The process
    runs in the background; its stdout and stderr are captured in a log
    file under ``/tmp/ghostdesk/proc-<pid>.log``. Use ``app_status(pid)``
    to check whether it is still running and to read its output.

    Returns a dict with:
    - pid: the process ID of the launched application.
    - log_file: path to the file capturing stdout and stderr.
    - action: description of what was launched.

    On failure, returns a dict with a single ``error`` key describing
    what went wrong (not a GUI app, invalid syntax, command not found).
    """
    try:
        parts = shlex.split(command)
    except ValueError as e:
        return {"error": f"Invalid command syntax: {e}"}
    if not parts:
        return {"error": "No command provided"}

    # Security: reject anything that is not a known desktop GUI app.
    exe = Path(parts[0]).name
    if exe not in known_executables():
        return {
            "error": (
                f"{exe!r} is not a known GUI app. "
                "Call app_list() to see what is available."
            )
        }

    LOG_DIR.mkdir(parents=True, exist_ok=True)

    # Use a unique temp file to avoid races between concurrent launches.
    fd, tmp_path_str = tempfile.mkstemp(dir=LOG_DIR, prefix="proc-", suffix=".log")
    tmp_path = Path(tmp_path_str)

    try:
        log_file = open(fd, "w")  # noqa: SIM115
        proc = await asyncio.create_subprocess_exec(
            *parts,
            stdout=log_file,
            stderr=log_file,
            process_group=0,
        )
    except FileNotFoundError:
        log_file.close()
        tmp_path.unlink(missing_ok=True)
        if ctx is not None:
            await ctx.error(f"app_launch: command not found: {parts[0]}")
        return {"error": f"Command not found: {parts[0]}"}

    # Rename to the canonical path now that we have the real PID.
    final_path = LOG_DIR / f"proc-{proc.pid}.log"
    tmp_path.rename(final_path)

    _launched_pids.add(proc.pid)

    if ctx is not None:
        await ctx.info(f"app_launch: {command} (pid={proc.pid}, log={final_path})")

    return {
        "pid": proc.pid,
        "log_file": str(final_path),
        "action": f"Launched: {command}",
    }
