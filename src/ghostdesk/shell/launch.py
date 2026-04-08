# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Shell launch tool — GUI application launcher with process tracking."""

import asyncio
import shlex
import tempfile
from pathlib import Path

LOG_DIR = Path("/tmp/ghostdesk")


async def launch(command: str) -> dict:
    """Launch a GUI application and return its PID and log file path.

    The process runs in the background. Its stdout and stderr are captured
    in a log file under ``/tmp/ghostdesk/proc-<pid>.log``. Use
    ``process_status(pid)`` to check whether it is still running and to
    read its output.

    Returns a dict with:
    - pid: the process ID of the launched application.
    - log_file: path to the file capturing stdout and stderr.
    - action: description of what was launched.

    On failure, returns a dict with a single ``error`` key describing
    what went wrong (invalid syntax, empty command, command not found).
    """
    try:
        parts = shlex.split(command)
    except ValueError as e:
        return {"error": f"Invalid command syntax: {e}"}
    if not parts:
        return {"error": "No command provided"}

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
        return {"error": f"Command not found: {parts[0]}"}

    # Rename to the canonical path now that we have the real PID.
    final_path = LOG_DIR / f"proc-{proc.pid}.log"
    tmp_path.rename(final_path)

    return {
        "pid": proc.pid,
        "log_file": str(final_path),
        "action": f"Launched: {command}",
    }
