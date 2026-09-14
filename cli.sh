#!/bin/bash

# Wrapper for the bkb CLI - builds before running so it's never stale.
# Symlink this to `bkb` via `just cli-install`.

SCRIPT_DIR=$(dirname "$(readlink -f "$0")")

stderr_tmp=$(mktemp)

start_time=$(date +%s)

(cd "$SCRIPT_DIR/rust_extra/backbeat_cli" && cargo build 2>"$stderr_tmp")

build_status=$?
end_time=$(date +%s)
elapsed=$((end_time - start_time))

if [ $build_status != 0 ]; then
	cat "$stderr_tmp"
	echo
	echo "cli has compilation errors, go fix them"
	exit 1
fi

if [ $elapsed -gt 1 ]; then
	echo "# compilation done"
fi

"$SCRIPT_DIR"/target/debug/bkb "$@"
