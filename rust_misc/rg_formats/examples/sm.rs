//! Basic SM parser on the CLI.

use std::process::ExitCode;

use rg_formats::sm;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
	tracing_subscriber::fmt::fmt()
		.with_writer(std::io::stderr)
		.with_env_filter(EnvFilter::from_default_env())
		.pretty()
		.init();

	let mut err_code = ExitCode::SUCCESS;

	// skip the first argument (as it's the path of this binary)
	// for all arguments given, try and load them as an sm path.
	for arg in std::env::args().skip(1) {
		eprintln!("loading {arg}...");

		match sm::from_path(&arg) {
			Ok(v) => match v {
				Ok(data) => {
					println!("{data:?}");
					eprintln!("parsed {arg} successfully!");
				}
				Err(err) => {
					eprintln!("Invalid SM file: {err}")
				}
			},
			Err(ioerr) => {
				eprintln!("Failed to find file {arg}: {ioerr}");

				err_code = ExitCode::FAILURE;
			}
		};
	}

	err_code
}
