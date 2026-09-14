//! Package `.bms` / `.bme` / `.bml` / `.pms` files into [`BackbeatFile`]s.
//!
//! A BMS file is treated as opaque bytes - we do not re-encode or reinterpret
//! the note data. The raw file contents are gzip-compressed and stored in the
//! `chart` field exactly as they appear on disk. Asset filenames referenced by
//! `#WAVxx` and `#BMPxx` headers are resolved relative to the chart file's
//! directory, hashed, and recorded in `assets`.
//!
//! ## Extension substitution
//!
//! BMS charts commonly reference audio as `.wav` even when the actual files on
//! disk are `.ogg`, `.mp3`, etc. (packs are often distributed with converted
//! audio). `resolve_asset` mimics lr2oraja's behaviour: it tries the
//! exact path first, then substitutes common audio and image extensions until
//! one exists. The asset key in the resulting [`BackbeatFile`] reflects the
//! actual extension found on disk.
//!
//! Unused `#WAVxx` and `#BMPxx` definitions are not resolved. Used files absent
//! from disk under any fallback extension are silently skipped.

use std::collections::HashSet;
use std::io;
use std::path::Path;

use rg_formats::bms;

use crate::seen_cache::SeenCache;
use backbeat_core::{AssetPath, BackbeatFile, BackbeatFileError};

use crate::deps::ensure_asset;
use crate::error::FromFileError;

const AUDIO_EXTS: &[&str] = &["wav", "flac", "ogg", "mp3"];
const VIDEO_EXTS: &[&str] = &[
	"mp4", "wmv", "m4v", "webm", "mpg", "mpeg", "m1v", "m2v", "avi",
];
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "gif", "bmp", "png", "tga"];

/// Errors that can occur when packaging a BMS file into a [`BackbeatFile`].
#[derive(Debug, thiserror::Error)]
pub enum BmsPackageError {
	#[error(transparent)]
	FromFile(#[from] crate::error::FromFileError),

	#[error("could not parse BMS file: {0}")]
	Parse(#[from] bms::LoadError),

	#[error("could not build BackbeatFile: {0}")]
	Build(#[from] BackbeatFileError),

	#[error("could not resolve BMS assets: {0}")]
	Io(#[from] io::Error),
}

/// Resolve BMS-referenced assets into `assets`.
pub(crate) fn resolve_assets(
	bb: &mut BackbeatFile,
	bms_path: &Path,
	cache: &SeenCache,
) -> Result<(), BmsPackageError> {
	let chart_dir = bms_path.parent().unwrap_or(Path::new("."));
	let chart_bytes = bb.chart.decompress();
	let ext = crate::util::safe_extension(bms_path).unwrap_or("");
	let chart = bms::from_bytes(&chart_bytes, ext, bms::BmsRandomStrategy::AlwaysFirstBranch)?;

	let used_wav_ids = chart.used_wav_lookup_ids().collect::<HashSet<_>>();
	for (id, path) in &chart.metadata.wav {
		if !used_wav_ids.contains(id) {
			continue;
		}
		ensure_asset(bb, chart_dir, cache, path, AUDIO_EXTS)?;
	}

	let used_bmp_ids = chart.used_bmp_lookup_ids().collect::<HashSet<_>>();
	for (id, path) in &chart.metadata.bmp {
		if !used_bmp_ids.contains(id) {
			continue;
		}
		let asset_path = AssetPath::from_path(path).map_err(FromFileError::from)?;
		if cache
			.check_many_exts(chart_dir, &asset_path, VIDEO_EXTS)
			.is_some()
		{
			ensure_asset(bb, chart_dir, cache, path, VIDEO_EXTS)?;
		} else {
			ensure_asset(bb, chart_dir, cache, path, IMAGE_EXTS)?;
		}
	}

	let meta_paths = [
		chart.metadata.stagefile.as_deref(),
		chart.metadata.banner.as_deref(),
		chart.metadata.backbmp.as_deref(),
		chart.metadata.preview.as_deref(),
	];
	for path in meta_paths.into_iter().flatten() {
		let asset_path = AssetPath::from_path(path).map_err(FromFileError::from)?;
		if cache
			.check_many_exts(chart_dir, &asset_path, IMAGE_EXTS)
			.is_some()
		{
			ensure_asset(bb, chart_dir, cache, path, IMAGE_EXTS)?;
		} else {
			ensure_asset(bb, chart_dir, cache, path, AUDIO_EXTS)?;
		}
	}

	Ok(())
}
