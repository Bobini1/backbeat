//! Package `.ksh` and `.kson` files into [`BackbeatFile`]s.

use std::io;
use std::path::Path;

use rg_formats::ksh::{self, BodyOption, MeasureEvent};
use rg_formats::kson;

use crate::seen_cache::SeenCache;
use backbeat_core::{BackbeatFile, BackbeatFileError};

use crate::deps::ensure_asset;

const AUDIO_EXTS: &[&str] = &["ogg", "mp3", "wav", "flac"];
const VIDEO_EXTS: &[&str] = &[
	"mp4", "wmv", "m4v", "webm", "mpg", "mpeg", "m1v", "m2v", "avi",
];
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp", "tga"];

const JACKET_PRESETS: &[&str] = &["nowprinting1", "nowprinting2", "nowprinting3"];
const BG_PRESETS: &[&str] = &[
	"desert", "grass", "night", "deepsea", "sky", "sunset", "ocean", "flame", "cyber", "space",
	"mars", "cloudy", "fantasy",
];
const LAYER_PRESETS: &[&str] = &[
	"arrow", "smoke", "wave", "techno", "snow", "sakura", "hidden",
];
const CHOKKAKU_SE_PRESETS: &[&str] = &["up", "down", "swing", "mute"];
const FX_SE_PRESETS: &[&str] = &["clap", "clap_punchy", "clap_impact", "snare", "snare_lo"];

fn is_filesystem_preset(value: &str, presets: &[&str]) -> bool {
	presets
		.iter()
		.any(|preset| preset.eq_ignore_ascii_case(value))
}

fn ksh_layer_separator(layer: &str, version: &str) -> char {
	if layer.contains("../") {
		// KSM uses slash for old versions, but charts in the wild use ../ layer paths.
		return ';';
	}

	let parsed_version = lexical::parse_partial::<u16, _>(version)
		.map(|(version, _)| version)
		.unwrap_or(100);
	if parsed_version >= 166 { ';' } else { '/' }
}

fn ksh_layer_filename<'a>(layer: &'a str, version: &str) -> &'a str {
	let separator = ksh_layer_separator(layer, version);
	layer.split(separator).next().unwrap_or("").trim()
}

/// Errors that can occur when packaging a KSH or KSON file into a [`BackbeatFile`].
#[derive(Debug, thiserror::Error)]
pub enum KsmPackageError {
	#[error(transparent)]
	FromFile(#[from] crate::error::FromFileError),

	#[error("could not parse KSH chart: {0}")]
	ParseKsh(#[from] ksh::LoadError),

	#[error("could not parse KSON chart: {0}")]
	ParseKson(#[from] kson::LoadError),

	#[error("could not build BackbeatFile: {0}")]
	Build(#[from] BackbeatFileError),

	#[error("could not resolve KSM assets: {0}")]
	Io(#[from] io::Error),
}

pub(crate) fn resolve_assets_ksh(
	bb: &mut BackbeatFile,
	ksh_path: &Path,
	cache: &SeenCache,
) -> Result<(), KsmPackageError> {
	let chart_dir = ksh_path.parent().unwrap_or(Path::new("."));
	let chart_bytes = bb.chart.decompress();
	let chart = ksh::from_bytes(&chart_bytes)?;
	resolve_ksh_assets(bb, chart_dir, &chart, cache)?;
	Ok(())
}

pub(crate) fn resolve_assets_kson(
	bb: &mut BackbeatFile,
	kson_path: &Path,
	cache: &SeenCache,
) -> Result<(), KsmPackageError> {
	let chart_dir = kson_path.parent().unwrap_or(Path::new("."));
	let chart_bytes = bb.chart.decompress();
	let chart = kson::from_bytes(&chart_bytes)?;
	resolve_kson_assets(bb, chart_dir, &chart, cache)?;
	Ok(())
}

fn resolve_ksh_assets(
	bb: &mut BackbeatFile,
	chart_dir: &Path,
	chart: &ksh::Chart,
	cache: &SeenCache,
) -> Result<(), KsmPackageError> {
	let header = &chart.header;

	for segment in &header.audio {
		ensure_ksh_asset(bb, chart_dir, cache, segment, AUDIO_EXTS)?;
	}

	let jacket = &header.jacket;
	if !jacket.is_empty() && !is_filesystem_preset(jacket, JACKET_PRESETS) {
		ensure_ksh_asset(bb, chart_dir, cache, jacket, IMAGE_EXTS)?;
	}

	if let Some(icon) = header.unknown.get("icon").filter(|icon| !icon.is_empty()) {
		ensure_ksh_asset(bb, chart_dir, cache, icon, IMAGE_EXTS)?;
	}

	if !header.title_img.is_empty() {
		ensure_ksh_asset(bb, chart_dir, cache, &header.title_img, IMAGE_EXTS)?;
	}

	if !header.artist_img.is_empty() {
		ensure_ksh_asset(bb, chart_dir, cache, &header.artist_img, IMAGE_EXTS)?;
	}

	for segment in &header.bg {
		if is_filesystem_preset(segment, BG_PRESETS) {
			continue;
		}
		ensure_ksh_asset(bb, chart_dir, cache, segment, IMAGE_EXTS)?;
	}

	let chart_version = if !header.version_compat.is_empty() {
		&header.version_compat
	} else {
		&header.version
	};
	let layer_name = ksh_layer_filename(&header.layer, chart_version);
	if !layer_name.is_empty() && !is_filesystem_preset(layer_name, LAYER_PRESETS) {
		ensure_ksh_asset(bb, chart_dir, cache, layer_name, IMAGE_EXTS)?;
	}

	if !header.video.is_empty() {
		ensure_ksh_asset(bb, chart_dir, cache, &header.video, VIDEO_EXTS)?;
	}

	for measure in &chart.measures {
		for event in &measure.events {
			let MeasureEvent::Option(opt) = event else {
				continue;
			};
			match opt {
				BodyOption::ChokkakuSe(name) => {
					if !CHOKKAKU_SE_PRESETS.contains(&name.as_str()) && !name.is_empty() {
						ensure_ksh_asset(bb, chart_dir, cache, name, AUDIO_EXTS)?;
					}
				}
				BodyOption::Unknown { key, value } if key == "fx-l_se" || key == "fx-r_se" => {
					let name = value.split(';').next().unwrap_or("").trim();
					if !FX_SE_PRESETS.contains(&name) && !name.is_empty() {
						ensure_ksh_asset(bb, chart_dir, cache, name, AUDIO_EXTS)?;
					}
				}
				_ => {}
			}
		}
	}

	Ok(())
}

fn ensure_ksh_asset(
	bb: &mut BackbeatFile,
	chart_dir: &Path,
	cache: &SeenCache,
	path: &str,
	extensions: &[&str],
) -> Result<(), KsmPackageError> {
	let path = path.strip_prefix('/').unwrap_or(path);
	ensure_asset(bb, chart_dir, cache, path, extensions)?;
	Ok(())
}

fn resolve_kson_assets(
	bb: &mut BackbeatFile,
	chart_dir: &Path,
	chart: &kson::Kson,
	cache: &SeenCache,
) -> Result<(), KsmPackageError> {
	if let Some(audio) = &chart.audio
		&& let Some(bgm) = &audio.bgm
	{
		if let Some(f) = bgm.filename.as_deref().filter(|s| !s.is_empty()) {
			ensure_asset(bb, chart_dir, cache, f, AUDIO_EXTS)?;
		}

		if let Some(legacy) = &bgm.legacy {
			for f in &legacy.fp_filenames {
				if !f.is_empty() {
					ensure_asset(bb, chart_dir, cache, f, AUDIO_EXTS)?;
				}
			}
		}
	}

	if let Some(j) = chart
		.meta
		.jacket_filename
		.as_deref()
		.filter(|s| !s.is_empty() && !is_filesystem_preset(s, JACKET_PRESETS))
	{
		ensure_asset(bb, chart_dir, cache, j, IMAGE_EXTS)?;
	}

	if let Some(bg) = &chart.bg {
		if let Some(legacy) = &bg.legacy {
			if let Some(bg_arr) = &legacy.bg {
				for entry in bg_arr {
					if let Some(f) = entry
						.filename
						.as_deref()
						.filter(|s| !s.is_empty() && !is_filesystem_preset(s, BG_PRESETS))
					{
						ensure_asset(bb, chart_dir, cache, f, IMAGE_EXTS)?;
					}
				}
			}

			if let Some(layer) = &legacy.layer
				&& let Some(f) = layer
					.filename
					.as_deref()
					.filter(|s| !s.is_empty() && !is_filesystem_preset(s, LAYER_PRESETS))
			{
				ensure_asset(bb, chart_dir, cache, f, IMAGE_EXTS)?;
			}

			if let Some(movie) = &legacy.movie
				&& let Some(f) = movie.filename.as_deref().filter(|s| !s.is_empty())
			{
				ensure_asset(bb, chart_dir, cache, f, VIDEO_EXTS)?;
			}
		}

		if let Some(f) = bg.filename.as_deref().filter(|s| !s.is_empty()) {
			ensure_asset(bb, chart_dir, cache, f, IMAGE_EXTS)?;
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn filesystem_presets_are_ascii_case_insensitive() {
		assert!(is_filesystem_preset("NOWPRINTING1", JACKET_PRESETS));
		assert!(is_filesystem_preset("DeEpSeA", BG_PRESETS));
		assert!(is_filesystem_preset("ArRoW", LAYER_PRESETS));
	}

	#[test]
	fn parse_layer_name_prefers_semicolon_for_modern_layer() {
		let layer = "../../../imgs/bg/truesakura.jpg;1200;0";
		assert_eq!(
			ksh_layer_filename(layer, "171"),
			"../../../imgs/bg/truesakura.jpg"
		);
	}

	#[test]
	fn parse_layer_name_uses_slash_for_legacy_layer() {
		let layer = "arrow/1200/3";
		assert_eq!(ksh_layer_filename(layer, "100"), "arrow");
	}

	#[test]
	fn parse_layer_name_preserves_up_pathed_legacy_layer() {
		let layer = "../../../Abyssal Wind/bg.gif";
		assert_eq!(ksh_layer_filename(layer, "140d"), layer);
	}

	#[test]
	fn parse_layer_name_accepts_a_version_suffix() {
		let layer = "truesakura.jpg;1200;0";
		assert_eq!(ksh_layer_filename(layer, "171d"), "truesakura.jpg");
	}

	#[test]
	fn parse_layer_name_defaults_to_legacy_separator_when_no_version_is_present() {
		let layer = "truesakura.jpg;1200;0";
		assert_eq!(ksh_layer_filename(layer, ""), layer);
	}

	#[test]
	fn package_resolves_up_pathed_icon_asset() {
		use std::fs;

		let tmp = tempfile::TempDir::new().unwrap();
		let chart_dir = tmp.path().join("charts/song");
		fs::create_dir_all(&chart_dir).unwrap();
		fs::write(tmp.path().join("banner.jpg"), b"banner").unwrap();
		fs::write(
			chart_dir.join("song.ksh"),
			b"title=Test\nicon=/../../banner.jpg\n--\n0000|00|--\n--\n",
		)
		.unwrap();

		let packages = crate::package(chart_dir.join("song.ksh")).unwrap();
		assert!(
			packages[0]
				.assets
				.contains_key(&backbeat_core::AssetPath::from_path("../../banner.jpg").unwrap())
		);
	}
}
