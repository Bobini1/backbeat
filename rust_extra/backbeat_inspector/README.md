# Backbeat Inspector

This is a crate that takes in `.bb` files and "inspects" their chart contents to extract metadata.

Most notably, it's used by the store to implement `describe`.

## Disclaimer

I'm still not happy with this crate being part of `backbeat_sdk`. It introduces a needless dependency of backbeat needing to know how to parse chart formats, if only to describe them.

I'm sure if I had more time, I could figure out a way to detach this - maybe by making bundle descriptions inherit straight from collections, and otherwise being `unknown chart`.

It results in terrible UX, though. This is as good as it's going to get.

## Usage

```sh
cargo add backbeat_inspector backbeat_core
```

Load a `.bb` file and inspect its chart metadata:

```rust,no_run
use backbeat_core::BackbeatFile;
use backbeat_inspector::inspect_bundle;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let bundle = BackbeatFile::from_file("path/to/chart.bb")?;
	let inspection = inspect_bundle(&bundle)?;

	println!("{}", inspection.description);
	println!("gamemode: {}", inspection.gamemode);

	Ok(())
}
```
