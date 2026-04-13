# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Shared .desktop file parser — source of truth for known GUI apps."""

import configparser
from pathlib import Path

_APPS_DIR = Path("/usr/share/applications")


def _parse_exec(exec_field: str) -> str:
    """Extract the executable basename from a .desktop Exec= field.

    Strips field codes (``%u``, ``%F`` …) and environment variable
    assignments (``KEY=value``) so that both of the following return
    ``"firefox"``:

        /usr/bin/firefox %U
        env MOZ_ENABLE_WAYLAND=1 /usr/bin/firefox %u
    """
    for token in exec_field.split():
        if token.startswith("%"):   # field code
            continue
        if "=" in token:            # env var assignment
            continue
        return Path(token).name     # basename of the real executable
    return ""


def _is_launchable(entry: configparser.SectionProxy) -> bool:
    """Return True if this .desktop entry represents a visible GUI app."""
    return (
        entry.get("Type") == "Application"
        and not entry.getboolean("NoDisplay", fallback=False)
        and not entry.getboolean("Hidden", fallback=False)
    )


def known_executables() -> frozenset[str]:
    """Return the set of executable basenames for all installed GUI apps.

    Used by ``app_launch`` to validate that a command is a known desktop
    application before executing it.
    """
    exes: set[str] = set()
    for path in _APPS_DIR.glob("*.desktop"):
        cp = configparser.ConfigParser(interpolation=None)
        cp.read(path)
        if "Desktop Entry" not in cp:
            continue
        entry = cp["Desktop Entry"]
        if not _is_launchable(entry):
            continue
        exe = _parse_exec(entry.get("Exec", ""))
        if exe:
            exes.add(exe)
    return frozenset(exes)


def desktop_apps() -> list[dict]:
    """Return all installed GUI apps as a list of ``{name, exec}`` dicts."""
    apps = []
    for path in _APPS_DIR.glob("*.desktop"):
        cp = configparser.ConfigParser(interpolation=None)
        cp.read(path)
        if "Desktop Entry" not in cp:
            continue
        entry = cp["Desktop Entry"]
        if not _is_launchable(entry):
            continue
        exec_raw = entry.get("Exec", "")
        exe = _parse_exec(exec_raw) if exec_raw else path.stem
        apps.append({
            "name": entry.get("Name", path.stem),
            "exec": exe,
        })
    return sorted(apps, key=lambda a: a["name"].lower())
