//! Add Backbeat-owned metadata to an unfractured StepMania chart file.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use rg_formats::sm_msd::{self, MsdElement};

use crate::blacklist::should_ignore_file;

#[derive(Debug, thiserror::Error)]
pub enum SmEnrichError {
	#[error("could not read StepMania chart file: {0}")]
	Io(#[from] std::io::Error),

	#[error("could not parse StepMania chart file: {0}")]
	Parse(#[from] rg_formats::sm::LoadError),
}

/// Enrich an SM file in place. The returned bytes are also useful to callers that want to
/// inspect or atomically write the result themselves.
pub fn enrich_sm_file(path: impl AsRef<Path>) -> Result<Vec<u8>, SmEnrichError> {
	let path = path.as_ref();
	let bytes = fs::read(path)?;
	let enriched = enrich_sm_bytes(path, &bytes)?;
	fs::write(path, &enriched)?;
	Ok(enriched)
}

pub fn enrich_sm_bytes(path: impl AsRef<Path>, bytes: &[u8]) -> Result<Vec<u8>, SmEnrichError> {
	let path = path.as_ref();
	let msd = sm_msd::from_bytes(bytes);
	if is_ssc(path) {
		rg_formats::ssc::from_bytes(bytes, path);
	} else if is_dwi(path) {
		// DWI parsing is deliberately lenient and returns every playable chart.
		// Run it here so enrichment has the same format-level validation path as SM/SSC.
		rg_formats::dwi::from_bytes(bytes, path);
	} else {
		rg_formats::sm::from_bytes(bytes, path)?;
	}
	let header_end = if is_ssc(path) {
		msd.elements
			.iter()
			.position(|element| element.tag.as_ref() == b"NOTEDATA")
			.unwrap_or(msd.elements.len())
	} else {
		msd.elements.len()
	};
	let header = &msd.elements[..header_end];

	let mut generated = vec![element("BKBENRICHED", vec![b"v1".to_vec()])];
	for (tag, value) in resolved_assets(path, header) {
		generated.push(element(tag, vec![value.into_bytes()]));
	}

	let generated_tags = [
		"BKBENRICHED",
		"REALBANNER",
		"REALJACKET",
		"REALMUSIC",
		"REALBACKGROUND",
		"REALCDTITLE",
	];
	let mut output = Vec::new();
	let mut inserted = false;
	let mut elements = Vec::with_capacity(msd.elements.len() + generated.len());
	for source in msd.elements {
		if generated_tags
			.iter()
			.any(|tag| source.tag.as_ref() == tag.as_bytes())
		{
			continue;
		}
		if !inserted && is_first_chart_tag(path, source.tag.as_ref()) {
			elements.extend(generated.iter().cloned());
			inserted = true;
		}
		elements.push(source);
	}
	if !inserted {
		elements.extend(generated);
	}
	output.extend(&sm_msd::serialize_msd_elements(elements));
	Ok(output)
}

fn element(tag: &str, values: Vec<Vec<u8>>) -> MsdElement {
	MsdElement {
		tag: tag.as_bytes().to_vec().into(),
		values: values.into_iter().map(Into::into).collect(),
	}
}

/// Mirrors `Song::TidyUpData` in ITGmania.  This must run before packaging: once an
/// asset has been copied into a bundle, its original sibling names and dimensions are
/// no longer available to the resolver.
fn resolved_assets(path: &Path, elements: &[MsdElement]) -> Vec<(&'static str, String)> {
	let dir = path.parent().unwrap_or(Path::new("."));
	let files = song_files(dir);
	let images = files
		.iter()
		.filter(|file| is_bitmap(file))
		.cloned()
		.collect::<Vec<_>>();
	let blacklisted_images = dwi_blacklisted_images(path, elements);

	let mut assets = DiscoveredAssets {
		music: explicit_asset(elements, if is_dwi(path) { "FILE" } else { "MUSIC" }, dir),
		banner: explicit_asset(elements, "BANNER", dir),
		jacket: explicit_asset(elements, "JACKET", dir),
		background: explicit_asset(elements, "BACKGROUND", dir),
		cdtitle: explicit_asset(elements, "CDTITLE", dir),
		cdimage: None,
		disc: None,
	};

	if assets.music.is_none() {
		let music = files
			.iter()
			.filter(|file| is_stepmania_audio(file))
			.cloned()
			.collect::<Vec<_>>();
		assets.music = music.first().cloned();
		if music.len() > 1 && music[0].stem().starts_with("intro") {
			assets.music = Some(music[1].clone());
		}
	}

	// FindFirstFilenameContaining's comparison order is starts-with, ends-with,
	// then contains.  The rule order below is also ITGmania's.
	if assets.banner.is_none() {
		assets.banner = first_filename_match(&images, &[], &["banner"], &[" bn"]);
	}
	if assets.background.is_none() {
		assets.background = first_filename_match(&images, &[], &["background"], &["bg"]);
	}
	if assets.jacket.is_none() {
		assets.jacket = first_filename_match(&images, &["jk_"], &["jacket", "albumart"], &[]);
	}
	if assets.cdimage.is_none() {
		assets.cdimage = first_filename_match(&images, &[], &[], &["-cd"]);
	}
	if assets.disc.is_none() {
		assets.disc = first_filename_match(&images, &[], &[], &[" disc", " title"]);
	}
	if assets.cdtitle.is_none() {
		assets.cdtitle = first_filename_match(&images, &[], &["cdtitle"], &[]);
	}

	for image in images {
		if assets.banner.is_some() && assets.background.is_some() && assets.cdtitle.is_some() {
			break;
		}
		if blacklisted_images.contains(&image.name.to_ascii_lowercase()) || assets.contains(&image)
		{
			continue;
		}
		let Ok((width, height)) = image::image_dimensions(&image.path) else {
			continue;
		};
		if assets.background.is_none() && width >= 320 && height >= 240 {
			assets.background = Some(image);
			continue;
		}
		if assets.banner.is_none()
			&& ((100..=320).contains(&width) && (50..=240).contains(&height)
				|| width > 200 && width as f32 / height as f32 > 2.0)
		{
			assets.banner = Some(image);
			continue;
		}
		if assets.cdtitle.is_none() && width <= 100 && height <= 48 {
			assets.cdtitle = Some(image);
			continue;
		}
		if assets.jacket.is_none() && width == height {
			assets.jacket = Some(image);
			continue;
		}
		if assets.disc.is_none() && width > height && assets.banner.is_some() {
			assets.disc = Some(image);
			continue;
		}
		if assets.cdimage.is_none() && width == height {
			assets.cdimage = Some(image);
		}
	}

	[
		("REALMUSIC", assets.music),
		("REALBANNER", assets.banner),
		("REALJACKET", assets.jacket),
		("REALBACKGROUND", assets.background),
		("REALCDTITLE", assets.cdtitle),
	]
	.into_iter()
	.filter_map(|(tag, file)| file.map(|file| (tag, file.name)))
	.collect()
}

/// ITGmania does not use images embedded in DWI `DISPLAYTITLE` or
/// `DISPLAYARTIST` markup for the song's own assets.  Its DWI loader gives
/// `TidyUpData` this exact lowercased filename set.
fn dwi_blacklisted_images(path: &Path, elements: &[MsdElement]) -> HashSet<String> {
	if !is_dwi(path) {
		return HashSet::new();
	}
	elements
		.iter()
		.filter(|element| {
			element.tag.as_ref().eq_ignore_ascii_case(b"DISPLAYTITLE")
				|| element.tag.as_ref().eq_ignore_ascii_case(b"DISPLAYARTIST")
		})
		.filter_map(|element| element.values.first())
		.flat_map(|value| {
			String::from_utf8_lossy(value)
				.split('{')
				.skip(1)
				.filter_map(|value| {
					value
						.split_once('}')
						.map(|(image, _)| image.to_ascii_lowercase())
				})
				.collect::<Vec<_>>()
		})
		.collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SongFile {
	name: String,
	path: PathBuf,
}

impl SongFile {
	fn stem(&self) -> String {
		self.path
			.file_stem()
			.unwrap_or_default()
			.to_string_lossy()
			.to_ascii_lowercase()
	}
}

#[derive(Default)]
struct DiscoveredAssets {
	music: Option<SongFile>,
	banner: Option<SongFile>,
	jacket: Option<SongFile>,
	background: Option<SongFile>,
	cdtitle: Option<SongFile>,
	cdimage: Option<SongFile>,
	disc: Option<SongFile>,
}

impl DiscoveredAssets {
	fn contains(&self, file: &SongFile) -> bool {
		[
			&self.banner,
			&self.background,
			&self.cdtitle,
			&self.jacket,
			&self.disc,
			&self.cdimage,
		]
		.into_iter()
		.flatten()
		.any(|candidate| candidate == file)
	}
}

fn song_files(dir: &Path) -> Vec<SongFile> {
	let mut files = fs::read_dir(dir)
		.into_iter()
		.flatten()
		.filter_map(Result::ok)
		.filter_map(|entry| {
			let path = entry.path();
			let name = entry.file_name().to_string_lossy().into_owned();
			(path.is_file() && !should_ignore_file(&name)).then_some(SongFile { name, path })
		})
		.collect::<Vec<_>>();
	// FilenameDB, which backs ITGmania's GetDirListing, orders by lowercase name.
	files.sort_by_key(|file| file.name.to_ascii_lowercase());
	files
}

fn explicit_asset(elements: &[MsdElement], tag: &str, dir: &Path) -> Option<SongFile> {
	let name = elements
		.iter()
		.find(|element| element.tag.as_ref() == tag.as_bytes())?
		.values
		.first()
		.map(|value| String::from_utf8_lossy(value).trim().to_owned())?;
	let path = dir.join(&name);
	if path
		.file_name()
		.and_then(|filename| filename.to_str())
		.is_some_and(should_ignore_file)
	{
		return None;
	}
	path.is_file().then_some(SongFile { name, path })
}

fn is_bitmap(file: &SongFile) -> bool {
	matches!(
		file.path
			.extension()
			.and_then(|extension| extension.to_str())
			.map(str::to_ascii_lowercase)
			.as_deref(),
		Some("bmp" | "gif" | "jpeg" | "jpg" | "png")
	)
}

fn is_stepmania_audio(file: &SongFile) -> bool {
	matches!(
		file.path
			.extension()
			.and_then(|extension| extension.to_str())
			.map(str::to_ascii_lowercase)
			.as_deref(),
		Some("mp3" | "oga" | "ogg" | "wav")
	)
}

fn first_filename_match(
	files: &[SongFile],
	starts_with: &[&str],
	contains: &[&str],
	ends_with: &[&str],
) -> Option<SongFile> {
	files.iter().find_map(|file| {
		let stem = file.stem();
		(starts_with.iter().any(|prefix| stem.starts_with(prefix))
			|| ends_with.iter().any(|suffix| stem.ends_with(suffix))
			|| contains.iter().any(|needle| stem.contains(needle)))
		.then(|| file.clone())
	})
}

fn is_ssc(path: &Path) -> bool {
	path.extension()
		.is_some_and(|extension| extension.eq_ignore_ascii_case("ssc"))
}

fn is_dwi(path: &Path) -> bool {
	path.extension()
		.is_some_and(|extension| extension.eq_ignore_ascii_case("dwi"))
}

fn is_first_chart_tag(path: &Path, tag: &[u8]) -> bool {
	if is_ssc(path) {
		return tag == b"NOTEDATA";
	}
	if is_dwi(path) {
		return matches!(tag, b"SINGLE" | b"DOUBLE" | b"COUPLE" | b"SOLO");
	}
	tag == b"NOTES"
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::{Path, PathBuf};

	fn write_png(path: &Path, width: u32, height: u32) {
		image::RgbaImage::new(width, height).save(path).unwrap();
	}

	#[test]
	fn enriches_and_replaces_generated_tags() {
		let path = PathBuf::from("Songs/Author (Charter)/song.sm");
		let bytes =
			b"#TITLE:Song;\n#MUSIC:music.ogg;\n#NOTES:dance-single:Author:Hard:10:0:0000;\n";
		let result = enrich_sm_bytes(&path, bytes).unwrap();
		let text = String::from_utf8(result.clone()).unwrap();
		assert_eq!(text.matches("#BKBENRICHED:").count(), 1);
		let result = enrich_sm_bytes(&path, &result).unwrap();
		let text = String::from_utf8(result).unwrap();
		assert_eq!(text.matches("#BKBENRICHED:").count(), 1);
	}

	#[test]
	fn preserves_existing_culture_tag() {
		let path = Path::new("song.sm");
		let bytes = b"#TITLE:Song;\n#CULTURE:pad;\n#NOTES:dance-single:Author:Hard:10:0:0000;\n";
		let result = enrich_sm_bytes(path, bytes).unwrap();
		let text = String::from_utf8(result).unwrap();
		assert!(text.contains("#CULTURE:pad;"));
	}

	#[test]
	fn enriches_ssc_song_header_before_notedata() {
		let path = Path::new("song.ssc");
		let bytes = b"#TITLE:Song;\n#NOTEDATA:;\n#STEPSTYPE:dance-single;\n#NOTES:0000;\n";
		let result = enrich_sm_bytes(path, bytes).unwrap();
		let text = String::from_utf8(result).unwrap();
		assert!(text.contains("#BKBENRICHED:v1;"));
		assert!(text.find("#BKBENRICHED:").unwrap() < text.find("#NOTEDATA:").unwrap());
	}

	#[test]
	fn enriches_dwi_before_the_first_chart_and_is_idempotent() {
		let path = Path::new("song.dwi");
		let bytes = b"#TITLE:Song;\n#FILE:music.ogg;\n#SINGLE:BASIC:3:44;\n";
		let result = enrich_sm_bytes(path, bytes).unwrap();
		let text = String::from_utf8(result.clone()).unwrap();
		assert!(text.find("#BKBENRICHED:").unwrap() < text.find("#SINGLE:").unwrap());
		let second = enrich_sm_bytes(path, &result).unwrap();
		assert_eq!(second, result);
	}

	#[test]
	fn uses_itgmania_filename_rules_then_image_dimensions() {
		let temp = tempfile::TempDir::new().unwrap();
		let path = temp.path().join("Stinger.sm");
		write_png(&temp.path().join("Stinger-BG.png"), 640, 480);
		write_png(&temp.path().join("Stinger-BN.png"), 240, 90);
		write_png(&temp.path().join("CDTitle.png"), 64, 32);

		let bytes =
			b"#TITLE:Stinger;\n#BANNER:missing.png;\n#NOTES:dance-single:Author:Hard:10:0:0000;\n";
		let text = String::from_utf8(enrich_sm_bytes(&path, bytes).unwrap()).unwrap();

		// `-BG` hits ITGmania's filename suffix rule. `-BN` does not: it is
		// classified by the same 100..=320 × 50..=240 dimension rule instead.
		assert!(text.contains("#REALBACKGROUND:Stinger-BG.png;"));
		assert!(text.contains("#REALBANNER:Stinger-BN.png;"));
		assert!(text.contains("#REALCDTITLE:CDTitle.png;"));
	}

	#[test]
	fn uses_dwi_file_tag_for_music() {
		let temp = tempfile::TempDir::new().unwrap();
		let path = temp.path().join("song.dwi");
		fs::write(temp.path().join("music.ogg"), []).unwrap();
		let bytes = b"#TITLE:Song;\n#FILE:music.ogg;\n#SINGLE:BASIC:3:44;\n";
		let text = String::from_utf8(enrich_sm_bytes(&path, bytes).unwrap()).unwrap();
		assert!(text.contains("#REALMUSIC:music.ogg;"));
	}

	#[test]
	fn ignores_appledouble_files_even_when_explicitly_referenced() {
		let temp = tempfile::TempDir::new().unwrap();
		let path = temp.path().join("song.sm");
		write_png(&temp.path().join("._banner.png"), 240, 90);
		let bytes =
			b"#TITLE:Song;\n#BANNER:._banner.png;\n#NOTES:dance-single:Author:Hard:10:0:0000;\n";
		let text = String::from_utf8(enrich_sm_bytes(&path, bytes).unwrap()).unwrap();
		assert!(!text.contains("#REALBANNER:"));
	}
}
