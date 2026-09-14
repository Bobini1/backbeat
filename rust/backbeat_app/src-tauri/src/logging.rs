//! Process-wide logging setup for the GUI.
//!
//! Two `tracing-subscriber` layers share one [`EnvFilter`]:
//!
//! - a `fmt` layer to stdout (useful for `tauri dev` and terminal launches), and
//! - a `fmt` layer over a daily-rotating [`tracing_appender`] file writer that
//!   writes into the open store's `logs/` directory.

use std::path::Path;
use std::time::{Duration, SystemTime};

use tracing::Subscriber;
use tracing_appender::non_blocking::NonBlocking;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::fmt::FormatFields;
use tracing_subscriber::fmt::format::{DefaultFields, Writer};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

/// Filename prefix for rotated log files (`backbeat.YYYY-MM-DD.txt`).
const FILE_PREFIX: &str = "backbeat";
/// Filename suffix for rotated log files.
const FILE_SUFFIX: &str = "txt";
/// How many days of rotated log files to keep.
const RETAIN_DAYS: u64 = 7;

/// Write fields without ANSI colour codes to a file.
#[derive(Debug, Default)]
struct PlainFields(DefaultFields);

impl<'w> FormatFields<'w> for PlainFields {
	fn format_fields<R: RecordFields>(&self, writer: Writer<'w>, fields: R) -> std::fmt::Result {
		self.0.format_fields(writer, fields)
	}
}

/// Initialise logging, with rotating files :)
pub fn init(log_dir: &Path) {
	install_panic_hook();

	let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

	let stdout_layer = fmt::layer().with_filter(env_filter.clone());
	let base = tracing_subscriber::registry().with(stdout_layer);

	// Box the subscriber so the file-layer-on / file-layer-off cases share one
	// type. The global default requires `Send + Sync + 'static`, which both
	// arms satisfy.
	let subscriber: Box<dyn Subscriber + Send + Sync + 'static> = match file_writer(log_dir) {
		Some((writer, guard)) => {
			// Keep the flush guard alive for the entire process lifetime.
			std::mem::forget(guard);
			let file_layer = fmt::layer()
				.fmt_fields(PlainFields::default())
				.with_ansi(false)
				.with_writer(writer)
				.with_filter(env_filter);
			Box::new(base.with(file_layer))
		}
		None => Box::new(base),
	};

	subscriber.init();
	prune_dir(log_dir, RETAIN_DAYS);
}

/// Build the non-blocking rolling file writer, creating the log directory if
/// needed. Returns `None` if the directory can't be created or the appender
/// can't be built (in which case we fall back to stdout-only logging).
fn file_writer(dir: &Path) -> Option<(NonBlocking, tracing_appender::non_blocking::WorkerGuard)> {
	if let Err(err) = std::fs::create_dir_all(dir) {
		eprintln!(
			"could not create log directory {}: {err}; falling back to stdout-only logging",
			dir.display()
		);
		return None;
	}
	let appender = RollingFileAppender::builder()
		.rotation(Rotation::DAILY)
		.filename_prefix(FILE_PREFIX)
		.filename_suffix(FILE_SUFFIX)
		.build(dir)
		.map_err(|err| {
			eprintln!(
				"could not open log appender in {}: {err}; falling back to stdout-only logging",
				dir.display()
			);
		})
		.ok()?;
	Some(tracing_appender::non_blocking(appender))
}

/// Forward panics through `tracing::error!` so they're captured by the file
/// layer, then crash normally.
fn install_panic_hook() {
	let prev = std::panic::take_hook();
	std::panic::set_hook(Box::new(move |info| {
		let location = info
			.location()
			.map(|l| format!("{}:{}", l.file(), l.line()));
		let message = info
			.payload()
			.downcast_ref::<&str>()
			.copied()
			.or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
			.unwrap_or("<non-string panic payload>");
		tracing::error!(panic = %message, location = ?location.as_deref(), "panic");
		prev(info);
	}));
}

fn prune_dir(dir: &std::path::Path, max_days: u64) {
	let cutoff = SystemTime::now()
		.checked_sub(Duration::from_secs(max_days * 86_400))
		.unwrap_or(SystemTime::UNIX_EPOCH);
	let Ok(entries) = std::fs::read_dir(dir) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		if !is_log_file(&path) {
			continue;
		}
		let Ok(meta) = entry.metadata() else {
			continue;
		};
		let Ok(mtime) = meta.modified() else {
			continue;
		};
		if mtime < cutoff {
			let _ = std::fs::remove_file(path);
		}
	}
}

/// Whether `path` is one of our rotated log files (`backbeat*.txt`).
fn is_log_file(path: &std::path::Path) -> bool {
	let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
		return false;
	};
	name.starts_with(FILE_PREFIX) && path.extension().is_some_and(|ext| ext == FILE_SUFFIX)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::fs;

	#[cfg(unix)]
	#[test]
	fn prunes_old_log_files_but_keeps_recent() {
		use std::fs::FileTimes;
		let dir = tempfile::TempDir::new().unwrap();
		let root = dir.path();

		let old = root.join("backbeat.2020-01-01.txt");
		let recent = root.join("backbeat.2099-01-01.txt");
		let ignored = root.join("notes.txt");
		let other = root.join("backbeat.2020-01-01.log");
		fs::write(&old, b"x").unwrap();
		fs::write(&recent, b"x").unwrap();
		fs::write(&ignored, b"x").unwrap();
		fs::write(&other, b"x").unwrap();

		// Set mtimes: `old` long ago, `recent` in the future-ish (now).
		let ancient = SystemTime::UNIX_EPOCH + Duration::from_secs(60);
		let times_old = FileTimes::new().set_modified(ancient).set_accessed(ancient);
		let times_recent = FileTimes::new()
			.set_modified(SystemTime::now())
			.set_accessed(SystemTime::now());
		fs::File::open(&old).unwrap().set_times(times_old).unwrap();
		fs::File::open(&recent)
			.unwrap()
			.set_times(times_recent)
			.unwrap();

		prune_dir(root, 7);

		assert!(!old.exists(), "old log file should have been pruned");
		assert!(recent.exists(), "recent log file should be kept");
		assert!(ignored.exists(), "non-log .txt should be left alone");
		assert!(
			other.exists(),
			"wrong-extension backbeat file should be left alone"
		);
	}
}
