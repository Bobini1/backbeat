//! Package `.tja` (TJAPlayer3 / Taiko) files into a [`BackbeatFile`].

use std::io;
use std::path::{Path, PathBuf};

use rg_formats::tja;

use crate::seen_cache::SeenCache;
use backbeat_core::{BackbeatFile, BackbeatFileError};

use crate::deps::ensure_asset;

const AUDIO_EXTS: &[&str] = &["wav", "flac", "ogg", "mp3"];
const VIDEO_EXTS: &[&str] = &[
	"mp4", "wmv", "m4v", "webm", "mpg", "mpeg", "m1v", "m2v", "avi",
];
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "gif", "bmp", "png", "tga"];

/// Errors that can occur when packaging a TJA file into a [`BackbeatFile`].
#[derive(Debug, thiserror::Error)]
pub enum TjaPackageError {
	#[error(transparent)]
	FromFile(#[from] crate::error::FromFileError),

	#[error("could not build BackbeatFile: {0}")]
	Build(#[from] BackbeatFileError),

	#[error("could not resolve TJA assets: {0}")]
	Io(#[from] io::Error),
}

pub(crate) fn resolve_assets(
	bb: &mut BackbeatFile,
	tja_path: &Path,
	cache: &SeenCache,
) -> Result<(), TjaPackageError> {
	let chart_dir = tja_path.parent().unwrap_or(Path::new("."));
	let raw_bytes = bb.chart.decompress();
	let chart = tja::from_bytes(&raw_bytes);

	if let Some(wave) = &chart.metadata.wave {
		let rel = PathBuf::from(wave);
		ensure_asset(bb, chart_dir, cache, &rel, AUDIO_EXTS)?;
	}

	if let Some(bg_image) = &chart.metadata.bg_image {
		let rel = PathBuf::from(bg_image);
		ensure_asset(bb, chart_dir, cache, &rel, IMAGE_EXTS)?;
	}

	if let Some(bg_movie) = &chart.metadata.bg_movie {
		let rel = PathBuf::from(bg_movie);
		if cache.check_many_exts(chart_dir, &rel, VIDEO_EXTS).is_some() {
			ensure_asset(bb, chart_dir, cache, &rel, VIDEO_EXTS)?;
		} else {
			ensure_asset(bb, chart_dir, cache, &rel, IMAGE_EXTS)?;
		}
	}

	Ok(())
}
