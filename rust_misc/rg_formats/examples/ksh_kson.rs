//! Smoke-test CLI: parse every .ksh / .kson path given on the command line and
//! report whether each one succeeded or failed.
//!
//! Exit code: 0 when every file parses cleanly, 1 if any file had a parse
//! error or could not be read.

use std::path::Path;
use std::process::ExitCode;

fn safe_extension(path: &Path) -> Option<&str> {
	if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
		return Some(ext);
	}
	let name = path.file_name()?.to_str()?;
	let rest = name.strip_prefix('.')?;
	if !rest.is_empty() && !rest.contains('.') {
		Some(rest)
	} else {
		None
	}
}

fn main() -> ExitCode {
	let mut any_err = false;

	for arg in std::env::args().skip(1) {
		let path = Path::new(&arg);

		match safe_extension(path) {
			Some("ksh") => match rg_formats::ksh::from_file(path) {
				Ok(Ok(_)) => println!("ok  {arg}"),
				Ok(Err(e)) => {
					eprintln!("FAIL {arg}  [{e}]");
					any_err = true;
				}
				Err(e) => {
					eprintln!("ERR  {arg}  (io: {e})");
					any_err = true;
				}
			},
			Some("kson") => match rg_formats::kson::from_file(path) {
				Ok(Ok(_)) => println!("ok  {arg}"),
				Ok(Err(e)) => {
					eprintln!("FAIL {arg}  [{e}]");
					any_err = true;
				}
				Err(e) => {
					eprintln!("ERR  {arg}  (io: {e})");
					any_err = true;
				}
			},
			_ => {
				eprintln!("SKIP {arg}  (unrecognised extension)");
			}
		}
	}

	if any_err {
		ExitCode::FAILURE
	} else {
		ExitCode::SUCCESS
	}
}
