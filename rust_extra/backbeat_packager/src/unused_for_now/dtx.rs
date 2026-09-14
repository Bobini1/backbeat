//! Package `.dtx` (DTXMania) files into [`BackbeatFile`]s.

use std::io;
use std::path::{Path, PathBuf};

use encoding_rs::SHIFT_JIS;
use rg_formats::dtx;

use crate::seen_cache::SeenCache;
use backbeat_core::{BackbeatFile, BackbeatFileError};

use crate::deps::ensure_asset;

const AUDIO_EXTS: &[&str] = &["wav", "flac", "ogg", "mp3"];
const VIDEO_EXTS: &[&str] = &[
	"mp4", "wmv", "m4v", "webm", "mpg", "mpeg", "m1v", "m2v", "avi",
];
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "gif", "bmp", "png", "tga"];

/// Errors that can occur when packaging a DTX file into a [`BackbeatFile`].
#[derive(Debug, thiserror::Error)]
pub enum DtxPackageError {
	#[error(transparent)]
	FromFile(#[from] crate::error::FromFileError),

	#[error("could not build BackbeatFile: {0}")]
	Build(#[from] BackbeatFileError),

	#[error("could not resolve DTX assets: {0}")]
	Io(#[from] io::Error),
}

pub(crate) fn resolve_assets(
	bb: &mut BackbeatFile,
	dtx_path: &Path,
	cache: &SeenCache,
) -> Result<(), DtxPackageError> {
	let chart_dir = dtx_path.parent().unwrap_or(Path::new("."));
	let chart_bytes = bb.chart.decompress();
	let chart = dtx::from_bytes(&chart_bytes);

	for filename in chart.metadata.wav.values() {
		let rel = decode_dtx_path(filename.as_bytes());
		ensure_asset(bb, chart_dir, cache, &rel, AUDIO_EXTS)?;
	}

	for filename in chart.metadata.bmp.values() {
		let rel = decode_dtx_path(filename.as_bytes());
		if cache.check_many_exts(chart_dir, &rel, VIDEO_EXTS).is_some() {
			ensure_asset(bb, chart_dir, cache, &rel, VIDEO_EXTS)?;
		} else {
			ensure_asset(bb, chart_dir, cache, &rel, IMAGE_EXTS)?;
		}
	}

	for filename in chart.metadata.avi.values() {
		let rel = decode_dtx_path(filename.as_bytes());
		if cache.check_many_exts(chart_dir, &rel, VIDEO_EXTS).is_some() {
			ensure_asset(bb, chart_dir, cache, &rel, VIDEO_EXTS)?;
		} else {
			ensure_asset(bb, chart_dir, cache, &rel, IMAGE_EXTS)?;
		}
	}

	if let Some(preview) = &chart.metadata.preview {
		let rel = PathBuf::from(preview);
		if cache.check_many_exts(chart_dir, &rel, AUDIO_EXTS).is_some() {
			ensure_asset(bb, chart_dir, cache, &rel, AUDIO_EXTS)?;
		} else {
			ensure_asset(bb, chart_dir, cache, &rel, IMAGE_EXTS)?;
		}
	}

	if let Some(preimage) = &chart.metadata.preimage {
		let rel = PathBuf::from(preimage);
		ensure_asset(bb, chart_dir, cache, &rel, IMAGE_EXTS)?;
	}

	Ok(())
}

fn decode_dtx_path(bytes: &[u8]) -> PathBuf {
	if let Ok(s) = std::str::from_utf8(bytes) {
		return PathBuf::from(s);
	}
	let (cow, _enc, _had_errors) = SHIFT_JIS.decode(bytes);
	PathBuf::from(cow.as_ref())
}
