# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""Apps app_list tool — enumerate installed GUI applications."""

from ghostdesk.apps._desktop import desktop_apps


async def app_list() -> list[dict]:
    """Return the catalogue of installed GUI applications.

    Read from ``.desktop`` entries under ``/usr/share/applications/``.
    This catalogue is the strict whitelist — ``app_launch`` refuses any
    executable not in it.

    Call this the first time a task names an application, and again
    after installing software during the session.

    Returns a list of dicts, each with:
    - name: human-readable application name.
    - exec: the string to pass to ``app_launch()``.
    """
    return desktop_apps()
