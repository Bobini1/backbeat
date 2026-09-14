gen:
	just gen-cbindings

gen-cbindings:
	#!/bin/bash
	set -euo pipefail

	header="$PWD/target/capi/include/backbeat.h"
	BKB_HEADER_PATH="$header" cargo build -p backbeat_c_sdk
	printf 'Generated %s\n' "$header"
