# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Sway IPC helpers — resilient socket discovery and tree walking.

``$XDG_RUNTIME_DIR`` is often a persistent volume in container deploys,
so dead ``sway-ipc.<uid>.<pid>.sock`` files from previous boots stick
around alongside the live one. We can't trust ``$SWAYSOCK`` (it is
captured at MCP-server start and grows stale when Sway restarts), and
we can't trust filename ordering or mtime — the only authoritative
test is "does this socket actually answer Sway IPC right now?".

The strategy:

1. Glob ``$XDG_RUNTIME_DIR/sway-ipc.*.sock``.
2. Filter to candidates whose embedded PID resolves to a live ``sway``
   process via ``/proc/<pid>/comm``.
3. Probe each survivor with a cheap ``swaymsg -s <sock> -t get_version``.
   First one that succeeds is the live compositor.
4. Cache the winner. On any subsequent ``swaymsg`` failure, invalidate
   the cache and re-discover once before giving up — this is what
   makes us self-heal across Sway restarts.

We always pass ``swaymsg -s <socket>`` explicitly. That bypasses
``$SWAYSOCK`` entirely, so a stale env var inherited from
``run.sh`` can no longer mislead us.
"""

from __future__ import annotations

import asyncio
import glob
import json
import logging
import os
import re
from collections.abc import Callable, Iterator

from ghostdesk._cmd import run as _run_cmd

logger = logging.getLogger("ghostdesk")

_SOCK_RE = re.compile(r"sway-ipc\.\d+\.(\d+)\.sock$")

_cached_sock: str | None = None
_cache_lock = asyncio.Lock()


def _runtime_dir() -> str:
    return os.environ.get("XDG_RUNTIME_DIR", "/run/user/1000")


def _pid_is_sway(pid: int) -> bool:
    try:
        with open(f"/proc/{pid}/comm") as f:
            return f.read().strip() == "sway"
    except OSError:
        return False


def _candidate_socks() -> list[str]:
    """Return socket paths whose embedded PID points at a live sway."""
    out: list[str] = []
    for sock in glob.glob(f"{_runtime_dir()}/sway-ipc.*.sock"):
        m = _SOCK_RE.search(sock)
        if m and _pid_is_sway(int(m.group(1))):
            out.append(sock)
    return out


async def _probe(sock: str) -> bool:
    """Return True iff ``swaymsg -s <sock> -t get_version`` succeeds."""
    try:
        await _run_cmd(
            ["swaymsg", "-s", sock, "-t", "get_version"], timeout=2.0,
        )
    except Exception:
        return False
    return True


async def _discover() -> str | None:
    """Pick a live socket by actually probing each candidate."""
    candidates = _candidate_socks()
    if not candidates:
        logger.warning(
            "sway: no candidate socket in %s with a live sway PID",
            _runtime_dir(),
        )
        return None
    for sock in candidates:
        if await _probe(sock):
            logger.info("sway: using IPC socket %s", sock)
            return sock
    logger.warning(
        "sway: %d candidate socket(s) but none answered get_version: %s",
        len(candidates), candidates,
    )
    return None


async def _resolve(force: bool = False) -> str | None:
    """Return the cached live socket, refreshing it if missing or forced."""
    global _cached_sock
    async with _cache_lock:
        if force or _cached_sock is None:
            _cached_sock = await _discover()
        return _cached_sock


async def _invalidate() -> None:
    global _cached_sock
    async with _cache_lock:
        _cached_sock = None


async def _swaymsg(args: list[str], timeout: float = 5.0) -> str:
    """Run ``swaymsg`` against the live socket, with one self-healing retry.

    On the first failure we drop the cached socket and try again — Sway
    may have restarted under us, leaving the old socket file dead.
    """
    sock = await _resolve()
    if sock is None:
        raise RuntimeError("sway: no live IPC socket found")
    try:
        return await _run_cmd(
            ["swaymsg", "-s", sock, *args], timeout=timeout,
        )
    except Exception:
        await _invalidate()
        sock = await _resolve(force=True)
        if sock is None:
            raise
        return await _run_cmd(
            ["swaymsg", "-s", sock, *args], timeout=timeout,
        )


async def get_tree() -> dict | None:
    """Return the parsed Sway tree, or ``None`` on IPC/JSON failure."""
    try:
        raw = await _swaymsg(["-t", "get_tree"])
    except Exception:
        logger.exception("sway: get_tree failed")
        return None
    try:
        return json.loads(raw)
    except json.JSONDecodeError:
        logger.exception("sway: malformed get_tree JSON")
        return None


async def kill_view(con_id: int) -> None:
    """Send a graceful close request to one Sway view by container id."""
    await _swaymsg([f"[con_id={con_id}]", "kill"])


def iter_views(tree: dict) -> Iterator[dict]:
    """Yield every node in ``tree`` that has a ``pid`` (i.e. a client surface)."""
    if tree.get("pid"):
        yield tree
    for child in tree.get("nodes", ()) or ():
        yield from iter_views(child)
    for child in tree.get("floating_nodes", ()) or ():
        yield from iter_views(child)


def collect_views[T](tree: dict, project: Callable[[dict], T]) -> list[T]:
    """Apply ``project`` to every view node and collect the results."""
    return [project(view) for view in iter_views(tree)]
