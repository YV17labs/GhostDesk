# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk._icons — the single GhostDesk mark reused everywhere."""

import base64

from ghostdesk._icons import GHOSTDESK_ICON, GHOSTDESK_ICONS
from ghostdesk.server import create_app


def test_icon_is_valid_data_uri():
    """Icon src is a valid base64 SVG data URI."""
    assert GHOSTDESK_ICON.mimeType == "image/svg+xml"
    assert GHOSTDESK_ICON.src.startswith("data:image/svg+xml;base64,")
    payload = GHOSTDESK_ICON.src.removeprefix("data:image/svg+xml;base64,")
    decoded = base64.b64decode(payload)
    assert decoded.startswith(b"<svg") and decoded.rstrip().endswith(b"</svg>")


def test_icons_list_wraps_single_icon():
    """GHOSTDESK_ICONS wraps the single canonical icon."""
    assert GHOSTDESK_ICONS == [GHOSTDESK_ICON]


async def test_every_tool_carries_the_ghostdesk_icon():
    """Every registered tool advertises GHOSTDESK_ICON."""
    app = create_app(port=9999)
    for name, tool in app._tool_manager._tools.items():
        assert tool.icons, f"tool {name!r} has no icons"
        assert tool.icons[0].src == GHOSTDESK_ICON.src, f"tool {name!r} uses a non-canonical icon"


async def test_prompt_and_resources_carry_the_icon():
    """The prompt and both resources also advertise GHOSTDESK_ICON."""
    app = create_app(port=9999)
    prompt = app._prompt_manager._prompts["system_prompt"]
    assert prompt.icons and prompt.icons[0].src == GHOSTDESK_ICON.src
    for uri, res in app._resource_manager._resources.items():
        assert res.icons and res.icons[0].src == GHOSTDESK_ICON.src, f"{uri} missing icon"
