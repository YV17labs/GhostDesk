# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Resources domain — read-only counterparts of tools, fetchable via ``resources/read``."""

from mcp.server.fastmcp import FastMCP

from ghostdesk._icons import GHOSTDESK_ICONS
from ghostdesk.resources.apps import apps_resource
from ghostdesk.resources.clipboard import clipboard_resource


def register(mcp: FastMCP) -> None:
    """Register resources."""
    mcp.resource(
        "ghostdesk://apps",
        name="apps",
        description=(
            "Installed GUI applications as a JSON array of {name, exec}. "
            "Same data as the app_list tool, fetchable without spending "
            "an agent turn."
        ),
        mime_type="application/json",
        icons=GHOSTDESK_ICONS,
    )(apps_resource)
    mcp.resource(
        "ghostdesk://clipboard",
        name="clipboard",
        description=(
            "Current system clipboard text. Same data as the "
            "clipboard_get tool, fetchable without spending an agent turn."
        ),
        mime_type="text/plain",
        icons=GHOSTDESK_ICONS,
    )(clipboard_resource)
