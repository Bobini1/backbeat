cli-install:
	mkdir -p ~/.local/bin && ln -sf "$(pwd)/cli.sh" ~/.local/bin/bkb
	mkdir -p ~/.local/bin && ln -sf "$(pwd)/cli-release.sh" ~/.local/bin/bkbrelease

# Profile the CLI with flamegraph (requires cargo-flamegraph, uses sudo for dtrace).
# Usage: just cli-flamegraph import -r path/to/charts/
cli-flamegraph *args:
	CARGO_HOME=$HOME/.cargo PATH=$PATH \
		cargo flamegraph --profile dev-release -p bkb --bin bkb -- {{args}}
