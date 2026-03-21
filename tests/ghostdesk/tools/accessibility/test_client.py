# Copyright (c) 2026 YV17 — MIT License
"""Tests for ghostdesk.tools.accessibility._client."""

import asyncio
import json
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from ghostdesk.tools.accessibility._client import run_atspi

MODULE = "ghostdesk.tools.accessibility._client"


def _make_proc(stdout: bytes = b"", stderr: bytes = b"", returncode: int = 0):
    """Create a mock subprocess."""
    proc = MagicMock()
    proc.returncode = returncode
    proc.communicate = AsyncMock(return_value=(stdout, stderr))
    return proc


async def test_run_atspi_success():
    data = [{"role": "button", "name": "OK"}]
    proc = _make_proc(stdout=json.dumps(data).encode(), returncode=0)

    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock, return_value=proc):
        result = await run_atspi("elements", ["--max", "10"])

    assert result == data


async def test_run_atspi_success_dict():
    data = {"name": "Submit", "role": "button"}
    proc = _make_proc(stdout=json.dumps(data).encode(), returncode=0)

    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock, return_value=proc):
        result = await run_atspi("find", ["Submit"])

    assert result == data


async def test_run_atspi_no_args():
    data = {"status": "ok"}
    proc = _make_proc(stdout=json.dumps(data).encode(), returncode=0)

    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock, return_value=proc) as mock_exec:
        result = await run_atspi("elements")

    # Should not include extra args
    call_args = mock_exec.call_args[0]
    assert call_args == ("/usr/bin/python3", "-m", "ghostdesk.atspi", "elements")
    assert result == data


async def test_run_atspi_nonzero_returncode():
    proc = _make_proc(stderr=b"No display found", returncode=1)

    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock, return_value=proc):
        with pytest.raises(RuntimeError, match="AT-SPI query failed: No display found"):
            await run_atspi("elements")


async def test_run_atspi_error_in_json():
    data = {"error": "Element not found"}
    proc = _make_proc(stdout=json.dumps(data).encode(), returncode=0)

    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock, return_value=proc):
        with pytest.raises(RuntimeError, match="Element not found"):
            await run_atspi("find", ["missing"])


async def test_run_atspi_timeout():
    proc = _make_proc()
    proc.communicate = AsyncMock(side_effect=asyncio.TimeoutError)

    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock, return_value=proc):
        with pytest.raises(asyncio.TimeoutError):
            await run_atspi("elements", timeout=0.1)


async def test_run_atspi_error_key_in_list_not_raised():
    """An 'error' key only triggers RuntimeError when result is a dict."""
    data = [{"error": "this is fine in a list"}]
    proc = _make_proc(stdout=json.dumps(data).encode(), returncode=0)

    with patch("asyncio.create_subprocess_exec", new_callable=AsyncMock, return_value=proc):
        result = await run_atspi("elements")

    assert result == data
