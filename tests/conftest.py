# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Shared test fixtures."""

from unittest.mock import AsyncMock, patch

import pytest


@pytest.fixture
def mock_cmd_run():
    """Patch ghostdesk.utils.cmd.run to capture commands without executing them.

    Usage:
        async def test_something(mock_cmd_run):
            mock_cmd_run.return_value = "mocked output"
            result = await some_function()
            mock_cmd_run.assert_called_once_with(["xdotool", "click", "1"])
    """
    with patch("ghostdesk.utils.cmd.run", new_callable=AsyncMock) as mock:
        mock.return_value = ""
        yield mock
