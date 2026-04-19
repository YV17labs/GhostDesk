# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Prompts domain — MCP prompts surfaced to clients."""

from mcp.server.fastmcp import FastMCP

from ghostdesk._icons import GHOSTDESK_ICONS
from ghostdesk.prompts.system_prompt import system_prompt


def register(mcp: FastMCP) -> None:
    """Register prompts."""
    mcp.prompt(icons=GHOSTDESK_ICONS)(system_prompt)
