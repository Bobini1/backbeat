website:
	#!/usr/bin/env bash
	set -euo pipefail

	if ! command -v bun >/dev/null 2>&1; then
		echo "bun is required: https://bun.sh" >&2
		exit 1
	fi

	cd typescript/backbeat-website

	if [ ! -d node_modules ]; then
		bun install
	fi

	exec bun run dev
