//! Smoke-test CLI: parse every .dtx path given on the command line and report
//! whether each succeeded, printing a short summary of the parsed content.
//!
//! Exit code: 0 when every file passes all checks, 1 if any file has an IO
//! error or fails a correctness assertion.
//!
//! "Correctness" checks (all real DTX charts should satisfy these):
//!   - `#TITLE` is present and non-empty
//!   - `#BPM` is set and in the range 1.0–1000.0
//!   - `#WAVzz` table has at least one entry
//!   - At least one note event is present
//!   - All event ticks are in 0..384
//!   - At least one of `#DLEVEL` / `#GLEVEL` / `#BLEVEL` is set

use std::path::Path;
use std::process::ExitCode;

fn check(path: &Path) -> Result<(), String> {
	let chart = rg_formats::dtx::from_file(path).map_err(|e| format!("io: {e}"))?;
	let meta = &chart.metadata;

	// Title
	let title = meta.title.as_deref().unwrap_or("").trim().to_owned();
	if title.is_empty() {
		return Err("title missing".to_owned());
	}

	// BPM
	let bpm = meta.bpm.unwrap_or(0.0);
	if !(1.0..=1000.0).contains(&bpm) {
		return Err(format!("bpm out of range: {bpm}"));
	}

	// WAV table
	if meta.wav.is_empty() {
		return Err("no WAV entries".to_owned());
	}

	// Events
	if chart.events.is_empty() {
		return Err("no note events".to_owned());
	}

	// Tick range
	if let Some(bad) = chart
		.events
		.iter()
		.find(|e| e.tick >= rg_formats::dtx::TICKS_PER_MEASURE)
	{
		return Err(format!(
			"event tick {} out of range (measure {})",
			bad.tick, bad.measure
		));
	}

	// At least one difficulty level set
	if meta.dlevel.is_none() && meta.glevel.is_none() && meta.blevel.is_none() {
		return Err("no difficulty level set (DLEVEL/GLEVEL/BLEVEL)".to_owned());
	}

	let level = meta
		.dlevel
		.or(meta.glevel)
		.or(meta.blevel)
		.map(|v| format!("lv{v}"))
		.unwrap_or_default();

	println!(
		"ok  {}  [{title}] bpm={bpm} wavs={} events={} {level}",
		path.display(),
		meta.wav.len(),
		chart.events.len(),
	);

	Ok(())
}

fn main() -> ExitCode {
	let mut any_err = false;

	for arg in std::env::args().skip(1) {
		let path = Path::new(&arg);
		match check(path) {
			Ok(()) => {}
			Err(e) => {
				eprintln!("FAIL {}  [{e}]", path.display());
				any_err = true;
			}
		}
	}

	if any_err {
		ExitCode::FAILURE
	} else {
		ExitCode::SUCCESS
	}
}
