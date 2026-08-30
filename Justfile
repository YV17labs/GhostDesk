# Every framework variable carries this prefix (GHOSTDESK_ENV, GHOSTDESK_HTTP__PORT, …).
# It must be set on the process, so it lives here and in your deployment —
# never in `.env`, which is read too late to have chosen itself.
export NESTRS_ENV_PREFIX := "GHOSTDESK"

_default:
    @just --list

# Run an app with auto-reload — watches the source, rebuilds and restarts on
# save. Default: ghostdesk. Usage: nestrs run dev
#
# `GHOSTDESK_ENV` is set here rather than in `.env`: it selects the cascade, so
# it has to exist before any file is read, and it is what arms the
# development-only affordances — absence has to mean "not development".
dev app="ghostdesk":
    GHOSTDESK_ENV=development bacon run-long -- --bin {{app}}

# Run an app in release mode. Usage: nestrs run start
start app="ghostdesk":
    cargo run --release --bin {{app}}

# Build in release: one app (default ghostdesk), or every app with `--all`.
# Usage: nestrs run build   |   nestrs run build --all
build app="ghostdesk":
    cargo build --release {{ if app == "--all" { "--workspace" } else { "-p " + app } }}

# Install an app's release binary into ~/.cargo/bin. The path is spelled out
# because the workspace root is a virtual manifest: a bare `cargo install` has
# no package to install and refuses rather than picking a member. `--locked`
# builds the dependency set the lockfile pins — the one CI checked — instead of
# re-resolving to whatever crates.io offers today.
# Usage: nestrs run install
install app="ghostdesk":
    cargo install --path apps/{{app}} --locked

# Type-check the workspace.
check:
    cargo check --workspace

# Tests — unit/integration/e2e/doctests. Usage: nestrs run test [unit|e2e|doc]
mod test

# No `mod db`: GhostDesk holds no persistent state, so the workspace carries
# neither a `migrations` nor a `seed` crate for the database verbs to drive.

# Apply rustfmt across the workspace.
fmt:
    cargo fmt --all

# Clippy (strict) + format check.
lint:
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all --check

# --- GhostDesk -------------------------------------------------------------
# The desktop the MCP server drives — Sway, mako, wayvnc, websockify and the
# server itself, all under one supervisord. These mirror the VS Code tasks in
# `.vscode/tasks.json`; both talk to the same daemon, so either entry point
# works from inside the devcontainer.

# Bring the stack up in the foreground — Ctrl-C stops every service.
stack:
    supervisord -c /etc/supervisord.conf -n

# Restart every service (supervisord itself keeps running).
stack-restart:
    supervisorctl -c /etc/supervisord.conf restart all

# List the services and their state.
stack-status:
    supervisorctl -c /etc/supervisord.conf status

# Ask supervisord to shut down — stops every service, then exits.
stack-down:
    supervisorctl -c /etc/supervisord.conf shutdown
