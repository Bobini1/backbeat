//! Package `.dwi` files into [`BackbeatFile`]s.

use std::io;
use std::path::Path;

use rg_formats::sm_msd;

use crate::deps::ensure_asset;
use crate::seen_cache::SeenCache;
use backbeat_core::{BackbeatFile, BackbeatFileError};

#[derive(Debug, thiserror::Error)]
pub enum DwiPackageError {
	#[error(transparent)]
	FromFile(#[from] crate::error::FromFileError),

	#[error("DWI file is not valid UTF-8: {0}")]
	InvalidFileUtf8(#[from] std::str::Utf8Error),

	#[error("could not build BackbeatFile: {0}")]
	Build(#[from] BackbeatFileError),

	#[error("could not resolve DWI assets: {0}")]
	Io(#[from] io::Error),
}

pub(crate) fn resolve_assets(
	bb: &mut BackbeatFile,
	dwi_path: &Path,
	cache: &SeenCache,
) -> Result<(), DwiPackageError> {
	let chart_dir = dwi_path.parent().unwrap_or(Path::new("."));
	let bytes = bb.chart.decompress();
	std::str::from_utf8(&bytes)?;
	let msd = sm_msd::from_bytes(&bytes);
	for tag in [b"FILE".as_ref(), b"BANNER", b"BACKGROUND", b"CDTITLE"] {
		let Some(path) = msd
			.elements
			.iter()
			.find(|element| element.tag.as_ref() == tag)
			.and_then(|element| element.values.first())
		else {
			continue;
		};
		ensure_asset(bb, chart_dir, cache, std::str::from_utf8(path)?, &[])?;
	}
	Ok(())
}
