#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![allow(unreachable_pub)]
mod cmd;
mod fmt;
mod store_util;
mod style;

use std::process::ExitCode;

use clap::Parser;

use self::cmd::Command;

#[derive(Debug, Parser)]
#[command(name = "bkb", about = "Backbeat CLI", version)]
#[command(propagate_version = true)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[tokio::main]
async fn main() -> ExitCode {
	tracing_subscriber::fmt()
		.with_env_filter(
			tracing_subscriber::EnvFilter::try_from_default_env()
				.unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
		)
		.init();

	let cli = Cli::parse();

	let result = cli.command.run().await;

	match result {
		Ok(()) => ExitCode::SUCCESS,
		Err(err) => {
			eprintln!("{} {err:#}", style::error_label());
			ExitCode::FAILURE
		}
	}
}
