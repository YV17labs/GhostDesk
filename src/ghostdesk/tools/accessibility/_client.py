# Copyright (c) 2026 YV17 — MIT License
"""AT-SPI subprocess client — shared by read, interact, click, and wait_for modules."""

import asyncio
import json
import os
import sys

from ghostdesk.roles import INTERACTIVE_ROLE_NAMES as VALID_ROLES  # noqa: F401 — re-exported

# src/ directory — needed so /usr/bin/python3 can find the ghostdesk package.
# Uses the actual install path of this package rather than fragile parent traversal.
_SRC_DIR = str(os.path.dirname(sys.modules["ghostdesk"].__path__[0]))


async def run_atspi(
    command: str,
    args: list[str] | None = None,
    timeout: float = 15.0,
) -> dict | list:
    """Call the AT-SPI query helper with a subcommand and return parsed output."""
    cmd = ["/usr/bin/python3", "-m", "ghostdesk.atspi", command]
    if args:
        cmd.extend(args)

    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
        env={
            **os.environ,
            "DISPLAY": os.environ.get("DISPLAY", ":99"),
            "PYTHONPATH": _SRC_DIR,
        },
    )
    stdout, stderr = await asyncio.wait_for(proc.communicate(), timeout=timeout)

    if proc.returncode != 0:
        msg = stderr.decode(errors="replace").strip()
        raise RuntimeError(f"AT-SPI query failed: {msg}")

    data = json.loads(stdout.decode())
    return data
