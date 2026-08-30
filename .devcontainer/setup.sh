#!/usr/bin/env bash

set -euo pipefail

CARGO_HOME="${CARGO_HOME:-${HOME}/.cargo}"
export PATH="${CARGO_HOME}/bin:${PATH}"

NEST_RS_CLI_VERSION=6.1.0

cargo install --locked --version "${NEST_RS_CLI_VERSION}" nest-rs-cli

cargo build -p ghostdesk
