# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Apps app_list tool — enumerate installed GUI applications."""

from ghostdesk.apps._desktop import desktop_apps


async def app_list() -> list[dict]:
    """Return installed GUI apps.

    Scans ``.desktop`` entries in ``/usr/share/applications/``. Call this
    before choosing which app to use for a task, or after installing new
    software during the session.

    Returns a list of dicts, each with:
    - name: human-readable application name.
    - exec: the executable to pass to ``app_launch()``.
    """
    return desktop_apps()
