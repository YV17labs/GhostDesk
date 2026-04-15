# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Shared test fixtures."""

from unittest.mock import AsyncMock, patch

import pytest


@pytest.fixture
def mock_cmd_run():
    """Patch ghostdesk._cmd.run to capture commands without executing them.

    Usage:
        async def test_something(mock_cmd_run):
            mock_cmd_run.return_value = "mocked output"
            await some_function()
            mock_cmd_run.assert_called_once_with(["swaymsg", "-t", "get_tree"])
    """
    with patch("ghostdesk._cmd.run", new_callable=AsyncMock) as mock:
        mock.return_value = ""
        yield mock
