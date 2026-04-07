# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Process status tool — check on a launched application."""

import os
from pathlib import Path

from ghostdesk.shell.launch import LOG_DIR

# Maximum number of log lines returned by default.
_DEFAULT_TAIL = 50


def _is_running(pid: int) -> bool:
    """Check whether a process is still alive."""
    try:
        os.kill(pid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        # Process exists but we can't signal it — still counts as running.
        return True


def _read_tail(path: Path, lines: int) -> str:
    """Read the last *lines* lines from a file, or '' if missing."""
    try:
        all_lines = path.read_text(errors="replace").splitlines()
        return "\n".join(all_lines[-lines:])
    except FileNotFoundError:
        return ""


async def process_status(pid: int, lines: int = _DEFAULT_TAIL) -> dict:
    """Check whether a launched process is still running and read its logs.

    Args:
        pid: Process ID returned by ``launch()``.
        lines: Number of trailing log lines to return (default 50).

    Returns a dict with:
    - pid: the process ID.
    - running: whether the process is still alive.
    - log_file: path to the log file.
    - tail: the last *lines* lines of stdout/stderr output.
    """
    log_path = LOG_DIR / f"proc-{pid}.log"
    return {
        "pid": pid,
        "running": _is_running(pid),
        "log_file": str(log_path),
        "tail": _read_tail(log_path, lines),
    }
