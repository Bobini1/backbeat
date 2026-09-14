//! Basic BMS parser on the CLI.

use std::process::ExitCode;

use rg_formats::bms;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
	tracing_subscriber::fmt::fmt()
		.with_writer(std::io::stderr)
		.with_env_filter(EnvFilter::from_default_env())
		.pretty()
		.init();

	let mut err_code = ExitCode::SUCCESS;

	// skip the first argument (as it's the path of this binary)
	// for all arguments given, try and load them as a bms file.
	for arg in std::env::args().skip(1) {
		eprintln!("loading {arg}...");

		match bms::from_file(&arg, bms::BmsRandomStrategy::AlwaysFirstBranch) {
			Ok(data) => match data {
				Ok(data) => {
					println!("{}", serde_json::to_string_pretty(&data).unwrap());
					eprintln!("parsed {arg} successfully!");
				}
				Err(err) => {
					eprintln!("failed to parse {arg} {err}");
				}
			},
			Err(err) => {
				eprintln!("Invalid BMS file: {err}");
				err_code = ExitCode::FAILURE;
			}
		};
	}

	err_code
}
