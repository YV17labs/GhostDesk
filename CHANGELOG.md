# Changelog

All notable changes to GhostDesk are documented here. This project follows [Semantic Versioning](https://semver.org/) and [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) conventions.

## [Unreleased] — v7.0.0

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
- **Environment variables namespaced under `GHOSTDESK_*`.** All Python runtime vars now live under a single namespace (`GHOSTDESK_PORT`, `GHOSTDESK_SCREEN_WIDTH`, `GHOSTDESK_MODEL_SPACE`, …). Standard POSIX vars (`TZ`, `LANG`) are kept as-is.
- **Input stack.** Replaced `dotool` with direct Wayland virtual-pointer / virtual-keyboard protocols.
- **Coordinate normalisation.** Middleware now normalises coordinates between model space and pixels (`GHOSTDESK_MODEL_SPACE=0` for frontier models, `1000` for Qwen-family models).
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
