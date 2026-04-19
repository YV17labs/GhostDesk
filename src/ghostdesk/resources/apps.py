# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Apps resource — read-only view of installed GUI applications."""

import json

from ghostdesk.apps.app_list import app_list


async def apps_resource() -> str:
    """Installed GUI apps as a JSON array of ``{name, exec}`` — same payload as ``app_list``."""
    return json.dumps(await app_list(), ensure_ascii=False)
