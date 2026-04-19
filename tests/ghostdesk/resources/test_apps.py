# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Tests for ghostdesk.resources.apps."""

import json
from unittest.mock import AsyncMock, patch

from ghostdesk.resources.apps import apps_resource
from ghostdesk.server import create_app


async def test_apps_resource_returns_json_array():
    """apps_resource serialises app_list() output as JSON."""
    payload = [{"name": "Firefox", "exec": "firefox"}]
    with patch(
        "ghostdesk.resources.apps.app_list",
        new_callable=AsyncMock,
        return_value=payload,
    ):
        body = await apps_resource()
    assert json.loads(body) == payload


async def test_create_app_registers_apps_resource():
    """create_app() exposes the ghostdesk://apps URI."""
    app = create_app(port=9999)
    assert "ghostdesk://apps" in app._resource_manager._resources
