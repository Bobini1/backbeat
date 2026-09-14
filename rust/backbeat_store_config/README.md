# Backbeat Store Config

Configuration types and helpers for the Backbeat store's `backbeat.toml` file.

This is broken out from the store. You might find it useful. Probably not, though.

[Documentation](https://docs.rs/backbeat_store_config)

## Usage

```sh
cargo add backbeat_store_config
```

Load the user's configuration, or write one to a specific directory:

```rust
use backbeat_store_config::BackbeatConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let config = BackbeatConfig::load()?;
	println!("where the user wants to store stuff: {}", config.store.path.display());
	Ok(())
}
```
