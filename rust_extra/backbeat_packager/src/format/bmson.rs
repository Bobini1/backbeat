//! Package `.bmson` files into [`BackbeatFile`]s.

use std::io;
use std::path::Path;

use rg_formats::bmson;

use crate::{error::FromFileError, seen_cache::SeenCache};
use backbeat_core::{AssetPath, BackbeatFile, BackbeatFileError};

use crate::deps::ensure_asset;

// taken from beatoraja
const AUDIO_EXTS: &[&str] = &["wav", "flac", "ogg", "mp3"];
const VIDEO_EXTS: &[&str] = &[
	"mp4", "wmv", "m4v", "webm", "mpg", "mpeg", "m1v", "m2v", "avi",
];
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "gif", "bmp", "png", "tga"];

/// Errors that can occur when packaging a BMSON file into a [`BackbeatFile`].
#[derive(Debug, thiserror::Error)]
pub enum BmsonPackageError {
	#[error(transparent)]
	FromFile(#[from] crate::error::FromFileError),

	#[error("could not parse BMSON file: {0}")]
	Parse(#[from] bmson::LoadError),

	#[error("could not build BackbeatFile: {0}")]
	Build(#[from] BackbeatFileError),

	#[error("could not resolve BMSON assets: {0}")]
	Io(#[from] io::Error),
}

pub(crate) fn resolve_assets(
	bb: &mut BackbeatFile,
	bmson_path: &Path,
	cache: &SeenCache,
) -> Result<(), BmsonPackageError> {
	let chart_dir = bmson_path.parent().unwrap_or(Path::new("."));
	let chart_bytes = bb.chart.decompress();
	let chart = bmson::from_bytes(&chart_bytes)?;

	let all_audio = chart
		.sound_channels
		.iter()
		.chain(chart.key_channels.iter())
		.chain(chart.mine_channels.iter());

	for channel in all_audio {
		ensure_asset(bb, chart_dir, cache, &channel.name, AUDIO_EXTS)?;
	}

	if let Some(bga) = &chart.bga {
		for header in &bga.bga_header {
			let asset_path = AssetPath::from_path(&header.name).map_err(FromFileError::from)?;

			if cache
				.check_many_exts(chart_dir, &asset_path, VIDEO_EXTS)
				.is_some()
			{
				ensure_asset(bb, chart_dir, cache, &header.name, VIDEO_EXTS)?;
			} else {
				ensure_asset(bb, chart_dir, cache, &header.name, IMAGE_EXTS)?;
			}
		}
	}

	if let Some(preview) = &chart.info.preview_music {
		ensure_asset(bb, chart_dir, cache, preview, AUDIO_EXTS)?;
	}

	let info_images = [
		chart.info.back_image.as_deref(),
		chart.info.eyecatch_image.as_deref(),
		chart.info.title_image.as_deref(),
		chart.info.banner_image.as_deref(),
	];
	for rel_str in info_images.into_iter().flatten() {
		ensure_asset(bb, chart_dir, cache, rel_str, &[])?;
	}

	Ok(())
}
