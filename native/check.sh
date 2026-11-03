#!/usr/bin/env bash
# Compiles and tests the native mod against a local Horizon checkout, as the launcher would
# build it: horizon-client from that checkout, with Horizon's own crate patches.
#
#   HORIZON_SRC=/path/to/horizon native/check.sh          # cargo test
#   HORIZON_SRC=/path/to/horizon native/check.sh clippy   # or any cargo command
#
# The checkout should be at the commit in native/create/horizon-mixin.toml.
set -euo pipefail
: "${HORIZON_SRC:?set HORIZON_SRC to a Horizon checkout (git clone https://github.com/horizon-rust/horizon)}"
H="$HORIZON_SRC"
cd "$(dirname "$0")/create/api"
# The bundle may only use crate versions in Horizon's lock file. A bundle may hold only
# .rs and .toml files, so the copy goes again afterwards.
cp "$H/Cargo.lock" .
trap 'rm -f Cargo.lock' EXIT
cmd="${1:-test}"
shift || true
cargo "$cmd" \
  --config "patch.'https://github.com/horizon-rust/horizon'.horizon-client.path='$H/crates/horizon-client'" \
  --config "patch.crates-io.wgpu-hal.path='$H/third_party/wgpu-hal'" \
  --config "patch.crates-io.naga.path='$H/third_party/naga'" \
  --config "patch.crates-io.libmimalloc-sys.path='$H/third_party/libmimalloc-sys'" \
  "$@"
