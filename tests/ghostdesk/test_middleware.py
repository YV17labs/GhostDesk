# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Tests for ghostdesk._middleware — coord normalisation, coercion, logging."""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from mcp.server.fastmcp.exceptions import ToolError

from ghostdesk._middleware import (
    _coerce_xy_args,
    _normalise_input_coords,
    install_middleware,
)

MODULE = "ghostdesk._middleware"

# With SCREEN_WIDTH=1280, SCREEN_HEIGHT=1024 and MODEL_SPACE=1000:
#   model x=500 → pixels 640
#   model y=500 → pixels 512
#   pixels 640   → model 500
#   pixels 512   → model 500


# ---------------------------------------------------------------------------
# _coerce_xy_args
# ---------------------------------------------------------------------------

def test_coerce_xy_splits_comma_string():
    args = _coerce_xy_args({"x": "383, 22", "other": "value"})
    assert args["x"] == 383
    assert args["y"] == 22
    assert args["other"] == "value"


def test_coerce_xy_splits_no_spaces():
    args = _coerce_xy_args({"x": "100,200"})
    assert args["x"] == 100
    assert args["y"] == 200


def test_coerce_xy_invalid_string_passthrough():
    args = _coerce_xy_args({"x": "383, invalid"})
    assert args == {"x": "383, invalid"}


def test_coerce_xy_no_comma_passthrough():
    args = _coerce_xy_args({"x": "383"})
    assert args == {"x": "383"}


def test_coerce_xy_non_string_passthrough():
    args = _coerce_xy_args({"x": 383, "y": 22})
    assert args == {"x": 383, "y": 22}


def test_coerce_xy_missing_x():
    args = _coerce_xy_args({"y": 22})
    assert args == {"y": 22}


# ---------------------------------------------------------------------------
# _normalise_input_coords
# ---------------------------------------------------------------------------

def test_normalise_input_xy_pair():
    args = _normalise_input_coords({"x": 500, "y": 500})
    assert args["x"] == 640
    assert args["y"] == 512


def test_normalise_input_drag_endpoints():
    args = _normalise_input_coords({"from_x": 0, "from_y": 0, "to_x": 1000, "to_y": 1000})
    assert args["from_x"] == 0
    assert args["from_y"] == 0
    assert args["to_x"] == 1280
    assert args["to_y"] == 1024


def test_normalise_input_region():
    args = _normalise_input_coords({
        "region": {"x": 500, "y": 500, "width": 500, "height": 500},
    })
    assert args["region"] == {"x": 640, "y": 512, "width": 640, "height": 512}


def test_normalise_input_preserves_other_args():
    args = _normalise_input_coords({"x": 500, "y": 500, "button": "left", "text": "hi"})
    assert args["button"] == "left"
    assert args["text"] == "hi"


def test_normalise_input_disabled(monkeypatch):
    """When coord normalisation is disabled, args pass through unchanged."""
    monkeypatch.setattr("ghostdesk._coords.MODEL_SPACE", 0)
    args = _normalise_input_coords({"x": 500, "y": 500})
    assert args["x"] == 500
    assert args["y"] == 500


# ---------------------------------------------------------------------------
# install_middleware / _call_tool wrapper
# ---------------------------------------------------------------------------


def _install_and_capture_call_tool(mock_mcp) -> tuple:
    """Install middleware and return the captured _call_tool function."""
    captured: dict = {}

    def capture_decorator(fn):
        captured["fn"] = fn
        return lambda x: None

    mock_mcp._mcp_server.call_tool.return_value = capture_decorator
    install_middleware(mock_mcp)
    return captured["fn"]


async def test_call_tool_success_path():
    """_call_tool logs success, returns result, normalises input coords."""
    mock_mcp = MagicMock()
    mock_original = AsyncMock(return_value="ok")
    mock_mcp.call_tool = mock_original

    with patch(f"{MODULE}.logger") as mock_logger:
        with patch(f"{MODULE}.time.monotonic", side_effect=[0.0, 0.5]):
            call_tool = _install_and_capture_call_tool(mock_mcp)
            # User sends model x=500 y=500 (centre)
            result = await call_tool("test_tool", {"x": 500, "y": 500})

            # Original tool received pixel coords
            args_sent_to_tool = mock_original.await_args_list[0].args[1]
            assert args_sent_to_tool["x"] == 640
            assert args_sent_to_tool["y"] == 512

            assert result == "ok"
            mock_logger.info.assert_called_once()


async def test_call_tool_validation_error_reformatted():
    """Validation errors are reformatted with the coerced arg summary."""
    mock_mcp = MagicMock()
    mock_mcp.call_tool = AsyncMock(side_effect=ToolError("validation error: x must be int"))

    with patch(f"{MODULE}.logger"):
        with patch(f"{MODULE}.time.monotonic", side_effect=[0.0, 0.3]):
            call_tool = _install_and_capture_call_tool(mock_mcp)
            with pytest.raises(ToolError) as exc_info:
                await call_tool("click", {"x": "100, 200"})
            msg = str(exc_info.value)
            assert "Invalid arguments for click" in msg
            assert "Each parameter must be passed separately" in msg


async def test_call_tool_tool_error_passthrough():
    """Non-validation ToolErrors keep their original message."""
    mock_mcp = MagicMock()
    mock_mcp.call_tool = AsyncMock(side_effect=ToolError("Tool not found"))

    with patch(f"{MODULE}.logger"):
        with patch(f"{MODULE}.time.monotonic", side_effect=[0.0, 0.2]):
            call_tool = _install_and_capture_call_tool(mock_mcp)
            with pytest.raises(ToolError) as exc_info:
                await call_tool("unknown_tool", {})
            assert str(exc_info.value) == "Tool not found"


async def test_call_tool_generic_exception_logged():
    """Generic exceptions use logger.exception and re-raise."""
    mock_mcp = MagicMock()
    mock_mcp.call_tool = AsyncMock(side_effect=RuntimeError("boom"))

    with patch(f"{MODULE}.logger") as mock_logger:
        with patch(f"{MODULE}.time.monotonic", side_effect=[0.0, 0.1]):
            call_tool = _install_and_capture_call_tool(mock_mcp)
            with pytest.raises(RuntimeError):
                await call_tool("broken_tool", {"param": "value"})
            mock_logger.exception.assert_called_once()
