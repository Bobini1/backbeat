//! Semantic color and progress helpers for terminal output.
//!
//! Color functions check `NO_COLOR` and whether the relevant stream is a TTY
//! before applying ANSI escape codes.

use std::io::IsTerminal;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

fn color_stdout() -> bool {
	std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

fn color_stderr() -> bool {
	std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
}

/// Green checkmark for successful operations (stdout).
pub fn check_mark() -> &'static str {
	if color_stdout() {
		"\x1b[92+\x1b[0m"
	} else {
		"✓"
	}
}

/// Red cross for failed checks (stdout).
pub fn cross_mark() -> &'static str {
	if color_stdout() {
		"\x1b[31mX\x1b[0m"
	} else {
		"✗"
	}
}

/// Yellow bold `warning:` label for stderr.
pub fn warn_label() -> &'static str {
	if color_stderr() {
		"\x1b[33;1mwarning:\x1b[0m"
	} else {
		"warning:"
	}
}

/// Red bold `error:` label for stderr.
pub fn error_label() -> &'static str {
	if color_stderr() {
		"\x1b[31;1merror:\x1b[0m"
	} else {
		"error:"
	}
}

/// Bold section header text on stdout.
pub fn bold(s: &str) -> String {
	if color_stdout() {
		format!("\x1b[1m{s}\x1b[0m")
	} else {
		s.to_owned()
	}
}

/// Bright green text on stdout (for "OK" / passing results).
pub fn green(s: &str) -> String {
	if color_stdout() {
		format!("\x1b[92m{s}\x1b[0m")
	} else {
		s.to_owned()
	}
}

/// Red text on stdout (for "FAIL" / failing results).
pub fn red(s: &str) -> String {
	if color_stdout() {
		format!("\x1b[31m{s}\x1b[0m")
	} else {
		s.to_owned()
	}
}

/// Create a new count-based progress bar for item downloads.
pub fn new_count_progress(total: u64, message: impl Into<String>) -> ProgressBar {
	let pb = ProgressBar::new(total);
	pb.set_style(
		ProgressStyle::with_template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
			.expect("template is valid")
			.progress_chars("=> "),
	);
	pb.set_message(message.into());
	pb
}

/// Create a new steady-ticking spinner that prints above the progress bar line.
///
/// indicatif automatically suppresses the spinner when stderr is not a TTY.
pub fn new_spinner(message: impl Into<String>) -> ProgressBar {
	let pb = ProgressBar::new_spinner();
	pb.set_style(
		ProgressStyle::with_template("{spinner:.green} {msg}")
			.unwrap()
			.tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
	);
	pb.enable_steady_tick(Duration::from_millis(80));
	pb.set_message(message.into());
	pb
}
