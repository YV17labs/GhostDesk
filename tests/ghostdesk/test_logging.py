# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk._logging — logging configuration."""

import logging
from unittest.mock import MagicMock, patch

import pytest

from ghostdesk._logging import configure_logging

MODULE = "ghostdesk._logging"


async def test_configure_logging_sets_root_handlers():
    """configure_logging() sets up a single StreamHandler on the root logger."""
    with patch(f"{MODULE}.logging.StreamHandler") as mock_handler_class:
        mock_handler = MagicMock()
        mock_handler_class.return_value = mock_handler

        configure_logging()

        root = logging.getLogger()
        assert root.handlers == [mock_handler]


async def test_configure_logging_sets_root_level():
    """configure_logging() sets root logger level to INFO."""
    root = logging.getLogger()
    original_level = root.level

    try:
        configure_logging()
        assert root.level == logging.INFO
    finally:
        root.setLevel(original_level)


async def test_configure_logging_formatter_created():
    """configure_logging() creates a formatter with correct format and datefmt."""
    with patch(f"{MODULE}.logging.Formatter") as mock_formatter_class:
        configure_logging()

        mock_formatter_class.assert_called_once()
        call_args = mock_formatter_class.call_args
        assert call_args[0][0] == "%(asctime)s  %(levelname)-5s  %(name)s  %(message)s"
        assert call_args[1]["datefmt"] == "%H:%M:%S"


async def test_configure_logging_formatters_exist():
    """configure_logging() updates uvicorn formatters when 'formatters' key exists."""
    # Mock uvicorn.config.LOGGING_CONFIG with formatters
    mock_formatters = {
        "default": {"format": "old_format"},
        "access": {"format": "old_access_format"},
    }

    with patch(f"{MODULE}.uvicorn.config.LOGGING_CONFIG", {"formatters": mock_formatters}):
        with patch(f"{MODULE}.logger") as mock_logger:
            configure_logging()

            # No warning should be logged
            mock_logger.warning.assert_not_called()

            # Formatters should be updated
            assert mock_formatters["default"]["fmt"] == "%(asctime)s  %(levelname)-5s  %(name)s  %(message)s"
            assert mock_formatters["default"]["datefmt"] == "%H:%M:%S"
            assert mock_formatters["access"]["fmt"] == "%(asctime)s  %(levelname)-5s  %(name)s  %(message)s"
            assert mock_formatters["access"]["datefmt"] == "%H:%M:%S"


async def test_configure_logging_formatters_missing():
    """configure_logging() logs warning when 'formatters' key is missing."""
    # Mock uvicorn.config.LOGGING_CONFIG without formatters
    with patch(f"{MODULE}.uvicorn.config.LOGGING_CONFIG", {}):
        with patch(f"{MODULE}.logger") as mock_logger:
            configure_logging()

            # Warning should be logged
            mock_logger.warning.assert_called_once()
            call_args = mock_logger.warning.call_args[0][0]
            assert "uvicorn.config.LOGGING_CONFIG missing 'formatters' key" in call_args
            assert "log format may not be unified" in call_args


async def test_configure_logging_formatters_empty_dict():
    """configure_logging() handles empty formatters dict without error."""
    # Mock uvicorn.config.LOGGING_CONFIG with empty formatters dict
    with patch(f"{MODULE}.uvicorn.config.LOGGING_CONFIG", {"formatters": {}}):
        with patch(f"{MODULE}.logger") as mock_logger:
            configure_logging()

            # No warning should be logged
            mock_logger.warning.assert_not_called()


async def test_configure_logging_formatters_multiple_entries():
    """configure_logging() updates all formatters in the dict."""
    mock_formatters = {
        "default": {"format": "fmt1"},
        "access": {"format": "fmt2"},
        "custom": {"format": "fmt3"},
    }

    with patch(f"{MODULE}.uvicorn.config.LOGGING_CONFIG", {"formatters": mock_formatters}):
        with patch(f"{MODULE}.logger"):
            configure_logging()

            # All formatters should be updated
            for name, formatter in mock_formatters.items():
                assert formatter["fmt"] == "%(asctime)s  %(levelname)-5s  %(name)s  %(message)s"
                assert formatter["datefmt"] == "%H:%M:%S"
                assert "format" in formatter  # Original key should still exist
