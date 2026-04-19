# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Apps app_status tool — check on a launched application."""

import os
from pathlib import Path

from ghostdesk.apps.app_launch import LOG_DIR, _launched_pids

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


async def app_status(pid: int, lines: int = _DEFAULT_TAIL) -> dict:
    """Check on an app started by ``app_launch()`` in this session.

    Reports whether the process is still alive and returns the tail of
    its captured stdout/stderr. Only PIDs returned by ``app_launch()``
    are accepted; any other PID is rejected.

    Use this to confirm an app crashed (tail the log for a traceback)
    or to watch a long-running app write progress to its output.

    Args:
        pid: Process ID returned by ``app_launch()``.
        lines: Number of trailing log lines to return (default 50).

    Returns a dict with:
    - pid: the process ID.
    - running: whether the process is still alive.
    - log_file: path to the log file.
    - tail: the last *lines* lines of stdout/stderr output.

    On failure, returns a dict with a single ``error`` key.
    """
    if pid not in _launched_pids:
        return {
            "error": (
                f"PID {pid} was not launched by this session. "
                "Use app_launch() first."
            )
        }

    log_path = LOG_DIR / f"proc-{pid}.log"
    return {
        "pid": pid,
        "running": _is_running(pid),
        "log_file": str(log_path),
        "tail": _read_tail(log_path, lines),
    }
