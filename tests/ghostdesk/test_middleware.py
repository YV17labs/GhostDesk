# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk._middleware — tool call middleware."""

import logging
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from mcp.server.fastmcp.exceptions import ToolError

from ghostdesk._middleware import _coerce_xy_args, install_middleware

MODULE = "ghostdesk._middleware"


# Tests for _coerce_xy_args function


async def test_coerce_xy_args_with_comma_separated_xy():
    """_coerce_xy_args() splits comma-separated xy values in x parameter."""
    arguments = {"x": "383, 22", "other": "value"}
    result = _coerce_xy_args(arguments)

    assert result["x"] == 383
    assert result["y"] == 22
    assert result["other"] == "value"


async def test_coerce_xy_args_with_comma_separated_xy_no_spaces():
    """_coerce_xy_args() splits comma-separated xy values without spaces."""
    arguments = {"x": "100,200"}
    result = _coerce_xy_args(arguments)

    assert result["x"] == 100
    assert result["y"] == 200


async def test_coerce_xy_args_invalid_string():
    """_coerce_xy_args() returns original arguments if parsing fails."""
    arguments = {"x": "383, invalid"}
    result = _coerce_xy_args(arguments)

    # Should return original arguments unchanged
    assert result == arguments


async def test_coerce_xy_args_no_comma():
    """_coerce_xy_args() returns original arguments if no comma in x."""
    arguments = {"x": "383"}
    result = _coerce_xy_args(arguments)

    assert result == arguments


async def test_coerce_xy_args_not_string():
    """_coerce_xy_args() returns original arguments if x is not a string."""
    arguments = {"x": 383, "y": 22}
    result = _coerce_xy_args(arguments)

    assert result == arguments


async def test_coerce_xy_args_x_none():
    """_coerce_xy_args() returns original arguments if x is None."""
    arguments = {"x": None}
    result = _coerce_xy_args(arguments)

    assert result == arguments


async def test_coerce_xy_args_x_missing():
    """_coerce_xy_args() returns original arguments if x is missing."""
    arguments = {"y": 22}
    result = _coerce_xy_args(arguments)

    assert result == arguments


# Tests for install_middleware _call_tool wrapper


async def test_call_tool_success():
    """_call_tool() logs successful tool calls with elapsed time."""
    mock_mcp = MagicMock()
    mock_original_call_tool = AsyncMock(return_value={"result": "success"})
    mock_mcp.call_tool = mock_original_call_tool
    # Setup the decorator mock
    mock_mcp._mcp_server.call_tool.return_value = lambda fn: None

    with patch(f"{MODULE}.logger") as mock_logger:
        with patch(f"{MODULE}.time.monotonic", side_effect=[0.0, 0.5]):
            # Capture the _call_tool function passed to the decorator
            captured_call_tool = None

            def capture_decorator(fn):
                nonlocal captured_call_tool
                captured_call_tool = fn
                return lambda x: None

            mock_mcp._mcp_server.call_tool.return_value = capture_decorator
            install_middleware(mock_mcp)

            # Call the captured _call_tool function
            result = await captured_call_tool("test_tool", {"x": 10})

            # Verify result is returned
            assert result == {"result": "success"}

            # Verify logging
            mock_logger.info.assert_called_once()
            call_args, call_kwargs = mock_logger.info.call_args
            # Args are: format_string, name, args_str, elapsed
            assert call_args[0] == "%s(%s) → OK (%.1fs)"
            assert call_args[1] == "test_tool"
            assert "x=10" in call_args[2]
            assert call_args[3] == 0.5


async def test_call_tool_tool_error_with_validation_error():
    """_call_tool() formats validation error messages with arguments."""
    mock_mcp = MagicMock()
    tool_error = ToolError("Tool validation error for parameter 'x'")
    mock_original_call_tool = AsyncMock(side_effect=tool_error)
    mock_mcp.call_tool = mock_original_call_tool

    with patch(f"{MODULE}.logger") as mock_logger:
        with patch(f"{MODULE}.time.monotonic", side_effect=[0.0, 0.3]):
            captured_call_tool = None

            def capture_decorator(fn):
                nonlocal captured_call_tool
                captured_call_tool = fn
                return lambda x: None

            mock_mcp._mcp_server.call_tool.return_value = capture_decorator
            install_middleware(mock_mcp)

            # Call should re-raise the formatted ToolError
            with pytest.raises(ToolError) as exc_info:
                await captured_call_tool("click", {"x": "383, 22"})

            # Verify error message was reformatted
            error_msg = str(exc_info.value)
            assert "Invalid arguments for click" in error_msg
            assert "Each parameter must be passed separately" in error_msg
            assert "x=383" in error_msg  # Should show coerced argument
            assert "y=22" in error_msg

            # Verify logging
            mock_logger.error.assert_called_once()
            call_args, call_kwargs = mock_logger.error.call_args
            # Args are: format_string, name, args_str, elapsed, msg
            assert call_args[0] == "%s(%s) → ERROR (%.1fs): %s"
            assert call_args[1] == "click"
            assert "x=383" in call_args[2] and "y=22" in call_args[2]
            assert call_args[3] == 0.3
            assert "Invalid arguments for click" in call_args[4]


async def test_call_tool_tool_error_without_validation():
    """_call_tool() logs ToolError without validation message as-is."""
    mock_mcp = MagicMock()
    tool_error = ToolError("Tool not found")
    mock_original_call_tool = AsyncMock(side_effect=tool_error)
    mock_mcp.call_tool = mock_original_call_tool

    with patch(f"{MODULE}.logger") as mock_logger:
        with patch(f"{MODULE}.time.monotonic", side_effect=[0.0, 0.2]):
            captured_call_tool = None

            def capture_decorator(fn):
                nonlocal captured_call_tool
                captured_call_tool = fn
                return lambda x: None

            mock_mcp._mcp_server.call_tool.return_value = capture_decorator
            install_middleware(mock_mcp)

            # Call should re-raise the original ToolError message
            with pytest.raises(ToolError) as exc_info:
                await captured_call_tool("unknown_tool", {})

            # Verify error message is not reformatted
            error_msg = str(exc_info.value)
            assert error_msg == "Tool not found"

            # Verify logging includes original message
            mock_logger.error.assert_called_once()
            call_args, call_kwargs = mock_logger.error.call_args
            # Args are: format_string, name, args_str, elapsed, msg
            assert call_args[0] == "%s(%s) → ERROR (%.1fs): %s"
            assert call_args[1] == "unknown_tool"
            assert call_args[3] == 0.2
            assert call_args[4] == "Tool not found"


async def test_call_tool_generic_exception():
    """_call_tool() logs generic exceptions with logger.exception and re-raises."""
    mock_mcp = MagicMock()
    generic_error = RuntimeError("Something went wrong")
    mock_original_call_tool = AsyncMock(side_effect=generic_error)
    mock_mcp.call_tool = mock_original_call_tool

    with patch(f"{MODULE}.logger") as mock_logger:
        with patch(f"{MODULE}.time.monotonic", side_effect=[0.0, 0.1]):
            captured_call_tool = None

            def capture_decorator(fn):
                nonlocal captured_call_tool
                captured_call_tool = fn
                return lambda x: None

            mock_mcp._mcp_server.call_tool.return_value = capture_decorator
            install_middleware(mock_mcp)

            # Call should re-raise the original exception
            with pytest.raises(RuntimeError) as exc_info:
                await captured_call_tool("broken_tool", {"param": "value"})

            assert str(exc_info.value) == "Something went wrong"

            # Verify logger.exception was called (includes traceback)
            mock_logger.exception.assert_called_once()
            call_args, call_kwargs = mock_logger.exception.call_args
            # Args are: format_string, name, args_str, elapsed
            assert call_args[0] == "%s(%s) → ERROR (%.1fs)"
            assert call_args[1] == "broken_tool"
            assert "param='value'" in call_args[2]
            assert call_args[3] == 0.1


async def test_call_tool_coerces_arguments():
    """_call_tool() applies _coerce_xy_args before calling original tool."""
    mock_mcp = MagicMock()
    mock_original_call_tool = AsyncMock(return_value=None)
    mock_mcp.call_tool = mock_original_call_tool

    with patch(f"{MODULE}.logger"):
        with patch(f"{MODULE}.time.monotonic", side_effect=[0.0, 0.1]):
            captured_call_tool = None

            def capture_decorator(fn):
                nonlocal captured_call_tool
                captured_call_tool = fn
                return lambda x: None

            mock_mcp._mcp_server.call_tool.return_value = capture_decorator
            install_middleware(mock_mcp)

            await captured_call_tool("click", {"x": "100, 200"})

            # Verify the original tool was called with coerced arguments
            mock_original_call_tool.assert_called_once()
            call_args = mock_original_call_tool.call_args
            assert call_args[0][0] == "click"
            assert call_args[0][1]["x"] == 100
            assert call_args[0][1]["y"] == 200


async def test_call_tool_logs_argument_truncation():
    """_call_tool() truncates long argument values in logs."""
    mock_mcp = MagicMock()
    long_string = "x" * 100
    mock_original_call_tool = AsyncMock(return_value=None)
    mock_mcp.call_tool = mock_original_call_tool

    with patch(f"{MODULE}.logger") as mock_logger:
        with patch(f"{MODULE}.time.monotonic", side_effect=[0.0, 0.1]):
            captured_call_tool = None

            def capture_decorator(fn):
                nonlocal captured_call_tool
                captured_call_tool = fn
                return lambda x: None

            mock_mcp._mcp_server.call_tool.return_value = capture_decorator
            install_middleware(mock_mcp)

            await captured_call_tool("test", {"long_arg": long_string})

            # Verify argument is truncated to 80 chars in log
            mock_logger.info.assert_called_once()
            call_args = mock_logger.info.call_args[0]
            # The repr should be truncated with [: 80]
            assert len(call_args[0]) < 200  # Should not contain full string
