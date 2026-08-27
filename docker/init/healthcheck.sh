#!/bin/sh
# Docker HEALTHCHECK — three questions, one exit code.
#
#   1. Are the sibling services up?      supervisorctl
#   2. Is the process worth keeping?     GET /health/live
#   3. Can an agent get work done?       GET /health/ready
#
# The first question is supervisord's alone: nothing inside the server can see
# whether wayvnc and websockify are serving the VNC path, and a desktop no
# human can watch is a broken container even while every tool answers.
#
# The last two are the server's alone, and asking them is the point of this
# rewrite. `supervisorctl status` reports RUNNING for a *wrapper* — a program
# waiting on a socket counts as up — so on its own it certified a container
# whose desktop was not there yet. The probes are the only thing that knows
# the difference: the compositor connection behind the virtual pointer, and
# the window seam three tools read.
#
# Both are GETs on the plain HTTP surface with no bearer: the framework mounts
# them #[public] because a probe an orchestrator cannot read reports nothing.
# The body carries indicator names and up/down, never a reason — those are in
# the server's log, filtered on nest_rs::health.
set -eu

output=$(supervisorctl -c /etc/supervisord.conf status 2>&1) || {
    echo "$output" >&2
    exit 1
}

echo "$output"
echo "$output" | awk '$2 != "RUNNING" { exit 1 }'

# GHOSTDESK_* by hand, like entrypoint.sh and for the same reason: the image
# bakes the prefix these names belong to and the entrypoint refuses to boot
# under any other, so there is no cascade left for a lookup to consult.
PORT="${GHOSTDESK_HTTP__PORT:-3000}"
TLS_CRT="${GHOSTDESK_TLS_CERT:-/etc/ghostdesk/tls/server.crt}"
TLS_KEY="${GHOSTDESK_TLS_KEY:-/etc/ghostdesk/tls/server.key}"

# The transport follows the mounted cert, exactly as websockify's wrapper does.
if [ -s "${TLS_CRT}" ] && [ -s "${TLS_KEY}" ]; then
    SCHEME="https"
else
    SCHEME="http"
fi

# python3, not curl: the runtime image ships neither curl nor wget on purpose
# (docker/base/Dockerfile), and python3 is already there for websockify.
# Certificate verification is off because the peer is this container's own
# loopback and the cert is routinely self-signed — there is no identity to
# check that the hostname 127.0.0.1 has not already settled.
exec python3 - "${SCHEME}://127.0.0.1:${PORT}" <<'PY'
import ssl
import sys
import urllib.error
import urllib.request

base = sys.argv[1]
context = ssl._create_unverified_context()
failed = False

for probe in ("live", "ready"):
    url = f"{base}/health/{probe}"
    try:
        with urllib.request.urlopen(url, timeout=5, context=context) as response:
            print(f"{probe}: {response.status} {response.read().decode(errors='replace')}")
    except urllib.error.HTTPError as error:
        # A 503 is the probe working: the body names which indicator is down.
        print(
            f"{probe}: {error.code} {error.read().decode(errors='replace')}",
            file=sys.stderr,
        )
        failed = True
    except OSError as error:
        print(f"{probe}: unreachable at {url} ({error})", file=sys.stderr)
        failed = True

sys.exit(1 if failed else 0)
PY
