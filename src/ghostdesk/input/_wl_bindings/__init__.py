# Copyright (c) 2026 YV17 — AGPL-3.0 with Commons Clause
"""Generated pywayland bindings for the protocols Ghostdesk consumes.

Regenerate with::

    uv run pywayland-scanner \\
        -i src/ghostdesk/input/protocols/wayland.xml \\
        -i src/ghostdesk/input/protocols/virtual-keyboard-unstable-v1.xml \\
        -i src/ghostdesk/input/protocols/wlr-virtual-pointer-unstable-v1.xml \\
        -o src/ghostdesk/input/_wl_bindings

The package-level ``__init__`` is required so the generated sub-packages
can use relative imports between each other (e.g. the virtual-pointer
bindings reference ``..wayland.WlOutput``).
"""
