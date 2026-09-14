//! Parsing, Processing and serializing of `.sm` files.
//!
//! This module handles parsing of `.sm` files. Parsing SSC is not yet supported.
//! For parsing the raw `.msd` format that underpins these formats, see [`sm_msd`].

use std::borrow::Cow;
use std::ffi::{OsStr, OsString};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;
use std::{fs, io};

use thiserror::Error;

use crate::sm_msd::{self, MsdElement, MsdFile};
use crate::utils::ByteString;

/// Parsing an SM file can fail in a couple of ways. Mainly nonsensical/invalid data.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum LoadError {
	/// This simply wasn't valid SM. This could be anything like having a bpm of `-1`,
	/// having a bpm that wasn't a parsable number, like the empty string,
	/// or not having enough fields in #NOTES.
	#[error("this was not valid sm ({0})")]
	InvalidSM(String),

	/// We expected valid UTF-8 at some point in the process, but got something else.
	/// This can happen if non-utf8 is placed into the BPM or OFFSET fields.
	#[error("expected {0} to parse as utf-8, got Utf8Error {1}")]
	UnexpectedNonUtf8(String, Utf8Error),
}

/// A bpm change in the SM format.
#[derive(Debug, Clone, PartialEq)]
pub struct Bpm {
	/// The BPM this chart is as a float.
	pub bpm: f64,
	/// When this BPM occurs. The first BPM **must** occur at 0.0.
	pub offset_beats: f64,
}

impl Bpm {
	fn from_bytes(bytes: &[u8]) -> Result<Self, LoadError> {
		let str = match std::str::from_utf8(bytes) {
			Ok(str) => str,
			Err(utf8err) => return Err(LoadError::UnexpectedNonUtf8("BPM".into(), utf8err)),
		};

		let elements = str.split('=').collect::<Vec<&str>>();

		if elements.len() < 2 {
			return Err(LoadError::InvalidSM(format!(
				"Invalid SM BPM string '{str}'."
			)));
		}

		let (offset, bpm) = (elements[0], elements[1]);

		let (offset_beats, _) = match lexical::parse_partial(offset) {
			Ok(v) => v,
			Err(_) => {
				return Err(LoadError::InvalidSM(format!(
					"Failed to parse {offset} as a float."
				)));
			}
		};

		let (bpm, _) = match lexical::parse_partial(bpm) {
			Ok(v) => v,
			Err(_) => {
				return Err(LoadError::InvalidSM(format!(
					"Failed to parse {bpm} as a float."
				)));
			}
		};

		if bpm <= 0.0 {
			return Err(LoadError::InvalidSM(format!("BPM was negative ({bpm})")));
		}

		Ok(Bpm { bpm, offset_beats })
	}
}

/// A stop in the SM format.
#[derive(Debug, Clone, PartialEq)]
pub struct Stop {
	/// For how many seconds should we stop?
	pub duration: f64,
	/// When this BPM occurs. The first BPM **must** occur at 0.0.
	pub offset_beats: f64,
}

/// Various parsed metadata and tags from the SM file.
#[derive(Debug, Clone)]
pub struct SongMetadata {
	/// The offset (in seconds) that this chart has. This is a parsed variant of the
	/// `#OFFSET` tag, and is multiplied by -1.0 versus the actual stored format.
	pub offset_secs: Option<f64>,

	/// All the bpm changes in this chart.
	pub bpms: Vec<Bpm>,
	/// All the stops in this chart.
	pub stops: Vec<Stop>,

	/// The subtitle for this chart, if it had one. This is converted into utf8.
	pub subtitle: Option<String>,
	/// The artist for this chart. This is parsed as utf8 lossily.
	pub artist: String,
	/// The title for this chart. This is parsed as utf8 lossily.
	pub title: String,
	/// The path to the audio file for this chart. This file may or may not exist, and you
	/// should do audio resolution to find the real path.
	pub music: Option<OsString>,
}

/// A complete SM chart, with song metadata and the chart data.
#[derive(Debug, Clone)]
pub struct Chart {
	/// all metadata on this chart. **All keys are UPPERCASED!**.
	pub tags: MsdFile,

	/// Info about the song this chart belonged to.
	pub song_info: SongMetadata,
	/// Info about the actual chart itself.
	pub chart_data: ChartData,
	/// Where this chart was loaded from on disk.
	pub path: PathBuf,
}

macro_rules! msd_tag_fallback {
	($metadata:expr, $tag:expr, $fallback:expr) => {{
		match $metadata.get_first_value($tag) {
			Some(slice) => String::from_utf8_lossy(&slice).into_owned(),
			None => $fallback.to_owned(),
		}
	}};
}

macro_rules! msd_tag {
	($metadata:expr, $tag:expr) => {{
		match $metadata.get_first_value($tag) {
			Some(slice) => Some(String::from_utf8_lossy(&slice).into_owned()),
			None => None,
		}
	}};
}

impl Chart {
	/// Try and infer the author of this chart from the file path.
	///
	/// More specifically, this infers from the *directory* name that contains this
	/// SM file, and not the name of the SM file itself.
	///
	/// As such, the provided file path should look like:
	///
	/// ```txt
	/// "Hello (Kommisar)/chart.sm"
	/// ```
	///
	/// Authors are usually placed in the folder name in one of four forms:
	/// Chart Name (Charter)
	/// Chart Name [Charter]
	/// (Charter) Chart Name
	/// [Charter] Chart Name
	///
	/// The latter three are *very* old and non-standard, but we support them
	/// for compatibility.
	///
	/// Sometimes, this function fails to correctly infer this information
	/// as some songs have brackets in them and the charter isn't mentioned in there
	/// This can happen if the folder name is something like:
	///
	/// Song Title (Speed Up Ver.)
	///
	/// However, it's rare for those charts to not have an author attached.
	fn infer_author(path: impl AsRef<Path>) -> Option<String> {
		let path = path.as_ref();

		let path: Cow<Path> = match path.parent() {
			Some(v) => Cow::Borrowed(v),
			None => {
				// try and canonicalise file
				let canon = path.canonicalize().ok()?;

				{
					let v = canon.parent()?;
					Cow::Owned(v.to_owned())
				}
			}
		};

		let name = path.file_name()?;
		let name = OsStr::to_string_lossy(name);

		use regex::Regex;

		let standard = Regex::new(r"\(([^)]+)\) *$").unwrap();
		let square = Regex::new(r"\[([^\]]+)\] *$").unwrap();
		let std_start = Regex::new(r"^ *\(([^)]+)\)").unwrap();
		let sqr_start = Regex::new(r"^ *\[([^\]]+)\]").unwrap();

		if let Some(m) = standard.captures(&name) {
			return m.get(1).map(|f| f.as_str().trim().to_owned());
		}

		if let Some(m) = square.captures(&name) {
			return m.get(1).map(|f| f.as_str().trim().to_owned());
		}

		if let Some(m) = std_start.captures(&name) {
			return m.get(1).map(|f| f.as_str().trim().to_owned());
		}

		if let Some(m) = sqr_start.captures(&name) {
			return m.get(1).map(|f| f.as_str().trim().to_owned());
		}

		None
	}
}

/// Load an SM file from a path.
///
/// This returns an error if the file was unable to be read.
///
/// This returns an inner error if the file was able to be read, but the contents
/// were not valid SM.
///
/// In the event that one chart fails to parse, all charts fail to parse.
pub fn from_path(sm_path: impl AsRef<Path>) -> io::Result<Result<Vec<Chart>, LoadError>> {
	let bytes = fs::read(&sm_path)?;

	Ok(from_bytes(&bytes, sm_path))
}

/// Load an SM file from bytes.
///
/// This method is kind of useless, as to parse the sm file we **need** to know its
/// location on disk (for resolving #MUSIC and other information).
///
/// You probably want [`from_path`]. This method is only publically exposed for testing
/// reasons, where you might want the byte content and the path to be disjoint.
///
/// In the event that one chart fails to parse, all charts fail to parse.
pub fn from_bytes(bytes: &[u8], sm_path: impl AsRef<Path>) -> Result<Vec<Chart>, LoadError> {
	let (metadata, charts) = parse(bytes);

	let bpms: Vec<Bpm> = match metadata.get_first_value("BPMS") {
		Some(bpm_str) => parse_bpms(&bpm_str).unwrap_or(vec![Bpm {
			bpm: 60.0,
			offset_beats: 0.0,
		}]),
		None => vec![Bpm {
			bpm: 60.0,
			offset_beats: 0.0,
		}],
	};

	// BPMs and stops use the exact same syntax. Lets leverage the same parser.
	let stops: Vec<Stop> = metadata
		.get_first_value("STOPS")
		.and_then(|f| parse_bpms(&f).ok())
		.unwrap_or_default()
		.into_iter()
		.map(|f| Stop {
			duration: f.bpm,
			offset_beats: f.offset_beats,
		})
		.collect();

	let offset = match metadata.get_first_value("OFFSET") {
		Some(v) => {
			let str = match std::str::from_utf8(&v) {
				Ok(s) => s,
				Err(err) => return Err(LoadError::UnexpectedNonUtf8("OFFSET".into(), err)),
			};

			match lexical::parse_partial::<f64, &str>(str) {
				Ok((float, _)) => {
					// an offset of 0 is irrelevant
					// n.b. you can't have floats in match arms (lol)
					if float == 0.0 {
						None
					} else {
						// i have literally no idea why SM stores offset * -1
						// the sm codebase *also* immediately multiplies by -1 so
						// who knows, lol
						Some(-float)
					}
				}

				// an invalid offset is treated as 0 offset.
				Err(_) => None,
			}
		}
		None => None,
	};

	let music_path = metadata
		.get_first_value("MUSIC")
		.map(|bytes| crate::utils::os_string_from_bytes(&bytes));

	let mut music = None;

	if music_path
		.as_ref()
		.is_some_and(|path| match sm_path.as_ref().parent() {
			Some(v) => v.join(path).exists(),
			None => Path::new(path).exists(),
		}) {
		music = music_path;
	}

	let song_info = SongMetadata {
		artist: msd_tag_fallback!(metadata, "ARTIST", "Unknown Artist"),
		title: match metadata.get_first_value("TITLE") {
			Some(v) => String::from_utf8_lossy(&v).into_owned(),
			None => {
				// gotta infer the song title from the file path, since it wasn't
				// specified in the file.

				let mut path = sm_path.as_ref().to_path_buf();

				path.pop();

				match path.file_name() {
					Some(name) => name.to_string_lossy().into_owned(),
					None => "Untitled Song".to_owned(),
				}
			}
		},
		subtitle: msd_tag!(metadata, "SUBTITLE"),
		bpms,
		stops,
		music,
		offset_secs: offset,
	};

	let mut ok_charts = vec![];

	for chart in charts {
		// if any chart failed to parse, bail out.
		let mut chart = chart?;

		// even if we know who made this chart, we should ignore
		// anything that says "Blank" or "Copied From", as
		// that basically also means unknown.
		if (chart.author.is_empty()
			|| &*chart.author.to_ascii_lowercase() == b"copied from"
			|| &*chart.author.to_ascii_lowercase() == b"blank")
			&& let Some(inferred_name) = Chart::infer_author(&sm_path)
		{
			chart.author = inferred_name.as_bytes().into();
		}

		let full_file = Chart {
			tags: metadata.clone(),
			song_info: song_info.clone(),
			chart_data: chart,
			path: sm_path.as_ref().to_path_buf(),
		};

		ok_charts.push(full_file);
	}

	Ok(ok_charts)
}

/// This is all the possible modes SM supports as of 2023/07/27.
///
/// There are still potentially more than this, for those cases they fall into "Unknown".
///
/// Note that most of these modes are effectively useless, and have never been played or
/// even tested by anyone. I'm not even honestly sure why I bothered writing them all out,
/// but it's done now.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum StepsType {
	DanceSingle,
	DanceDouble,
	DanceCouple,
	DanceSolo,
	DanceThreepanel,
	DanceRoutine,
	PumpSingle,
	PumpHalfDouble,
	PumpDouble,
	PumpCouple,
	PumpRoutine,
	Kb7Single,
	Ez2Single,
	Ez2Double,
	Ez2Real,
	ParaSingle,
	Ds3ddxSingle,
	BmSingle5,
	BmVersus5,
	BmDouble5,
	BmSingle7,
	BmVersus7,
	BmDouble7,
	ManiaxSingle,
	ManiaxDouble,
	TechnoSingle4,
	TechnoSingle5,
	TechnoSingle8,
	TechnoDouble4,
	TechnoDouble5,
	TechnoDouble8,
	PnmFive,
	PnmNine,
	LightsCabinet,
	KickboxHuman,
	KickboxQuadarm,
	KickboxInsect,
	KickboxArachnid,

	/// Some unknown gamemode.
	Other(ByteString),
}

impl Display for StepsType {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let str = match self {
			StepsType::DanceSingle => "dance-single".into(),
			StepsType::DanceDouble => "dance-double".into(),
			StepsType::DanceCouple => "dance-couple".into(),
			StepsType::DanceSolo => "dance-solo".into(),
			StepsType::DanceThreepanel => "dance-threepanel".into(),
			StepsType::DanceRoutine => "dance-routine".into(),
			StepsType::PumpSingle => "pump-single".into(),
			StepsType::PumpHalfDouble => "pump-halfdouble".into(),
			StepsType::PumpDouble => "pump-double".into(),
			StepsType::PumpCouple => "pump-couple".into(),
			StepsType::PumpRoutine => "pump-routine".into(),
			StepsType::Kb7Single => "kb7-single".into(),
			StepsType::Ez2Single => "ez2-single".into(),
			StepsType::Ez2Double => "ez2-double".into(),
			StepsType::Ez2Real => "ez2-real".into(),
			StepsType::ParaSingle => "para-single".into(),
			StepsType::Ds3ddxSingle => "ds3ddx-single".into(),
			StepsType::BmSingle5 => "bm-single5".into(),
			StepsType::BmVersus5 => "bm-versus5".into(),
			StepsType::BmDouble5 => "bm-double5".into(),
			StepsType::BmSingle7 => "bm-single7".into(),
			StepsType::BmVersus7 => "bm-versus7".into(),
			StepsType::BmDouble7 => "bm-double7".into(),
			StepsType::ManiaxSingle => "maniax-single".into(),
			StepsType::ManiaxDouble => "maniax-double".into(),
			StepsType::TechnoSingle4 => "techno-single4".into(),
			StepsType::TechnoSingle5 => "techno-single5".into(),
			StepsType::TechnoSingle8 => "techno-single8".into(),
			StepsType::TechnoDouble4 => "techno-double4".into(),
			StepsType::TechnoDouble5 => "techno-double5".into(),
			StepsType::TechnoDouble8 => "techno-double8".into(),
			StepsType::PnmFive => "pnm-five".into(),
			StepsType::PnmNine => "pnm-nine".into(),
			StepsType::LightsCabinet => "lights-cabinet".into(),
			StepsType::KickboxHuman => "kickbox-human".into(),
			StepsType::KickboxQuadarm => "kickbox-quadarm".into(),
			StepsType::KickboxInsect => "kickbox-insect".into(),
			StepsType::KickboxArachnid => "kickbox-arachnid".into(),
			StepsType::Other(a) => String::from_utf8_lossy(a).into_owned(),
		};

		f.write_str(&str)
	}
}

impl StepsType {
	/// Parse a StepsType from its simfile tag bytes (e.g. `b"dance-single"`).
	pub fn from_bytes(bytes: &[u8]) -> Self {
		match bytes {
			b"dance-single" => StepsType::DanceSingle,
			b"dance-double" => StepsType::DanceDouble,
			b"dance-couple" => StepsType::DanceCouple,
			b"dance-solo" => StepsType::DanceSolo,
			b"dance-threepanel" => StepsType::DanceThreepanel,
			b"dance-routine" => StepsType::DanceRoutine,
			b"pump-single" => StepsType::PumpSingle,
			b"pump-halfdouble" => StepsType::PumpHalfDouble,
			b"pump-double" => StepsType::PumpDouble,
			b"pump-couple" => StepsType::PumpCouple,
			b"pump-routine" => StepsType::PumpRoutine,
			b"kb7-single" => StepsType::Kb7Single,
			b"ez2-single" => StepsType::Ez2Single,
			b"ez2-double" => StepsType::Ez2Double,
			b"ez2-real" => StepsType::Ez2Real,
			b"para-single" => StepsType::ParaSingle,
			b"ds3ddx-single" => StepsType::Ds3ddxSingle,
			b"bm-single5" => StepsType::BmSingle5,
			b"bm-versus5" => StepsType::BmVersus5,
			b"bm-double5" => StepsType::BmDouble5,
			b"bm-single7" => StepsType::BmSingle7,
			b"bm-versus7" => StepsType::BmVersus7,
			b"bm-double7" => StepsType::BmDouble7,
			b"maniax-single" => StepsType::ManiaxSingle,
			b"maniax-double" => StepsType::ManiaxDouble,
			b"techno-single4" => StepsType::TechnoSingle4,
			b"techno-single5" => StepsType::TechnoSingle5,
			b"techno-single8" => StepsType::TechnoSingle8,
			b"techno-double4" => StepsType::TechnoDouble4,
			b"techno-double5" => StepsType::TechnoDouble5,
			b"techno-double8" => StepsType::TechnoDouble8,
			b"pnm-five" => StepsType::PnmFive,
			b"pnm-nine" => StepsType::PnmNine,
			b"lights-cabinet" => StepsType::LightsCabinet,
			b"kickbox-human" => StepsType::KickboxHuman,
			b"kickbox-quadarm" => StepsType::KickboxQuadarm,
			b"kickbox-insect" => StepsType::KickboxInsect,
			b"kickbox-arachnid" => StepsType::KickboxArachnid,
			_ => StepsType::Other(bytes.into()),
		}
	}
}

/// There are 5 possible difficulties for an SM chart, which correspond to multiple
/// possible names in the format.
///
/// There is also a 6th overflow difficulty, called "Edit". This takes one argument
/// which disambiguates further, as multiple edits are legal for the same song.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Difficulty {
	/// This is a beginner chart.
	Beginner,
	/// This is an easy, basic or light chart.
	Easy,
	/// This is a medium, another, trick, standard or difficult chart.
	Medium,
	/// This is a hard, ssr, maniac or heavy chart.
	Hard,
	/// This is a challenge, expert or oni chart.
	Challenge,
	/// This is an edit chart. The `Author` information becomes part of the difficulty
	/// name for disambiguation between multiple Edit charts.
	Edit(ByteString),
}

impl Display for Difficulty {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use Difficulty::*;

		let str = match self {
			Beginner => "Beginner".into(),
			Easy => "Easy".into(),
			Medium => "Medium".into(),
			Hard => "Hard".into(),
			Challenge => "Challenge".into(),
			Edit(txt) => format!("Edit {}", String::from_utf8_lossy(txt)),
		};

		write!(f, "{str}")
	}
}

/// Actual chart/notes data for an SM file. This has no song metadata attached onto it.
/// For a convenient combination of [`ChartData`] and [`SongMetadata`], see [`Chart`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartData {
	/// What [`StepsType`] this notedata says it is.
	pub steps_type: StepsType,
	/// Who made this chart. If this isn't present in the file, or is set to "Copied From"
	/// or "Blank", this is inferred from the name of the folder.
	pub author: ByteString,
	/// What difficulty this chart is.
	pub difficulty: Difficulty,

	/// What level this chart is. Negative numbers and 0 are converted into `1`.
	pub level: usize,

	/// The actual note data in parsed form. It's rare, but if you *really* need to access
	/// the raw note data, you can interact with the unparsed MSD file on [`Chart::tags`].
	pub notedata: Vec<Measure>,
}

/// An event in SM is one of the following variants.
#[derive(Debug, PartialEq, Clone, Eq)]
pub enum NoteVariant {
	/// A note was here. This corresponds to "1" in the SM file.
	Note,
	/// A hold started here. This corresponds to "2" in the SM file.
	HoldStart,
	/// A roll started here. This corresponds to "4" in the SM file.
	RollStart,
	/// A hold or roll was terminated here. This corresponds to "3" in the SM file.
	HoldOrRollEnd,

	/// A keysound should trigger here. This corresponds to "K" in the SM file.
	AutoKeysound,
	/// A lift was here. This corresponds to "L" in the SM file.
	Lift,
	/// A fake was here. This corresponds to "F" in the SM file.
	Fake,
	/// A mine was here. This corresponds to "M" in the SM file.
	Mine,

	/// An unknown note type was here -- any char that doesn't match one of the other
	/// kinds.
	Unknown(u8),
}

/// An event (note, hold start, mine, etc.) that happened in an SM chart's notedata.
/// This tells you what row it occured on, what column it occured on, and what kind
/// of note it was.
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Event {
	/// What row (into the measure) this event occured on. How far this is into the chart
	/// is relative to the containing [`Measure`]s `size` property.
	pub row: usize,

	/// What column this note occured on. This is indexed from 0.
	///
	/// NOTE: there is absolutely no guarantee how many columns can appear in a chart.
	/// A chart is totally within its right to change how many columns it has at any time,
	/// for any reason. SM simply discards columns it doesn't care for. You should likely
	/// do the same.
	pub column: usize,

	/// What kind of event this was.
	pub variant: NoteVariant,
}

/// A measure in an SM file. This is a collection of `size` rows, with `events` happening
/// on a given column with a given type.
#[derive(Debug, PartialEq, Clone, Eq)]
pub struct Measure {
	/// How many rows were in this measure.
	pub size: usize,
	/// What events are in this measure. See [`Event`] for more information.
	pub events: Vec<Event>,
}

pub(crate) fn parse_notedata(raw_notedata: &[u8]) -> Vec<Measure> {
	let mut measures = vec![];

	for measure in raw_notedata.split(|char| *char == b',') {
		let mut size = 0;
		let mut events = vec![];

		for row in measure.split(|char| *char == b'\n') {
			let row = row.trim_ascii();

			if row.is_empty() {
				continue;
			}

			// we can't use enumerate because of stupid SM functionality
			// where you can strap keysounds onto column events.
			let mut skip_until_closebrck = false;
			let mut index = 0;

			for ch in row.iter() {
				// Stepmania supports annotating events with keysounds
				// but basically nobody has ever used this feature and it is deep
				// in the guts of the SM codebase.
				//
				// the syntax for this is `1[0]001`, which corresponds to `1001`, but
				//                                                         ^
				//          this guy has the 0th keysound associated with it.
				//
				// as such, if we see [ we skip until ]. simple.
				if *ch == b'[' {
					skip_until_closebrck = true;
					continue;
				}

				if *ch == b']' {
					skip_until_closebrck = false;
					continue;
				}

				if skip_until_closebrck {
					continue;
				}

				let variant = match ch {
					// skip all non-data.
					b'0' => {
						index += 1;
						continue;
					}
					b'1' => NoteVariant::Note,
					b'2' => NoteVariant::HoldStart,
					b'3' => NoteVariant::HoldOrRollEnd,
					b'4' => NoteVariant::RollStart,
					b'M' => NoteVariant::Mine,
					b'K' => NoteVariant::AutoKeysound,
					b'L' => NoteVariant::Lift,
					b'F' => NoteVariant::Fake,
					ch => NoteVariant::Unknown(*ch),
				};

				events.push(Event {
					row: size,
					column: index,
					variant,
				});

				index += 1;
			}

			size += 1;
		}

		if size == 0 {
			// entire measure is empty?
			continue;
		}

		measures.push(Measure { size, events })
	}

	measures
}

/// Actually parse all the `#NOTES` tags in this SM file. Since technically all of these
/// can fail independently, this returns a vector of results.
fn parse(sm: &[u8]) -> (MsdFile, Vec<Result<ChartData, LoadError>>) {
	let msd_file = sm_msd::from_bytes(sm);

	let charts = msd_file
		.all_with_tag("NOTES")
		.iter()
		.map(|el| parse_notes(el))
		.collect();

	(msd_file, charts)
}

pub(crate) fn parse_bpms(bpms: &[u8]) -> Result<Vec<Bpm>, LoadError> {
	let mut bpm_vec = vec![];

	for bpm in bpms.split(|by| *by == b',') {
		{
			let v = Bpm::from_bytes(bpm.trim_ascii())?;
			bpm_vec.push(v)
		}
	}

	Ok(bpm_vec)
}

fn parse_notes(el: &MsdElement) -> Result<ChartData, LoadError> {
	// this is absolutely *unbelievably* ridiculous. We expect exactly 6 values.
	//
	// A lot of charts (for whatever reason) in modern packs have comments like this
	// //--------------- dance-single - sorae 80/31
	// ---------------
	// The newline means the comment doesn't tear out the whole thing
	// and there's a trailing "-------" in the list of values
	// as such, even though we only expect 6 params
	// we may find more than that. that's completely fine, stepmania will accept it.
	if el.values.len() < 6 {
		return Err(LoadError::InvalidSM(format!(
			"Invalid amount of fields inside #NOTES. Got {}, expected at least 6.",
			el.values.len()
		)));
	}

	let fields = &el.values;

	let steps_type = StepsType::from_bytes(&fields[0]);

	let author = &fields[1];
	let diff = &fields[2];

	let difficulty = match diff.to_ascii_lowercase().as_slice() {
		b"beginner" => Difficulty::Beginner,
		b"easy" | b"basic" | b"light" => Difficulty::Easy,
		b"medium" | b"another" | b"trick" | b"standard" | b"difficult" => Difficulty::Medium,
		b"hard" | b"ssr" | b"maniac" | b"heavy" => Difficulty::Hard,
		b"challenge" | b"expert" | b"oni" => Difficulty::Challenge,
		b"edit" => Difficulty::Edit(author.clone()),
		d => {
			return Err(LoadError::InvalidSM(format!(
				"Unknown difficulty {}",
				String::from_utf8_lossy(d),
			)));
		}
	};

	let level = String::from_utf8_lossy(&fields[3]).parse().unwrap_or(1);

	let raw_notedata = &fields[5];

	let notedata = parse_notedata(raw_notedata);

	Ok(ChartData {
		steps_type,
		author: author.clone(),
		difficulty,
		level,
		notedata,
	})
}

#[cfg(test)]
mod tests {
	use pretty_assertions::assert_eq;

	use super::*;

	#[test]
	fn bpms() {
		assert_eq!(
			parse_bpms(b"0.000=104.03"),
			Ok(vec![Bpm {
				bpm: 104.03,
				offset_beats: 0.0
			}])
		);

		assert_eq!(
			parse_bpms(b"0.000=104.03,1.000=400"),
			Ok(vec![
				Bpm {
					bpm: 104.03,
					offset_beats: 0.0
				},
				Bpm {
					bpm: 400.00,
					offset_beats: 1.0
				}
			])
		);

		assert_eq!(
			parse_bpms(b"0.000=-104.03"),
			Err(LoadError::InvalidSM("BPM was negative (-104.03)".into()))
		);
	}

	#[test]
	fn bpm_partial() {
		assert_eq!(
			parse_bpms(b"0.000=123.456.789"),
			Ok(vec![Bpm {
				bpm: 123.456,
				offset_beats: 0.0
			}])
		);
	}

	#[test]
	fn load_notes() {
		assert_eq!(
			parse_notes(&MsdElement {
				tag: Box::new(*b"NOTES"),
				values: vec![
					Box::new(*b"dance-single"),
					Box::new(*b"Author"),
					Box::new(*b"Hard"),
					Box::new(*b"1"),
					Box::new(*b"nonsense groove"),
					Box::new(
						*b"1000
0100
0010
0001,
M000
00000
1234
LKMF"
					)
				]
			}),
			Ok(ChartData {
				steps_type: StepsType::DanceSingle,
				author: Box::new(*b"Author"),
				difficulty: Difficulty::Hard,
				level: 1,
				notedata: vec![
					Measure {
						size: 4,
						events: vec![
							Event {
								column: 0,
								row: 0,
								variant: NoteVariant::Note
							},
							Event {
								column: 1,
								row: 1,
								variant: NoteVariant::Note
							},
							Event {
								column: 2,
								row: 2,
								variant: NoteVariant::Note
							},
							Event {
								column: 3,
								row: 3,
								variant: NoteVariant::Note
							},
						]
					},
					Measure {
						size: 4,
						events: vec![
							Event {
								column: 0,
								row: 0,
								variant: NoteVariant::Mine
							},
							Event {
								column: 0,
								row: 2,
								variant: NoteVariant::Note
							},
							Event {
								column: 1,
								row: 2,
								variant: NoteVariant::HoldStart
							},
							Event {
								column: 2,
								row: 2,
								variant: NoteVariant::HoldOrRollEnd
							},
							Event {
								column: 3,
								row: 2,
								variant: NoteVariant::RollStart
							},
							Event {
								column: 0,
								row: 3,
								variant: NoteVariant::Lift
							},
							Event {
								column: 1,
								row: 3,
								variant: NoteVariant::AutoKeysound
							},
							Event {
								column: 2,
								row: 3,
								variant: NoteVariant::Mine
							},
							Event {
								column: 3,
								row: 3,
								variant: NoteVariant::Fake
							},
						]
					}
				]
			})
		)
	}

	#[test]
	fn load_notes_obscurekeysounds() {
		assert_eq!(
			parse_notes(&MsdElement {
				tag: Box::new(*b"NOTES"),
				values: vec![
					Box::new(*b"dance-single"),
					Box::new(*b"Author"),
					Box::new(*b"Hard"),
					Box::new(*b"1"),
					Box::new(*b"nonsense groove"),
					Box::new(
						*b"1000
0100[1]
001[100000]0
0001[1,
[1]M000
00[1]000
123[999}>)]4
LKMF"
					)
				]
			}),
			Ok(ChartData {
				steps_type: StepsType::DanceSingle,
				author: Box::new(*b"Author"),
				difficulty: Difficulty::Hard,
				level: 1,
				notedata: vec![
					Measure {
						size: 4,
						events: vec![
							Event {
								column: 0,
								row: 0,
								variant: NoteVariant::Note
							},
							Event {
								column: 1,
								row: 1,
								variant: NoteVariant::Note
							},
							Event {
								column: 2,
								row: 2,
								variant: NoteVariant::Note
							},
							Event {
								column: 3,
								row: 3,
								variant: NoteVariant::Note
							},
						]
					},
					Measure {
						size: 4,
						events: vec![
							Event {
								column: 0,
								row: 0,
								variant: NoteVariant::Mine
							},
							Event {
								column: 0,
								row: 2,
								variant: NoteVariant::Note
							},
							Event {
								column: 1,
								row: 2,
								variant: NoteVariant::HoldStart
							},
							Event {
								column: 2,
								row: 2,
								variant: NoteVariant::HoldOrRollEnd
							},
							Event {
								column: 3,
								row: 2,
								variant: NoteVariant::RollStart
							},
							Event {
								column: 0,
								row: 3,
								variant: NoteVariant::Lift
							},
							Event {
								column: 1,
								row: 3,
								variant: NoteVariant::AutoKeysound
							},
							Event {
								column: 2,
								row: 3,
								variant: NoteVariant::Mine
							},
							Event {
								column: 3,
								row: 3,
								variant: NoteVariant::Fake
							},
						]
					}
				]
			})
		)
	}

	#[test]
	fn infer_author_normal() {
		assert_eq!(
			Chart::infer_author("Songs/Tachyon Epsilon/Hello (Kommisar)/chart.sm"),
			Some("Kommisar".into())
		);
	}

	#[test]
	fn infer_author_multiple() {
		assert_eq!(
			Chart::infer_author(
				"Songs/Tachyon Epsilon/Hello (as we approach the sky) (Kommisar)/chart.sm"
			),
			Some("Kommisar".into())
		);
	}

	#[test]
	fn infer_author_square() {
		assert_eq!(
			Chart::infer_author("Songs/Tachyon Epsilon/Hello [Kommisar]/chart.sm"),
			Some("Kommisar".into())
		);
	}

	#[test]
	fn infer_author_start() {
		assert_eq!(
			Chart::infer_author("Songs/Tachyon Epsilon/(Kommisar) Hello/chart.sm"),
			Some("Kommisar".into())
		);
	}

	#[test]
	fn infer_author_sq_start() {
		assert_eq!(
			Chart::infer_author("Songs/Tachyon Epsilon/[Kommisar] Hello/chart.sm"),
			Some("Kommisar".into())
		);
	}

	#[test]
	fn infer_author_space() {
		assert_eq!(
			Chart::infer_author("Songs/Tachyon Epsilon/[ Kommisar ] Hello/chart.sm"),
			Some("Kommisar".into())
		);
	}

	#[test]
	fn infer_author_space2() {
		assert_eq!(
			Chart::infer_author("Songs/Tachyon Epsilon/( Kommisar ) Hello/chart.sm"),
			Some("Kommisar".into())
		);
	}
}
