# Run the Backbeat VF app interactively in dev mode.
app-vf:
	#!/usr/bin/env bash
	set -euo pipefail

	if ! command -v bun >/dev/null 2>&1; then
		echo "bun is required: https://bun.sh" >&2
		exit 1
	fi

	cd rust/backbeat_vf_app

	if [ ! -d node_modules ]; then
		bun install
	fi

	exec bun run tauri dev
