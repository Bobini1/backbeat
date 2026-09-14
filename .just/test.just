check:
	#!/bin/bash
	set -euo pipefail

	source .scripts/multi_evaluator.sh

	evaluate "just fmt-check"
	evaluate "just bun-workspace-run lint"
	evaluate "just typecheck"
	# evaluate "just doctest"
	evaluate "cargo clippy --locked --quiet --all-targets -- -D warnings"
	evaluate "cargo machete"

	post_evaluate

[private]
bun-workspace-run SCRIPT *ARGS:
	#!/bin/bash
	set -euo pipefail
	set +e
	output="$(bun run --filter '*' {{SCRIPT}} {{ARGS}} 2>&1)"
	status=$?
	set -e
	if [ "$status" -ne 0 ] && [[ "$output" == *"No packages matched the filter"* ]]; then
		echo "No workspace packages define {{SCRIPT}} yet."
		exit 0
	fi
	printf '%s\n' "$output"
	exit "$status"

typecheck:
	just bun-workspace-run typecheck
	just app-typecheck

fix:
	#!/bin/bash
	set -euo pipefail

	just bun-workspace-run lint-fix -- --quiet
	cargo clippy --fix --allow-dirty
	just fmt

# Automatically fix any issues we can in the codebase and then
# make a commit for them
autofix:
	#!/bin/bash
	set -euo pipefail

	# Only generate a commit if the repo is clean
	SHOULD_COMMIT=0
	if [ -z "$(git status --porcelain)" ]; then
		SHOULD_COMMIT=1
	fi

	just fix

	if [ $SHOULD_COMMIT -eq 1 ]; then
		git add .
		git commit -m "chore: autofix"
	fi

fmt:
	cargo fmt
	bun biome format --write

fmt-check:
	cargo fmt --check
	bun biome format

doctest:
	cargo test --locked --doc --workspace

test-rust FILTER="*":
	#!/bin/bash
	set -euo pipefail

	if [[ "{{FILTER}}" == *"|"* ]]; then
		echo "\`|\` doesn't work in the regex here for tests, sorry" >&2
		exit 1
	fi

	if [ "{{FILTER}}" = "*" ]; then
		cargo nextest run --locked --cargo-quiet
	else
		cargo nextest run --locked --cargo-quiet '{{FILTER}}'
	fi

test-rust-package PACKAGE FILTER="*":
	#!/bin/bash
	set -euo pipefail

	if [ "{{FILTER}}" = "*" ]; then
		cargo nextest run --locked --cargo-quiet -p "{{PACKAGE}}"
	else
		cargo nextest run --locked --cargo-quiet -p "{{PACKAGE}}" '{{FILTER}}'
	fi

# Run tests with llvm-cov and generate HTML + LCOV reports.
# First run is slow (~minutes): instrumented builds live in target/llvm-cov-target/.
test-coverage FILTER="*":
	#!/bin/bash
	set -euo pipefail

	export CARGO_LLVM_COV_TARGET_DIRECTORY="target/llvm-cov-target"

	COVERAGE_DIR="target/llvm-cov"
	mkdir -p "${COVERAGE_DIR}"

	LLVM_COV_ARGS=(
		--no-clean
		--lcov --output-path "${COVERAGE_DIR}/lcov.info"
		--cargo-quiet
	)

	if [ "{{FILTER}}" = "*" ]; then
		cargo llvm-cov nextest "${LLVM_COV_ARGS[@]}"
	else
		cargo llvm-cov nextest "${LLVM_COV_ARGS[@]}" '{{FILTER}}'
	fi

	cargo llvm-cov report --html --output-dir "${COVERAGE_DIR}/html"

	echo
	echo "Coverage reports:"
	echo "  HTML: ${COVERAGE_DIR}/html/index.html"
	echo "  LCOV: ${COVERAGE_DIR}/lcov.info"

test FILTER="*":
	#!/bin/bash
	set -euo pipefail

	source .scripts/multi_evaluator.sh

	evaluate "just test-typescript '{{FILTER}}'"
	evaluate "just test-rust '{{FILTER}}'"
	evaluate "just test-capi"

	post_evaluate

test-typescript FILTER="*":
	#!/bin/bash
	set -euo pipefail
	set +e
	output="$(bun run --filter '{{FILTER}}' test -- --reporter=default 2>&1)"
	status=$?
	set -e
	if [ "$status" -ne 0 ] && [[ "$output" == *"No packages matched the filter"* ]]; then
		echo "No workspace packages define test yet."
		exit 0
	fi
	printf '%s\n' "$output"
	exit "$status"
