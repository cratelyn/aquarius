#!/usr/bin/env bash
# =========================================================================== #
# demo: run a load-test against a local http/2 server.                        #
# =========================================================================== #

set -eou pipefail

# build aquarius and the test server.
cargo build --release --package aquarius
cargo build --release --package aquarius-test-server

# start the test server, running it in the background.
./target/release/aquarius-test-server &
server="$!"
trap 'kill -9 "$server"' EXIT

# wait a few seconds, allowing time for the server to start.
sleep 5;

# run a load-test against the server.
#
# NB: emit tracing logs, and render some ascii charts when we are finished.
cargo run --release --bin aquarius -- \
    --trace --show-charts \
    --total 512 --rate 128 localhost:8080
