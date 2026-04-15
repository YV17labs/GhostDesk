#!/bin/sh
# Docker HEALTHCHECK — exits 0 only if every supervisord program is RUNNING.
set -eu

output=$(supervisorctl -c /etc/supervisord.conf status 2>&1) || {
    echo "$output" >&2
    exit 1
}

echo "$output"
echo "$output" | awk '$2 != "RUNNING" { exit 1 }'
