//! Parsing `.ssc` (StepMania SSC) files.
//!
//! SSC is StepMania's preferred simfile format, extending `.sm` with per-chart
//! metadata tags and optional per-chart (split) timing. It uses the same MSD
//! grammar — `#TAG:value;` — but wraps each chart in a `#NOTEDATA:;` block
//! instead of encoding chart metadata as colon-params of a `#NOTES` element.
//!
//! Types for notes and timing segments are re-exported from [`sm`][crate::sm].

use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::sm::{parse_bpms, parse_notedata};
use crate::sm_msd::{self, MsdElement, MsdFile};

pub use crate::sm::{Bpm, Difficulty, Event, Measure, NoteVariant, StepsType, Stop};

/// Song-level metadata from the header section of an SSC file (before any `#NOTEDATA:;`).
#[derive(Debug, Clone)]
pub struct SongData {
	/// All song-level MSD tags, including ones not promoted into typed fields.
	///
	/// Keys are stored as they appear after MSD normalisation (uppercase tags).
	pub tags: MsdFile,
	/// Title of the song.
	pub title: String,
	/// Subtitle, if present.
	pub subtitle: Option<String>,
	/// Artist name.
	pub artist: String,
	/// Value of `#MUSIC:`, if present and non-empty.
	pub music: Option<String>,
	/// BPM changes across the song.
	pub bpms: Vec<Bpm>,
	/// Stops across the song.
	pub stops: Vec<Stop>,
	/// Audio offset in seconds (sign-corrected: `-(#OFFSET)`).
	pub offset_secs: Option<f64>,
}

/// Per-chart data from a single `#NOTEDATA:;` block.
#[derive(Debug, Clone)]
pub struct SscChart {
	/// All MSD tags from this `#NOTEDATA:;` block, including the `#NOTEDATA`
	/// marker itself and any tags not promoted into typed fields.
	pub tags: MsdFile,
	/// Game mode / steps type (e.g. `dance-single`).
	pub steps_type: Option<StepsType>,
	/// Chart description / subtitle.
	pub description: Option<String>,
	/// Chart name tag (v≥0.74; falls back to `description` in older files).
	pub chart_name: Option<String>,
	/// Difficulty tier.
	pub difficulty: Option<Difficulty>,
	/// Numeric difficulty rating.
	pub meter: Option<u32>,
	/// Chart author / credit.
	pub credit: Option<String>,
	/// Per-chart BPMs (split timing). `None` when the chart inherits song timing.
	pub bpms: Option<Vec<Bpm>>,
	/// Per-chart stops (split timing). `None` when the chart inherits song timing.
	pub stops: Option<Vec<Stop>>,
	/// Per-chart audio offset (split timing). `None` when inheriting song timing.
	pub offset_secs: Option<f64>,
	/// Parsed measures from the `#NOTES:` body. `None` when no `#NOTES:` tag was present.
	pub measures: Option<Vec<Measure>>,
}

/// A fully parsed SSC file.
#[derive(Debug, Clone)]
pub struct SscFile {
	/// Song-level metadata.
	pub song: SongData,
	/// One entry per `#NOTEDATA:;` block found in the file.
	pub charts: Vec<SscChart>,
	/// Path this file was loaded from.
	pub path: PathBuf,
}

// ────────────────────────────────────────────────────────────────────────────
// Public entry points
// ────────────────────────────────────────────────────────────────────────────

/// Load and parse an SSC file from disk.
pub fn from_path(ssc_path: impl AsRef<Path>) -> io::Result<SscFile> {
	let path = ssc_path.as_ref();
	let bytes = fs::read(path)?;
	Ok(parse(&bytes, path))
}

/// Parse an SSC file from raw bytes.
///
/// `ssc_path` is used to resolve `#MUSIC` existence on disk; pass the actual
/// `.ssc` file path, or any placeholder when asset resolution is not needed.
pub fn from_bytes(bytes: &[u8], ssc_path: impl AsRef<Path>) -> SscFile {
	parse(bytes, ssc_path.as_ref())
}

// ────────────────────────────────────────────────────────────────────────────
// Internal parsing
// ────────────────────────────────────────────────────────────────────────────

fn parse(bytes: &[u8], ssc_path: &Path) -> SscFile {
	let msd = sm_msd::from_bytes(bytes);
	let elements = msd.elements;

	// Everything before the first NOTEDATA element is song-level.
	let split_pos = elements
		.iter()
		.position(|el| &*el.tag == b"NOTEDATA")
		.unwrap_or(elements.len());

	let (song_els, chart_stream) = elements.split_at(split_pos);

	let song = parse_song(song_els, ssc_path);

	// Group chart_stream into slices delimited by NOTEDATA boundaries.
	let mut chart_groups: Vec<Vec<&MsdElement>> = vec![];
	let mut current: Vec<&MsdElement> = vec![];
	for el in chart_stream {
		if &*el.tag == b"NOTEDATA" && !current.is_empty() {
			chart_groups.push(std::mem::take(&mut current));
		}
		current.push(el);
	}
	if !current.is_empty() {
		chart_groups.push(current);
	}

	let charts = chart_groups.iter().map(|g| parse_chart(g)).collect();

	SscFile {
		song,
		charts,
		path: ssc_path.to_path_buf(),
	}
}

/// Return the first value of the element whose tag matches `tag` (uppercase).
fn get_tag<'a>(elements: &'a [MsdElement], tag: &[u8]) -> Option<&'a [u8]> {
	elements
		.iter()
		.find(|el| &*el.tag == tag)
		.and_then(|el| el.values.first())
		.map(|v| v.as_ref())
}

/// Same as [`get_tag`] but over a slice of element references.
fn get_tag_ref<'a>(elements: &[&'a MsdElement], tag: &[u8]) -> Option<&'a [u8]> {
	elements
		.iter()
		.find(|el| &*el.tag == tag)
		.and_then(|el| el.values.first())
		.map(|v| v.as_ref())
}

fn parse_song(elements: &[MsdElement], ssc_path: &Path) -> SongData {
	let title = get_tag(elements, b"TITLE")
		.map(|v| String::from_utf8_lossy(v).into_owned())
		.unwrap_or_else(|| {
			ssc_path
				.parent()
				.and_then(|p| p.file_name())
				.map(|n| n.to_string_lossy().into_owned())
				.unwrap_or_else(|| "Untitled Song".to_owned())
		});

	let subtitle = get_tag(elements, b"SUBTITLE")
		.map(|v| String::from_utf8_lossy(v).trim().to_owned())
		.filter(|s| !s.is_empty());

	let artist = get_tag(elements, b"ARTIST")
		.map(|v| String::from_utf8_lossy(v).into_owned())
		.unwrap_or_else(|| "Unknown Artist".to_owned());

	let music = get_tag(elements, b"MUSIC")
		.map(|v| String::from_utf8_lossy(v).trim().to_owned())
		.filter(|s| !s.is_empty());

	let bpms = get_tag(elements, b"BPMS")
		.and_then(|v| parse_bpms(v).ok())
		.unwrap_or_else(|| {
			vec![Bpm {
				bpm: 60.0,
				offset_beats: 0.0,
			}]
		});

	let stops = get_tag(elements, b"STOPS")
		.and_then(|v| parse_bpms(v).ok())
		.unwrap_or_default()
		.into_iter()
		.map(|b| Stop {
			duration: b.bpm,
			offset_beats: b.offset_beats,
		})
		.collect();

	let offset_secs = parse_offset(get_tag(elements, b"OFFSET"));

	SongData {
		tags: MsdFile {
			elements: elements.to_vec(),
		},
		title,
		subtitle,
		artist,
		music,
		bpms,
		stops,
		offset_secs,
	}
}

fn parse_chart(elements: &[&MsdElement]) -> SscChart {
	let steps_type = get_tag_ref(elements, b"STEPSTYPE").map(StepsType::from_bytes);

	let description = get_tag_ref(elements, b"DESCRIPTION")
		.map(|v| String::from_utf8_lossy(v).trim().to_owned())
		.filter(|s| !s.is_empty());

	let chart_name = get_tag_ref(elements, b"CHARTNAME")
		.map(|v| String::from_utf8_lossy(v).trim().to_owned())
		.filter(|s| !s.is_empty());

	let credit = get_tag_ref(elements, b"CREDIT")
		.map(|v| String::from_utf8_lossy(v).trim().to_owned())
		.filter(|s| !s.is_empty());

	let meter = get_tag_ref(elements, b"METER")
		.and_then(|v| std::str::from_utf8(v).ok())
		.and_then(|s| s.trim().parse().ok());

	// For Edit difficulty, the description serves as the disambiguator (analogous
	// to how SM uses the author param).
	let difficulty = get_tag_ref(elements, b"DIFFICULTY").and_then(|diff| {
		let edit_desc: Box<[u8]> = description.as_deref().unwrap_or("").as_bytes().into();
		parse_difficulty(diff, &edit_desc)
	});

	// Split timing tags — present only when the chart overrides song timing.
	let bpms = get_tag_ref(elements, b"BPMS").and_then(|v| parse_bpms(v).ok());

	let stops = get_tag_ref(elements, b"STOPS")
		.and_then(|v| parse_bpms(v).ok())
		.map(|bs| {
			bs.into_iter()
				.map(|b| Stop {
					duration: b.bpm,
					offset_beats: b.offset_beats,
				})
				.collect::<Vec<_>>()
		});

	let offset_secs = parse_offset(get_tag_ref(elements, b"OFFSET"));

	// Note data from #NOTES: or #NOTES2: (keysound-annotated variant).
	let measures = get_tag_ref(elements, b"NOTES")
		.or_else(|| get_tag_ref(elements, b"NOTES2"))
		.map(parse_notedata);

	SscChart {
		tags: MsdFile {
			elements: elements.iter().map(|el| (*el).clone()).collect(),
		},
		steps_type,
		description,
		chart_name,
		difficulty,
		meter,
		credit,
		bpms,
		stops,
		offset_secs,
		measures,
	}
}

/// Parse `#OFFSET:` bytes into a sign-corrected seconds value.
///
/// SM stores offset as `-(actual_offset)`, so we negate here. Returns `None`
/// for a zero offset (irrelevant) or an unparseable value.
fn parse_offset(raw: Option<&[u8]>) -> Option<f64> {
	let f: f64 = lexical::parse_partial(std::str::from_utf8(raw?).ok()?)
		.ok()
		.map(|(v, _)| v)?;
	if f == 0.0 { None } else { Some(-f) }
}

/// Parse a `#DIFFICULTY:` tag value into a [`Difficulty`].
///
/// `edit_desc` is used as the [`Difficulty::Edit`] disambiguator when the
/// difficulty string is `"edit"`.
fn parse_difficulty(diff: &[u8], edit_desc: &[u8]) -> Option<Difficulty> {
	match diff.trim_ascii().to_ascii_lowercase().as_slice() {
		b"beginner" => Some(Difficulty::Beginner),
		b"easy" | b"basic" | b"light" => Some(Difficulty::Easy),
		b"medium" | b"another" | b"trick" | b"standard" | b"difficult" => Some(Difficulty::Medium),
		b"hard" | b"ssr" | b"maniac" | b"heavy" => Some(Difficulty::Hard),
		b"challenge" | b"expert" | b"oni" => Some(Difficulty::Challenge),
		b"edit" => Some(Difficulty::Edit(edit_desc.into())),
		_ => None,
	}
}

// ────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
	use super::*;

	fn make_ssc(charts: &[(&str, &str, &str, u32, &str)]) -> Vec<u8> {
		let mut s = String::from(
			"#TITLE:Test Song;\n\
			 #ARTIST:Test Artist;\n\
			 #MUSIC:song.ogg;\n\
			 #BPMS:0.000=120.000;\n",
		);
		for (steps_type, desc, diff, meter, notes) in charts {
			s.push_str(&format!(
				"#NOTEDATA:;\n\
				 #STEPSTYPE:{steps_type};\n\
				 #DESCRIPTION:{desc};\n\
				 #DIFFICULTY:{diff};\n\
				 #METER:{meter};\n\
				 #NOTES:\n{notes}\n;\n"
			));
		}
		s.into_bytes()
	}

	#[test]
	fn parse_no_charts() {
		let bytes = b"#TITLE:Empty;\n#ARTIST:Nobody;\n";
		let ssc = from_bytes(bytes, ".");
		assert_eq!(ssc.song.title, "Empty");
		assert!(ssc.charts.is_empty());
	}

	#[test]
	fn parse_song_metadata() {
		let bytes = make_ssc(&[]);
		let ssc = from_bytes(&bytes, ".");
		assert_eq!(ssc.song.title, "Test Song");
		assert_eq!(ssc.song.artist, "Test Artist");
		assert_eq!(ssc.song.bpms.len(), 1);
		assert_eq!(ssc.song.bpms[0].bpm, 120.0);
	}

	#[test]
	fn parse_chart_count() {
		let bytes = make_ssc(&[
			("dance-single", "Basic", "easy", 3, "0000\n0000\n0000\n0000"),
			("dance-single", "Hard", "hard", 8, "1000\n0100\n0010\n0001"),
		]);
		let ssc = from_bytes(&bytes, ".");
		assert_eq!(ssc.charts.len(), 2);
	}

	#[test]
	fn parse_chart_metadata() {
		let bytes = make_ssc(&[("dance-double", "MyChart", "challenge", 15, "0000\n")]);
		let ssc = from_bytes(&bytes, ".");
		let chart = &ssc.charts[0];
		assert!(matches!(chart.steps_type, Some(StepsType::DanceDouble)));
		assert_eq!(chart.description.as_deref(), Some("MyChart"));
		assert!(matches!(chart.difficulty, Some(Difficulty::Challenge)));
		assert_eq!(chart.meter, Some(15));
	}

	#[test]
	fn parse_chart_measures() {
		let bytes = make_ssc(&[("dance-single", "Test", "hard", 10, "1000\n0100\n0010\n0001")]);
		let ssc = from_bytes(&bytes, ".");
		let chart = &ssc.charts[0];
		let measures = chart.measures.as_ref().expect("should have measures");
		assert_eq!(measures.len(), 1);
		assert_eq!(measures[0].events.len(), 4);
	}

	#[test]
	fn split_timing_absent_by_default() {
		let bytes = make_ssc(&[("dance-single", "Test", "medium", 5, "0000\n")]);
		let ssc = from_bytes(&bytes, ".");
		let chart = &ssc.charts[0];
		assert!(chart.bpms.is_none());
		assert!(chart.stops.is_none());
		assert!(chart.offset_secs.is_none());
	}

	#[test]
	fn split_timing_present_when_specified() {
		let bytes = b"#TITLE:T;\n#ARTIST:A;\n#BPMS:0=120;\n\
		              #NOTEDATA:;\n#STEPSTYPE:dance-single;\n#DIFFICULTY:easy;\n\
		              #METER:3;\n#BPMS:0=180;\n#NOTES:\n0000\n;\n";
		let ssc = from_bytes(bytes, ".");
		let chart = &ssc.charts[0];
		let bpms = chart.bpms.as_ref().expect("split timing bpms");
		assert_eq!(bpms[0].bpm, 180.0);
	}

	#[test]
	fn preserves_raw_song_and_chart_tags() {
		let bytes = b"#TITLE:T;\n#ARTIST:A;\n#GENRE:Trance;\n#BANNER:bn.png;\n#BPMS:0=120;\n\
		              #NOTEDATA:;\n#STEPSTYPE:dance-single;\n#DIFFICULTY:easy;\n\
		              #METER:3;\n#CREDIT:chartist;\n#RADARVALUES:0,0,0,0,0;\n\
		              #NOTES:\n0000\n;\n";
		let ssc = from_bytes(bytes, ".");

		assert_eq!(
			ssc.song.tags.get_first_value("GENRE").as_deref(),
			Some(b"Trance".as_slice())
		);
		assert_eq!(
			ssc.song.tags.get_first_value("BANNER").as_deref(),
			Some(b"bn.png".as_slice())
		);

		let chart = &ssc.charts[0];
		assert!(chart.tags.get_element("NOTEDATA").is_some());
		assert_eq!(
			chart.tags.get_first_value("CREDIT").as_deref(),
			Some(b"chartist".as_slice())
		);
		assert_eq!(
			chart.tags.get_first_value("RADARVALUES").as_deref(),
			Some(b"0,0,0,0,0".as_slice())
		);
	}
}
