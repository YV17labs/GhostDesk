# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.apps._desktop — .desktop file parsing."""

from pathlib import Path

import pytest

from ghostdesk.apps._desktop import (
    _parse_exec,
    desktop_apps,
    known_executables,
)


# --- _parse_exec ---

class TestParseExec:
    def test_simple_command(self):
        assert _parse_exec("firefox") == "firefox"

    def test_strips_field_codes(self):
        assert _parse_exec("firefox %u") == "firefox"
        assert _parse_exec("gedit %F") == "gedit"

    def test_strips_full_path(self):
        assert _parse_exec("/usr/bin/firefox %U") == "firefox"

    def test_strips_env_var_assignments(self):
        assert _parse_exec("env MOZ_ENABLE_WAYLAND=1 firefox") == "firefox"
        assert _parse_exec("env MOZ_ENABLE_WAYLAND=1 /usr/bin/firefox %u") == "firefox"

    def test_empty_string(self):
        assert _parse_exec("") == ""

    def test_only_field_codes(self):
        assert _parse_exec("%u %F") == ""


# --- known_executables / desktop_apps ---

def _write_desktop(tmp_path: Path, name: str, content: str) -> Path:
    p = tmp_path / f"{name}.desktop"
    p.write_text(content)
    return p


FIREFOX_DESKTOP = """\
[Desktop Entry]
Type=Application
Name=Firefox
Exec=firefox %u
"""

HIDDEN_DESKTOP = """\
[Desktop Entry]
Type=Application
Name=Hidden App
Exec=hiddenapp
NoDisplay=true
"""

NON_APP_DESKTOP = """\
[Desktop Entry]
Type=Link
Name=Some Link
Exec=somelink
"""

GIMP_DESKTOP = """\
[Desktop Entry]
Type=Application
Name=GIMP
Exec=gimp %U
Hidden=false
"""


@pytest.fixture
def apps_dir(tmp_path, monkeypatch):
    """Point _desktop._APPS_DIR at a temp directory."""
    import ghostdesk.apps._desktop as mod
    monkeypatch.setattr(mod, "_APPS_DIR", tmp_path)
    return tmp_path


def test_known_executables_includes_visible_apps(apps_dir):
    _write_desktop(apps_dir, "firefox", FIREFOX_DESKTOP)
    assert "firefox" in known_executables()


def test_known_executables_excludes_nodisplay(apps_dir):
    _write_desktop(apps_dir, "hidden", HIDDEN_DESKTOP)
    assert "hiddenapp" not in known_executables()


def test_known_executables_excludes_non_application(apps_dir):
    _write_desktop(apps_dir, "link", NON_APP_DESKTOP)
    assert "somelink" not in known_executables()


def test_known_executables_multiple(apps_dir):
    _write_desktop(apps_dir, "firefox", FIREFOX_DESKTOP)
    _write_desktop(apps_dir, "gimp", GIMP_DESKTOP)
    exes = known_executables()
    assert "firefox" in exes
    assert "gimp" in exes


def test_desktop_apps_returns_sorted(apps_dir):
    _write_desktop(apps_dir, "gimp", GIMP_DESKTOP)
    _write_desktop(apps_dir, "firefox", FIREFOX_DESKTOP)
    apps = desktop_apps()
    names = [a["name"] for a in apps]
    assert names == sorted(names, key=str.lower)


def test_desktop_apps_excludes_nodisplay(apps_dir):
    _write_desktop(apps_dir, "hidden", HIDDEN_DESKTOP)
    assert all(a["name"] != "Hidden App" for a in desktop_apps())


def test_desktop_apps_exec_field(apps_dir):
    _write_desktop(apps_dir, "firefox", FIREFOX_DESKTOP)
    apps = desktop_apps()
    assert apps[0]["exec"] == "firefox"


def test_desktop_apps_empty_dir(apps_dir):
    assert desktop_apps() == []
