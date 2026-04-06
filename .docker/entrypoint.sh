#!/bin/bash
# Container entrypoint: one-shot init, then hand off to supervisord.
set -euo pipefail

/usr/local/bin/init-container.sh

exec /usr/bin/supervisord -c /etc/supervisor/conf.d/ghostdesk.conf
