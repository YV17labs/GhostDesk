# Changelog

All notable changes to GhostDesk are documented here. This project follows [Semantic Versioning](https://semver.org/) and [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) conventions.

## [v7.4.1] — 2026-05-19

Operator-supplied `LANG` (e.g. `fr_CA.UTF-8`, `de_DE.UTF-8`) is honored again at boot. The Ubuntu 26.04 base regression that crashed any non-default locale on `docker run` is neutralized inside the entrypoint.

### Fixed
- **Container restart loop when `-e LANG=` is set to anything other than `en_US.UTF-8` or `C.UTF-8`.** Ubuntu 26.04 (introduced in v7.3.0) ships `rust-coreutils 0.8.0`, whose `icu_collator` panics with `index out of bounds` when `locale-gen` runs while `LC_COLLATE` points at the locale being generated — a chicken-and-egg trigger that fires on every locale the base image does not pre-build. `locale-gen` then exits 0 with nothing actually built, `update-locale` rejects the result, `set -e` kills the entrypoint, and Docker loops on exit 255 with no useful signal beyond the perl/ICU warning soup. Two-part defense in `docker/init/entrypoint.sh`: pin `LC_ALL=C` around both `locale-gen` and `update-locale` so the broken collator path never fires, and verify the locale actually landed in `locale -a` instead of trusting `locale-gen`'s misleading exit code. If the requested locale cannot be built (typo, unsupported tag), the entrypoint logs an explicit `WARN ... falling back to en_US.UTF-8` and continues — the operator gets a one-line diagnosis in `docker logs`, not a silent restart loop. The `locale -a` membership check uses the documented glibc internal mapping (`.UTF-8` → `.utf8`) and applies no other normalization — what the operator writes is what the operator gets.

---

## [v7.4.0] — 2026-05-19

`screen_shot` exposes a WebP `quality` knob, and its effective default drops from 80 to 50 — roughly halving the encoded payload on every capture without altering coordinates or breaking any caller.

### Added
- **`quality` parameter on `screen_shot`.** Accepts `1-100`, validated eagerly with `ValueError`. Forwarded to the WebP encoder via `Pillow.Image.save(..., format="WebP", quality=...)`. Ignored when `format="png"` (PNG is lossless by construction). The docstring spells out when the agent should raise it back to `80+` — small fonts in PDFs, design mockups, photo content — so the model has a deterministic policy instead of paying for the maximum on every capture.
- **`save_image_bytes(..., quality=)` propagation** in `ghostdesk.screen._shared`. The shared encoder helper threads `quality` through to Pillow for any caller that bypasses `screen_shot`.

### Changed
- **Default WebP encoder quality 80 → 50.** Previously Pillow's implicit default (80) was used; the encoder call is now explicit at 50, chosen empirically on Hacker News, GitHub, and IDE captures: the visual difference vs. 80 is imperceptible on solid-background UI content (text, buttons, menus), while encoded bytes shrink by roughly 50% on a typical 1280×1024 desktop. Agents that need full fidelity explicitly pass `quality=80+`. Pixel coordinates are unaffected — the image dimensions stay native, so `mouse_click` / `mouse_drag` receive screen-space coordinates verbatim and the change is safe for any LLM regardless of vision-coordinate calibration.

---

## [v7.3.1] — 2026-05-02

Wallpaper renders correctly on Ubuntu 26.04 production images, and brand assets no longer need a re-edit at every release.

### Fixed
- **Wallpaper not rendering on Ubuntu 26.04 prod images.** `swaybg` in 26.04 routes image loading through `libglycin`, which sandboxes its loaders in `bubblewrap`. Under Docker's default AppArmor profile `bwrap` fails at `pivot_root` with "Operation not permitted" — a string glycin's no-sandbox fallback heuristic does not match, so it kept retrying the sandbox and only the `#1a1b2e` fallback colour from the sway config was visible. Shadowed `/usr/local/bin/bwrap` with a stub that emits the error glycin DOES recognize (`No permissions to create a new namespace`), forcing the no-sandbox loader path. Applied to both `docker/base/Dockerfile` and `.devcontainer/Dockerfile`. Cost is one layer of defense-in-depth on a static asset we control; the agent never feeds untrusted images to glycin.
- **Sway / wayvnc config directories created at image build time.** `~/.config/sway` and `~/.config/wayvnc` are now created (with correct ownership and modes) inside `docker/base/Dockerfile` and `.devcontainer/Dockerfile`. The matching `install -d` calls were removed from `docker/init/entrypoint.sh`, so the directories exist before the entrypoint renders the configs into them and the path is identical between prod and devcontainer images.

### Changed
- **Wallpaper migrated PNG → SVG.** `swaybg` rasterizes the SVG natively at output resolution via `glycin-svg`, eliminating the 8-bit PNG quantization step that produced visible gradient banding on the dark navy background. Asset shrinks from 327 KB raster to 4.5 KB vector source — and the build no longer needs any rasterization step. `docker/services/sway/config`, `docker/base/Dockerfile`, and `.devcontainer/Dockerfile` updated to reference `wallpaper.svg`.
- **Wallpaper redesign — Aurora.** Removed the HUD-style frame: corner brackets, "SESSION / ACTIVE", "VNC · READY", `v7.0`, "YV17 · GHOSTDESK", scan rings, starfield, horizon line, grid and dot overlays. Replaced with a minimalist composition: deep radial-gradient base, three soft floutéd auroras (purple / mint / magenta), subtle vignette, refined typography (Inter weight 300, larger letter-spacing). Result is timeless and version-free — survives all releases without re-edits.
- **Banner and social-card SVGs de-versioned.** Removed `v7.0` and the corner-label HUD (`SESSION / ACTIVE`, `YV17 · GHOSTDESK`, `VNC · READY`) from `assets/banner.svg` and `assets/social-card.svg`. Brand assets are now permanent — no edits needed each release.

### Removed
- **Orphan PNG assets.** Deleted `banner.png`, `logo-mark.png`, `logo.png`, `social-card.png`, and `wallpaper.png` from `assets/`. Every in-repo reference (Dockerfiles, noVNC index, README, `_icons.py`) already pointed at the `.svg` variants — the PNGs were leftover build artefacts taking ~1 MB on disk.

---

## [v7.3.0] — 2026-05-01

Window-awareness for the agent (so it stops launching duplicates of apps that are already open) and a server-side idle watchdog (so long-running sessions don't slowly leak Firefox / foot / mousepad windows). Base image refreshed to Ubuntu 26.04 LTS.

### Added
- **`app_running` MCP tool.** Walks the Sway tree and returns one entry per open client window (`app`, `title`, `pid`, `focused`). Agents are now instructed to call it before `app_launch` so they switch to an existing window instead of starting a second instance — the doctrine in `instructions.py` and the `app_launch` docstring both point at it. Tool count goes from 13 to 14.
- **Idle session watchdog.** When no MCP tool call has been observed for `GHOSTDESK_IDLE_TIMEOUT` seconds (default `1800`, i.e. 30 minutes), the server walks the Sway tree and asks every client window to close gracefully via `swaymsg [con_id=N] kill`. Sway, mako, wayvnc and the MCP server itself are spared — only client surfaces with a backing PID are touched. Set `GHOSTDESK_IDLE_TIMEOUT=0` to disable. The clock is reset on every tool invocation by middleware (`mark_activity()`).
- **`GHOSTDESK_IDLE_TIMEOUT` environment variable.** Operator-side, not client-side: the agent cannot override the deployment's cleanup policy. Documented in the README env-var table.
- **`ghostdesk._sway` IPC helpers.** Resilient socket discovery that probes `swaymsg -s <sock> -t get_version` against every `sway-ipc.*.sock` whose embedded PID resolves to a live `sway` process — `$XDG_RUNTIME_DIR` accumulates dead sockets across container restarts, and `$SWAYSOCK` captured at process start grows stale when Sway restarts. The cached socket is invalidated on any IPC failure and rediscovered once before giving up, so the server self-heals across compositor restarts.

### Changed
- **Ubuntu base image bumped to 26.04 LTS** (was 24.04) in both `docker/base/Dockerfile` and `.devcontainer/Dockerfile`. The `wayvnc` / `neatvnc` / `aml` source pins are unchanged and still build cleanly against the new toolchain.
- **noVNC pinned bump 1.6.0 → 1.7.0** (`NOVNC_VERSION` in `docker/base/Dockerfile` and `.devcontainer/Dockerfile`). Brings the upstream NPM bundle ES-module conversion, improved H.264 detection, more efficient memory usage (received image data dropped after rendering), and a tab-close warning when not in view-only mode. The wayvnc workaround for VeNCrypt remains in place — noVNC 1.7 still does not negotiate the X509Plain/262 sub-type.
- **wayvnc / neatvnc / aml switched from master SHAs to upstream release tags** (`WAYVNC_REF=v0.10.0`, `NEATVNC_REF=v1.0.0`, `AML_REF=v1.0.0`). neatvnc 1.0.0 ships the async-auth API rework that wayvnc 0.10.0 has migrated to, so the local `wayvnc-async-auth.patch` (which bridged wayvnc's old sync `on_auth` to neatvnc's new async signature) is no longer needed and has been deleted along with the now-empty `docker/base/patches/` directory. `git clone --depth 1 --branch ${REF}` replaces the previous full-clone-then-checkout for faster builds. The `sed` that strips `RFB_SECURITY_TYPE_APPLE_DH` from neatvnc remains — with `relax_encryption=true` the `NVNC_AUTH_REQUIRE_ENCRYPTION` flag is unset, so neatvnc would still advertise Apple-DH and noVNC would pick it first and fail.
- **Python deps refresh** via `uv lock --upgrade`: starlette `0.52.1 → 1.0.0`, cryptography `46.0.5 → 47.0.0`, rich `14.3.3 → 15.0.0`, pydantic `2.12.5 → 2.13.3`, uvicorn `0.42.0 → 0.46.0`, pillow `12.1.1 → 12.2.0`, plus minor bumps across anyio / certifi / click / idna / packaging / pydantic-core / pydantic-settings / pygments / pytest / pytest-cov / python-multipart / sse-starlette / typer. All transitive — GhostDesk does not import starlette / cryptography / rich directly, and 218 tests still pass. Direct pin `Pillow>=10.0` → `Pillow>=12.0`; dev pins tightened to match installed versions (`pytest>=9.0`, `pytest-asyncio>=1.3`, `pytest-cov>=7.0`).
- **`app_list` filters out `Terminal=true` `.desktop` entries** (vim, htop, …). These are TUIs that need a tty — launching them headless gives a process that exits instantly with no window, so they don't belong in a GUI catalogue.
- **`app_launch` docstring and server `instructions`** now direct the agent to consult `app_running()` first rather than relying solely on a screenshot, since the screenshot only shows the focused workspace.

---

## [v7.2.0] — 2026-04-21

Hover-only mouse interaction, screen-wide change feedback, and middleware-side timing instrumentation.

### Added
- **`mouse_move` tool.** Repositions the virtual pointer without clicking — needed for hover-only UI reactions (tooltips, dropdown previews, hover-revealed icons). Tool count goes from 12 to 13.

### Changed
- **`poll_for_change` polls the full screen** instead of a 200×200 zone around the cursor. The previous bounded zone missed UI updates that landed elsewhere on screen (modal openings, taskbar changes, status-bar text), producing false-negative miss warnings.
- **Tool-call durations logged in milliseconds** (`name(args) → OK (123 ms)`), making slow tools easier to spot in logs.

### Removed
- **Unused `_cursor` module.** Dead code from an earlier capture pipeline iteration; nothing imported it.

---

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

[Unreleased]: https://github.com/yv17labs/ghostdesk/compare/v7.4.0...HEAD
[v7.4.0]: https://github.com/yv17labs/ghostdesk/compare/v7.3.1...v7.4.0
[v7.3.1]: https://github.com/yv17labs/ghostdesk/compare/v7.3.0...v7.3.1
[v7.3.0]: https://github.com/yv17labs/ghostdesk/compare/v7.2.0...v7.3.0
[v7.2.0]: https://github.com/yv17labs/ghostdesk/compare/v7.1.0...v7.2.0
[v7.1.0]: https://github.com/yv17labs/ghostdesk/compare/v7.0.1...v7.1.0
[v7.0.1]: https://github.com/yv17labs/ghostdesk/compare/v7.0.0...v7.0.1
[v7.0.0]: https://github.com/yv17labs/ghostdesk/compare/v6.0.0...v7.0.0
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
