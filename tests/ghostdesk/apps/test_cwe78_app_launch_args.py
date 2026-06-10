# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""CWE-78 regression: app_launch must reject arbitrary arguments.

The allowlist check validates only the executable basename but passes all
remaining tokens to ``asyncio.create_subprocess_exec``.  An attacker (or a
prompt-injected LLM) can exploit this to pass dangerous arguments — e.g.
``gnome-terminal -- /bin/sh -c 'payload'`` — even though ``gnome-terminal``
is in the allowlist.

The contract (see ``app_list()`` docstring and server instructions) is
clear: the ``exec`` field returned by ``app_list()`` is the *only string*
``app_launch()`` will accept, and that field is always a bare executable
basename — never a command with arguments.
"""

from unittest.mock import AsyncMock, patch

import pytest

from ghostdesk.apps.app_launch import _launched_pids, app_launch

MODULE = "ghostdesk.apps.app_launch"


@pytest.fixture(autouse=True)
def _clear_pid_registry():
    _launched_pids.clear()
    yield
    _launched_pids.clear()


@pytest.fixture
def _allow_terminals():
    """Allowlist several common GUI apps for argument-injection tests."""
    with patch(
        f"{MODULE}.known_executables",
        return_value=frozenset({
            "firefox", "gnome-terminal", "gedit", "xterm",
        }),
    ):
        yield


# --- CWE-78: argument injection variants ---

@pytest.mark.usefixtures("_allow_terminals")
class TestArgumentInjectionRejected:
    """app_launch must reject any command containing extra arguments."""

    async def test_rejects_url_argument(self):
        result = await app_launch("firefox https://evil.example.com")
        assert "error" in result

    async def test_rejects_flag_argument(self):
        result = await app_launch("firefox --headless")
        assert "error" in result

    async def test_rejects_gnome_terminal_exec(self):
        result = await app_launch("gnome-terminal -- /bin/sh -c 'id > /tmp/pwn'")
        assert "error" in result

    async def test_rejects_xterm_exec(self):
        result = await app_launch("xterm -e /bin/sh")
        assert "error" in result

    async def test_rejects_full_path_with_args(self):
        result = await app_launch("/usr/bin/firefox --screenshot /etc/shadow")
        assert "error" in result

    async def test_bare_executable_still_accepted(self):
        """Ensure the fix doesn't break the normal case."""
        mock_proc = AsyncMock()
        mock_proc.pid = 42
        with patch(
            "asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
            return_value=mock_proc,
        ), patch(f"{MODULE}.LOG_DIR", __import__("pathlib").Path("/tmp/ghostdesk-test")):
            result = await app_launch("firefox")
        assert "pid" in result

    async def test_full_path_bare_accepted(self):
        """Full path to a known executable without args should still work."""
        mock_proc = AsyncMock()
        mock_proc.pid = 43
        with patch(
            "asyncio.create_subprocess_exec",
            new_callable=AsyncMock,
            return_value=mock_proc,
        ), patch(f"{MODULE}.LOG_DIR", __import__("pathlib").Path("/tmp/ghostdesk-test")):
            result = await app_launch("/usr/bin/firefox")
        assert "pid" in result
