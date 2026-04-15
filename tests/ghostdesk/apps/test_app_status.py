# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk.apps.app_status."""

from pathlib import Path
from unittest.mock import patch

import pytest

from ghostdesk.apps.app_launch import _launched_pids
from ghostdesk.apps.app_status import _is_running, _read_tail, app_status

MODULE = "ghostdesk.apps.app_status"


@pytest.fixture(autouse=True)
def _clear_pid_registry():
    """Ensure the PID registry is clean before and after every test."""
    _launched_pids.clear()
    yield
    _launched_pids.clear()


# --- _is_running ---

def test_is_running_true():
    """_is_running returns True when os.kill succeeds (process alive)."""
    with patch("os.kill") as mock_kill:
        assert _is_running(1234) is True
        mock_kill.assert_called_once_with(1234, 0)


def test_is_running_false():
    """_is_running returns False when process doesn't exist."""
    with patch("os.kill", side_effect=ProcessLookupError):
        assert _is_running(1234) is False


def test_is_running_permission_error():
    """_is_running returns True when process exists but we can't signal it."""
    with patch("os.kill", side_effect=PermissionError):
        assert _is_running(9999) is True


# --- _read_tail ---

def test_read_tail_returns_last_lines(tmp_path):
    """_read_tail returns the last N lines of a file."""
    log = tmp_path / "test.log"
    log.write_text("line1\nline2\nline3\nline4\nline5\n")
    assert _read_tail(log, 3) == "line3\nline4\nline5"


def test_read_tail_fewer_lines_than_requested(tmp_path):
    """_read_tail returns all lines when file is shorter than requested."""
    log = tmp_path / "test.log"
    log.write_text("only\n")
    assert _read_tail(log, 50) == "only"


def test_read_tail_missing_file(tmp_path):
    """_read_tail returns empty string for missing files."""
    assert _read_tail(tmp_path / "missing.log", 10) == ""


# --- PID guard ---

async def test_app_status_rejects_unregistered_pid():
    """app_status() returns an error for PIDs not launched by ghostdesk."""
    result = await app_status(1)  # PID 1 = init, never launched by us
    assert "error" in result
    assert "not launched by this session" in result["error"]


async def test_app_status_rejects_arbitrary_pid():
    """app_status() rejects any PID not in the registry."""
    result = await app_status(99999)
    assert "error" in result


# --- app_status with registered PID ---

async def test_app_status_running(tmp_path):
    """app_status returns running=True with log tail for a registered PID."""
    _launched_pids.add(42)
    log = tmp_path / "proc-42.log"
    log.write_text("Starting...\nReady.\n")

    with (
        patch(f"{MODULE}.LOG_DIR", tmp_path),
        patch(f"{MODULE}._is_running", return_value=True),
    ):
        result = await app_status(42, lines=10)

    assert result["pid"] == 42
    assert result["running"] is True
    assert "Ready." in result["tail"]
    assert "proc-42.log" in result["log_file"]


async def test_app_status_exited(tmp_path):
    """app_status returns running=False for a dead registered process."""
    _launched_pids.add(99)
    log = tmp_path / "proc-99.log"
    log.write_text("Error: crash\n")

    with (
        patch(f"{MODULE}.LOG_DIR", tmp_path),
        patch(f"{MODULE}._is_running", return_value=False),
    ):
        result = await app_status(99)

    assert result["running"] is False
    assert "crash" in result["tail"]


async def test_app_status_no_log(tmp_path):
    """app_status returns empty tail when log file is missing."""
    _launched_pids.add(12345)
    with (
        patch(f"{MODULE}.LOG_DIR", tmp_path),
        patch(f"{MODULE}._is_running", return_value=False),
    ):
        result = await app_status(12345)

    assert result["tail"] == ""
