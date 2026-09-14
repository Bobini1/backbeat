//! Package `.ssc` files into a [`BackbeatFile`].

use std::io;
use std::path::Path;

use rg_formats::sm_msd::{self, MsdElement};

use crate::seen_cache::SeenCache;
use backbeat_core::{BackbeatFile, BackbeatFileError};

use crate::deps::ensure_asset;

/// Errors that can occur when packaging an SSC file into a [`BackbeatFile`].
#[derive(Debug, thiserror::Error)]
pub enum SscPackageError {
	#[error(transparent)]
	FromFile(#[from] crate::error::FromFileError),

	#[error("SSC file is not valid UTF-8: {0}")]
	InvalidFileUtf8(#[from] std::str::Utf8Error),

	#[error("could not build BackbeatFile: {0}")]
	Build(#[from] BackbeatFileError),

	#[error("could not resolve SSC assets: {0}")]
	Io(#[from] io::Error),
}

pub(crate) fn resolve_assets(
	bb: &mut BackbeatFile,
	ssc_path: &Path,
	cache: &SeenCache,
) -> Result<(), SscPackageError> {
	let chart_dir = ssc_path.parent().unwrap_or(Path::new("."));
	let bytes = bb.chart.decompress();
	std::str::from_utf8(&bytes)?;
	let msd = sm_msd::from_bytes(&bytes);
	let elements = msd.elements;

	let split_pos = elements
		.iter()
		.position(|el| &*el.tag == b"NOTEDATA")
		.unwrap_or(elements.len());

	let (song_elements, chart_stream) = elements.split_at(split_pos);
	resolve_msd_assets(bb, chart_dir, song_elements, chart_stream, cache)?;
	Ok(())
}

fn resolve_msd_assets(
	bb: &mut BackbeatFile,
	chart_dir: &Path,
	song_elements: &[MsdElement],
	chart_stream: &[MsdElement],
	cache: &SeenCache,
) -> Result<(), SscPackageError> {
	for tag in &[
		b"MUSIC".as_ref(),
		b"BANNER",
		b"BACKGROUND",
		b"CDTITLE",
		b"LYRICSPATH",
	] {
		if let Some(path) = get_tag_value(song_elements, tag) {
			let path = std::str::from_utf8(path)?;
			ensure_asset(bb, chart_dir, cache, path, &[])?;
		}
	}

	for el in chart_stream {
		if &*el.tag == b"MUSIC"
			&& let Some(val) = el.values.first()
		{
			let path = std::str::from_utf8(val)?;
			ensure_asset(bb, chart_dir, cache, path, &[])?;
		}
	}

	Ok(())
}

fn get_tag_value<'a>(elements: &'a [MsdElement], tag: &[u8]) -> Option<&'a [u8]> {
	elements
		.iter()
		.find(|el| &*el.tag == tag)
		.and_then(|el| el.values.first())
		.map(Box::as_ref)
		.filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn make_ssc(charts: &[(&str, &str, usize)]) -> Vec<u8> {
		let mut s = String::from(
			"#TITLE:Test Song;\n\
			 #ARTIST:Test Artist;\n\
			 #MUSIC:song.ogg;\n\
			 #BPMS:0.000=120.000;\n",
		);
		for (steps_type, diff, level) in charts {
			s.push_str(&format!(
				"#NOTEDATA:;\n\
				 #STEPSTYPE:{steps_type};\n\
				 #DESCRIPTION:;\n\
				 #DIFFICULTY:{diff};\n\
				 #METER:{level};\n\
				 #NOTES:\n\
				 0000\n\
				 ;\n"
			));
		}
		s.into_bytes()
	}

	fn no_cache() -> SeenCache {
		SeenCache::new()
	}

	fn package_bytes(bytes: &[u8], path: &str, cache: &SeenCache) -> BackbeatFile {
		let chart_dir = Path::new(".");
		let chart_path = chart_dir.join(path);
		let mut bb = crate::base::from_bytes(bytes, &chart_path, cache).unwrap();
		resolve_assets(&mut bb, &chart_path, cache).unwrap();

		bb
	}

	#[test]
	fn convert_whole_file_preserves_bytes() {
		let raw = make_ssc(&[("dance-single", "easy", 3), ("dance-single", "hard", 8)]);
		let bb = package_bytes(&raw, "test.ssc", &no_cache());
		assert_eq!(bb.filename.extension(), Some("ssc"));
		assert_eq!(bb.chart.decompress(), raw);
	}

	#[test]
	fn rejects_non_unicode_files() {
		let raw = b"#TITLE:\x80;\n#MUSIC:song.ogg;\n";
		let chart_path = Path::new("test.ssc");
		let cache = no_cache();
		let mut bb = crate::base::from_bytes(raw, chart_path, &cache).unwrap();

		let error = resolve_assets(&mut bb, chart_path, &cache).unwrap_err();

		assert!(matches!(error, SscPackageError::InvalidFileUtf8(_)));
	}
}
