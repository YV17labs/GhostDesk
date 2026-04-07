# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk.shell.process_status."""

from pathlib import Path
from unittest.mock import patch

import pytest

from ghostdesk.shell.process_status import _is_running, _read_tail, process_status

MODULE = "ghostdesk.shell.process_status"


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
    result = _read_tail(log, 3)
    assert result == "line3\nline4\nline5"


def test_read_tail_fewer_lines_than_requested(tmp_path):
    """_read_tail returns all lines when file is shorter than requested."""
    log = tmp_path / "test.log"
    log.write_text("only\n")
    result = _read_tail(log, 50)
    assert result == "only"


def test_read_tail_missing_file(tmp_path):
    """_read_tail returns empty string for missing files."""
    result = _read_tail(tmp_path / "missing.log", 10)
    assert result == ""


# --- process_status ---

async def test_process_status_running(tmp_path):
    """process_status returns running=True with log tail for a live process."""
    log = tmp_path / "proc-42.log"
    log.write_text("Starting...\nReady.\n")

    with (
        patch(f"{MODULE}.LOG_DIR", tmp_path),
        patch(f"{MODULE}._is_running", return_value=True),
    ):
        result = await process_status(42, lines=10)

    assert result["pid"] == 42
    assert result["running"] is True
    assert "Ready." in result["tail"]
    assert "proc-42.log" in result["log_file"]


async def test_process_status_exited(tmp_path):
    """process_status returns running=False for a dead process."""
    log = tmp_path / "proc-99.log"
    log.write_text("Error: crash\n")

    with (
        patch(f"{MODULE}.LOG_DIR", tmp_path),
        patch(f"{MODULE}._is_running", return_value=False),
    ):
        result = await process_status(99)

    assert result["running"] is False
    assert "crash" in result["tail"]


async def test_process_status_no_log(tmp_path):
    """process_status returns empty tail when log file is missing."""
    with (
        patch(f"{MODULE}.LOG_DIR", tmp_path),
        patch(f"{MODULE}._is_running", return_value=False),
    ):
        result = await process_status(12345)

    assert result["tail"] == ""
