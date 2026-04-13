#!/command/with-contenv sh
# Ensure ${APP_DIR}/.venv exists and matches uv.lock before any service
# that depends on it (the MCP server) is started. Runs as ${RUN_USER} so
# the venv is owned by the same uid that will later exec ghostdesk.
#
# --frozen is deliberate: if uv.lock is out of date, we want container
# boot to fail loudly here rather than silently resolve and install
# unexpected versions. To refresh the lock, run `uv lock` interactively,
# commit the result, then rebuild.
#
# In dev, .venv lives in the bind-mounted ${APP_DIR} so it survives
# image rebuilds and tracks the source tree as an editable install. In
# prod, the image should pre-populate ${APP_DIR}/.venv at build time via
# `uv sync --frozen --no-dev`; this script then becomes a fast idempotent
# no-op (uv sync exits early when the env already matches the lock).
set -eu

: "${APP_DIR:?APP_DIR must be set (see 00-service-env.sh)}"
: "${RUN_USER:?RUN_USER must be set (see 00-service-env.sh)}"

if [ ! -f "${APP_DIR}/pyproject.toml" ]; then
    echo "uv-sync: ${APP_DIR}/pyproject.toml missing — skipping" >&2
    exit 0
fi

echo "uv-sync: syncing ${APP_DIR}/.venv as ${RUN_USER}..."
cd "${APP_DIR}"
exec s6-setuidgid "${RUN_USER}" uv sync --frozen
