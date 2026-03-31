# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Visible interactive elements via AT-SPI (best-effort)."""

from __future__ import annotations

import asyncio
import json
import os
from pathlib import Path

# src/ directory so system Python can find the ghostdesk package.
_SRC_DIR = str(Path(__file__).resolve().parents[2])


async def get_clickables() -> list[dict]:
    """Return visible interactive elements with their coordinates.

    Calls the AT-SPI query via system Python (which has python3-gi).
    Returns an empty list on any failure — never blocks the screenshot.
    """
    try:
        proc = await asyncio.create_subprocess_exec(
            "/usr/bin/python3", "-m", "ghostdesk._atspi",
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env={
                **os.environ,
                "DISPLAY": os.environ.get("DISPLAY", ":99"),
                "PYTHONPATH": _SRC_DIR,
            },
        )
        stdout, _ = await asyncio.wait_for(proc.communicate(), timeout=10.0)
        return json.loads(stdout.decode())
    except Exception:
        return []
