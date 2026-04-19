# Copyright (c) 2026 Yoann Vanitou — FSL-1.1-ALv2
"""GhostDesk branding exposed as MCP :class:`Icon` objects.

Mirrors ``assets/logo-mark.svg``; inlined so the wheel has no runtime
filesystem dependency. Update both in the same commit.
"""

import base64

from mcp.types import Icon

_LOGO_SVG = (
    b'<svg width="400" height="400" viewBox="240 210 200 240" '
    b'xmlns="http://www.w3.org/2000/svg">'
    b'<path d="M260 430 L260 310 C260 265, 280 230, 340 230 '
    b'C400 230, 420 265, 420 310 L420 430 L400 400 L380 430 '
    b'L360 400 L340 430 L320 400 L300 430 L280 400 Z" '
    b'fill="#7B6FDE" opacity="0.85"/>'
    b'<rect x="285" y="275" width="110" height="80" rx="6" fill="#0D0E1A"/>'
    b'<rect x="295" y="290" width="45" height="4" rx="2" fill="#5DCAA5" opacity="0.9"/>'
    b'<rect x="295" y="302" width="70" height="4" rx="2" fill="#5DCAA5" opacity="0.6"/>'
    b'<rect x="295" y="314" width="35" height="4" rx="2" fill="#5DCAA5" opacity="0.4"/>'
    b'<rect x="335" y="314" width="8" height="4" rx="1" fill="#5DCAA5" opacity="0.9"/>'
    b'<rect x="295" y="326" width="55" height="4" rx="2" fill="#5DCAA5" opacity="0.3"/>'
    b'<circle cx="310" cy="260" r="10" fill="#E8E6FF"/>'
    b'<circle cx="370" cy="260" r="10" fill="#E8E6FF"/>'
    b'<circle cx="313" cy="261" r="5" fill="#1A1B2E"/>'
    b'<circle cx="373" cy="261" r="5" fill="#1A1B2E"/>'
    b'<path d="M385 345 L385 365 L392 358 L400 370 L404 368 L396 356 L405 353 Z" '
    b'fill="#FFFFFF" opacity="0.9"/>'
    b"</svg>"
)

_LOGO_DATA_URI = "data:image/svg+xml;base64," + base64.b64encode(_LOGO_SVG).decode("ascii")

GHOSTDESK_ICON = Icon(src=_LOGO_DATA_URI, mimeType="image/svg+xml")
GHOSTDESK_ICONS: list[Icon] = [GHOSTDESK_ICON]
