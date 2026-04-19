# Changelog

All notable changes to GhostDesk are documented here. This project follows [Semantic Versioning](https://semver.org/) and [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) conventions.

## [v7.1.0] — 2026-04-19

Native MCP surfaces the server wasn't exposing yet (resources, lifespan warm-up, icons, tool annotations), stricter HTTP-transport security, finer-grained tool feedback through MCP `notifications/message`, and a consolidated system-level brief delivered through the spec-canonical `instructions` field.

### Added
- **MCP resources.** `ghostdesk://apps` (JSON catalogue of installed GUI apps) and `ghostdesk://clipboard` (current clipboard text) mirror the `app_list` / `clipboard_get` tools so clients that surface resources in a dedicated picker can reach read-only state without spending an agent turn on a tool call.
- **FastMCP lifespan.** The server pre-binds `zwlr_virtual_pointer_v1` and `zwp_virtual_keyboard_v1` during ASGI startup. Missing compositor protocols now fail at boot instead of surfacing mid-request on the first `mouse_click`.
- **MCP context notifications on tools.** `mouse_*` and `key_*` push a `warning` when the 200×200 zone around the action does not change within 2 s — the miss is visible in the client's transcript, not only in the tool result dict. `app_launch` and `clipboard_set` mirror their outcomes through `ctx.info` / `ctx.error`.
- **GhostDesk icon on every MCP surface.** The branded mark is advertised on the server itself, every tool, and both resources through MCP's `icons` field. Inlined as a base64 SVG data URI — no packaging asset to ship alongside the wheel.
- **`ToolAnnotations` on every tool.** `readOnlyHint`, `destructiveHint`, and `idempotentHint` let MCP clients differentiate approval flows for read-only vs destructive actions: `screen_shot` / `clipboard_get` / `app_list` are tagged read-only + idempotent, `mouse_click` / `mouse_drag` / `key_press` are tagged destructive, etc.
- **Origin header validation** (MCP Streamable HTTP spec § DNS-rebinding). Browser requests must match `GHOSTDESK_ALLOWED_ORIGINS` (comma-separated) or get a `403`. Non-browser clients (no `Origin` header) pass through unchanged.
- **Loopback bind by default.** `GHOSTDESK_HOST` defaults to `127.0.0.1`; the container entrypoint exports `0.0.0.0` so Docker port-publishing still reaches the server, but standalone `uv run ghostdesk` no longer silently exposes the port to the LAN.

### Changed
- **Consolidated system-level brief.** The full agent doctrine (SEE → ACT → SEE, prefer-keyboard, interruption handling, scroll-to-end, final self-check) is now carried by the server `instructions` field — the MCP spec-canonical payload delivered in the `initialize` response and auto-injected by every compliant client. Per the MCP spec, `prompts` are user-controlled templates (slash commands, picker entries), which makes them the wrong mechanism for a system-level brief that must always reach the model. One document, guaranteed delivery.
- **Package layout for MCP surfaces.** `resources` is now a package (matching `apps`, `clipboard`, `input`, `screen`) — every domain with a `register(mcp)` function follows the same `__init__.py` convention.
- **`warn_on_miss` helper.** Lives in `input/feedback.py` alongside `build_feedback` and `poll_for_change`, so mouse and keyboard tools share the miss-warning path without crossing underscore-prefixed module boundaries.
- **`mcp[cli]` pinned to `>=1.27`.** Unlocks the `ToolAnnotations`, `Icon`, and lifespan APIs used throughout this release.

### Fixed
- **Wheel scroll direction inverted.** `mouse_scroll(direction="up")` (and `"left"`) silently scrolled the other way: the virtual-pointer `axis_discrete` request was sent with `discrete=+1` regardless of `value`'s sign, violating the `wl_pointer` protocol invariant that the two must match within a frame. Firefox — like any wheel-aware client — trusts `delta_discrete`, so every "up" scroll collapsed into "down" and pinned at the page bottom. Sign is now carried in `_SCROLL_VECTORS` alongside `value`, and a static test locks the invariant.

### Removed
- **Standalone `SYSTEM_PROMPT.md`.** Its content is now folded into the server `instructions` field, delivered automatically at session init. Users who referenced the markdown file directly no longer need to — the guidance now reaches the model through the MCP handshake.

---

## [v7.0.1] — 2026-04-15

### Fixed
- **Missing `envsubst` in runtime images.** `entrypoint.sh` uses `envsubst` to inject `GHOSTDESK_SCREEN_WIDTH` / `GHOSTDESK_SCREEN_HEIGHT` into the Sway config, but the binary was not part of the runtime stack — containers booted into a crash loop (`envsubst: command not found`). Added `gettext-base` to both `docker/base/Dockerfile` and `.devcontainer/Dockerfile`.

---

## [v7.0.0] — 2026-04-15

Major platform overhaul: migration from X11 / Openbox to a native Wayland / Sway stack, end-to-end TLS, split Docker images, and a simplified agent-first documentation story.

### Added
- **Native Wayland stack.** Migrated the devcontainer and runtime to a Wayland / Sway session managed by supervisord. `wl-copy` / `wl-paste` replace the X11 clipboard path, and `grim` replaces the X11 capture tool.
- **wayvnc from pinned source.** `wayvnc` / `neatvnc` / `aml` are now built from a pinned `master` commit inside a dedicated `vnc-builder` Docker stage so classic VNC Auth (RFB security type 2) can be advertised — required for noVNC 1.6 interop. See `docker/base/Dockerfile`.
- **End-to-end TLS.** `websockify` and the MCP server auto-detect a mounted certificate at `/etc/ghostdesk/tls/server.{crt,key}` (or via `GHOSTDESK_TLS_CERT` / `GHOSTDESK_TLS_KEY`) and switch to `wss://` / `https://` at boot.
- **Unified supervisord stack.** Both the runtime and devcontainer images use supervisord as the process manager, with TLS enforced end-to-end.
- **arm64 base image.** The base image now builds cleanly on arm64 from a clean checkout.
- **Dialog handling** in the agent click loop (`SYSTEM_PROMPT.md`).
- **mkcert quickstart** for local HTTPS in the README.

### Changed
- **License:** AGPL-3.0 with Commons Clause → **[Functional Source License 1.1, ALv2 Future License](https://fsl.software/) (FSL-1.1-ALv2)**. The new license is cleaner and less ambiguous than the previous pairing: permitted purposes explicitly cover internal commercial use, non-commercial research/education, and professional services, while prohibiting Competing Use (reselling GhostDesk or offering it as a managed service). Each released version transitions automatically to Apache 2.0 on its second anniversary. File headers, PyPI classifier, OCI image labels, README, and CONTRIBUTING updated accordingly. Licensor: Yoann Vanitou.
- **Environment variables namespaced under `GHOSTDESK_*`.** All Python runtime vars now live under a single namespace (`GHOSTDESK_PORT`, `GHOSTDESK_SCREEN_WIDTH`, …). Standard POSIX vars (`TZ`, `LANG`) are kept as-is.
- **Input stack.** Replaced `dotool` with direct Wayland virtual-pointer / virtual-keyboard protocols.
- **Coordinate normalisation.** Middleware rescales LLM coordinates to screen pixels per request, driven by the `GhostDesk-Model-Space` HTTP header (e.g. `1000` for Qwen-family models). No header → pass-through (frontier models like Claude, GPT-4o, Gemini).
- **Tool naming.** Adopted a `verb_noun` convention throughout the tool surface.
- **Docker layout.** Restructured into per-service subdirectories (`docker/base`, `docker/init`, `docker/services/...`).
- **README** restructured around an agents-first pitch with an mkcert quickstart.
- **Qwen vision guidance** clarified; all references to grid mode removed.
- **VNC hardening.** `GHOSTDESK_VNC_ADDRESS` is now hard-pinned to `127.0.0.1`; override attempts are logged and ignored.

### Removed
- **Humanizer module.** Dropped in favour of direct input event injection.
- **Window metadata / output normalisation** from the screen capture pipeline.
- **Grid mode** references and the previous grid-precision workflow.

### Fixed
- `_desktop._parse_exec` now strips a leading env wrapper when resolving `.desktop` entries.

---

## [v6.0.0] — 2026-04-10

GPA-GUI-Detector integration and small-model workflow tooling.

### Added
- **GPA-GUI-Detector integration** for automatic UI element detection; the model is pre-downloaded in the devcontainer.
- **`grid=True` ruler overlay** on `screenshot()` with minor ticks every 25px and persistent major labels.
- **Small-model prompt** (`SYSTEM_PROMPT.md`) with explicit click-coordinate recipe built around the grid ruler.
- **llama.cpp fork recommendation** over LM Studio for local inference.

### Changed
- Screenshots default to **WebP** encoding.
- Adaptive detection padding; clearer module boundaries in `ghostdesk.screen`.
- Compare raw PNG bytes in feedback poll instead of MD5 hashing.
- `press_key` is now case-tolerant for multi-char keysyms.

### Removed
- GPA-GUI-Detector replaced the earlier detector path after ergonomic review.
- Force-include config in the wheel build.

---

## [v5.0.0] — 2026-04-08

Visual feedback for mouse / keyboard actions.

### Added
- **Visual feedback** on mouse and keyboard actions; LLM instructions updated to rely on it.
- `cursor`, `feedback`, and `process_status` modules extracted.
- `SYS_ADMIN` capability + gnome-keyring unlock; locale persistence.
- Comprehensive tests for `_logging`, `_shared`, middleware coercion, `capture._reencode`, and `server.main`.
- Minor ticks every 25px on the ruler overlay.
- OCI labels + annotations in the CI workflow.

### Changed
- Extracted `save_image_bytes` utility; consolidated image encoding.
- Anonymised system prompt for generic desktop control.

### Removed
- `wait` tool.
- Obsolete `inspect()` documentation and `screenshot.webp` demo image.

### Fixed
- Stale `.venv` cleaned on container create; canonical locale.
- Test assertions moved inside patch context; added `filterwarnings`.
- Source URL casing + vendor label in OCI metadata.

---

## [v4.1.0] — 2026-04-07

Split Docker images (`base` + `latest`).

### Added
- **`base` Docker image** for building custom agents on top of the virtual desktop + VNC + MCP server, without any preinstalled GUI application.
- CI workflow split for `base` and `latest` images.
- `gnome-keyring` daemon added to supervisor.
- README "Custom image" section.

### Changed
- Moved Docker scripts to a shared directory.
- README uses the SVG logo instead of PNG.

### Removed
- Unused files; tightened `.dockerignore`.

---

## [v4.0.1] — 2026-04-06

### Fixed
- Healthcheck uses `supervisorctl` instead of `curl` on the MCP endpoint.

### Added
- Restart policy in Docker examples.
- Required environment variables documented in Docker examples.

---

## [v4.0.0] — 2026-04-06

SOM grounding + `inspect()` + overlay API overhaul.

### Added
- **SOM (Set-of-Mark) grounding** integration, desktop environment overhaul, small-model prompt.
- **`inspect()` tool** re-enabled with improved annotation label readability.
- `onnxruntime` dependency.
- Persistent volumes, `shm_size`, and consistent naming in Docker examples.
- `region` field in screenshot JSON output + docstring.
- Enterprise workforce section in README.

### Changed
- Renamed `annotate` → `overlay`; unified `screenshot` and `inspect` output.
- Restructured tools into dedicated modules.
- Regenerated wallpapers from SVG for 1280×800 and 1280×1024.

### Removed
- Standalone prompt files.
- Obsolete `exec` tool references from README.

### Fixed
- `screenshot()` now includes the captured region in metadata for spatial awareness.

---

## [v3.0.0] — 2026-04-01

License change to **AGPL-3.0-only**; AT-SPI layer removed.

### Added
- `PROMPT.md` — system prompt for desktop assistant agents.
- Branding: logo and wallpaper assets, README header.
- Window listing via `xdotool` in screenshot metadata.

### Changed
- Extracted Openbox startup into a dedicated script.
- Unified Docker stack: `setup-desktop.sh` extracted, `supervisor.conf` parameterised.
- Wallpaper now set via Openbox autostart instead of supervisor.
- **License:** AGPL-3.0-only.

### Removed
- **AT-SPI accessibility layer** and its system dependencies.
- Shell `exec` tool.

---

## [v2.1.0] — 2026-03-31

### Changed
- Trimmed screenshot metadata: dropped `active_window`, filtered phantom windows.
- Split logging / middleware; coerce malformed xy args.
- File headers updated from MIT to AGPL-3.0 with Commons Clause.

---

## [v2.0.0] — 2026-03-28

Simplified tool API (22 → 13 tools); governance & license change.

### Added
- Governance docs.
- Screenshot output format support; optimised LLM instructions.
- Improved error messages for invalid tool arguments.

### Changed
- **License:** MIT → **AGPL-3.0 with Commons Clause**.
- `read_screen` refactored with flat output, browser/content split, and filtering.
- CI: Docker workflow triggers only on version tags.

### Removed
- **Accessibility tools** removed from the public API; focus shifted to screenshot / desktop control.
- Redundant tools removed to reduce API surface (22 → 13).

---

## [v1.1.0] — 2026-03-25

### Added
- Full AT-SPI role coverage (130/130 roles).

### Changed
- Docker image push restricted to git tags only.

---

## [v1.0.1] — 2026-03-25

### Fixed
- `set_clipboard` timeout caused by the `xclip` background process; replaced with stdin-based implementation.

### Added
- Flight-search demo (CDG → JFK).

---

## [v1.0.0] — 2026-03-25

Initial public release.

### Added
- MCP server with desktop control tools (screenshot, mouse, keyboard, clipboard).
- AT-SPI accessibility tools for element discovery and interaction.
- Unified `read_screen` semantic tree tool (replaces `cmd_elements` / `cmd_text`).
- Unicode typing support; tool call logging; optimised screenshot pipeline.
- Click fallback + scroll limit rules in MCP instructions.
- Unit test suite: 183 tests, ~97% coverage.
- Docker image + GitHub Actions CI/CD workflow.
- Google Sheets automation demo; Wikipedia agent demo GIF.
- VS Code devcontainer with MCP server auto-start.

[Unreleased]: https://github.com/yv17labs/ghostdesk/compare/v6.0.0...HEAD
[v6.0.0]: https://github.com/yv17labs/ghostdesk/compare/v5.0.0...v6.0.0
[v5.0.0]: https://github.com/yv17labs/ghostdesk/compare/v4.1.0...v5.0.0
[v4.1.0]: https://github.com/yv17labs/ghostdesk/compare/v4.0.1...v4.1.0
[v4.0.1]: https://github.com/yv17labs/ghostdesk/compare/v4.0.0...v4.0.1
[v4.0.0]: https://github.com/yv17labs/ghostdesk/compare/v3.0.0...v4.0.0
[v3.0.0]: https://github.com/yv17labs/ghostdesk/compare/v2.1.0...v3.0.0
[v2.1.0]: https://github.com/yv17labs/ghostdesk/compare/v2.0.0...v2.1.0
[v2.0.0]: https://github.com/yv17labs/ghostdesk/compare/v1.1.0...v2.0.0
[v1.1.0]: https://github.com/yv17labs/ghostdesk/compare/v1.0.1...v1.1.0
[v1.0.1]: https://github.com/yv17labs/ghostdesk/compare/v1.0.0...v1.0.1
[v1.0.0]: https://github.com/yv17labs/ghostdesk/releases/tag/v1.0.0
