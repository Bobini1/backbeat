# `backbeat_server_client`

This crate provides a HTTP client with a nice API for speaking to a Backbeat server.

[Docs](https://docs.rs/backbeat_server_client)

## Usage

```sh
cargo add backbeat_server_client url
cargo add tokio --features macros,rt-multi-thread
```

```rust,no_run
use backbeat_server_client::BackbeatServerClient;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	let url = Url::parse("https://data.example.com")?;
	let client = BackbeatServerClient::new(&url);

	client.check_up().await?;
	let info = client.get_info().await?;

	println!("{} ({})", info.name, client.base_url());
	Ok(())
}
```
