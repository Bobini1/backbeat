//! Parsing and processing of `.bms` files.
//!
//! This also includes `.pms`, `.bme` and `.bml` files.

pub mod lanes;
mod random;
mod raw_headers;

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Display;
use std::path::Path;
use std::{fs, io, str};

use encoding_rs::SHIFT_JIS;
use tracing::Level;

pub use self::lanes::{Lane, LaneKind, LanePosition};
pub use self::random::BmsRandomStrategy;
pub use self::raw_headers::{RawBmsHeaders, RawBmsHeadersIter};

/// Various ways loading a bms file might fail.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
	/// The BMS file could not be read.
	#[error("could not read BMS file: {0}")]
	Io(#[from] io::Error),
}

/// Events are positioned in BMS using fractions. You can think of this like a mixed number.
///
/// `measure_no` declares the whole number before the fraction,
/// and `numerator`/`denominator` indicate Where in this measure the event occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EventPosition {
	/// The measure this event occurs under.
	pub measure_num: u32,
	/// The numerator for this event
	pub numerator: u32,
	/// The denominator for this event.
	pub denominator: u32,
}

impl EventPosition {
	/// The zero point for any event.
	pub const ZERO: Self = Self {
		denominator: 1,
		measure_num: 0,
		numerator: 0,
	};
}

impl PartialOrd for EventPosition {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for EventPosition {
	fn cmp(&self, other: &Self) -> Ordering {
		match self.measure_num.cmp(&other.measure_num) {
			Ordering::Equal => {}
			other => return other,
		}

		let self_frac = self.numerator * other.denominator;
		let other_frac = other.numerator * self.denominator;

		self_frac.cmp(&other_frac)
	}
}

/// An event is merely something that *happens* in a BMS chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Event {
	// order of fields important here bc partialord derivation
	/// When this event occured in the chart.
	pub pos: EventPosition,
	/// What channel this event occured on.
	pub channel: Channel,
	/// What the actual "value" at this channel is. This declares what should happen here
	/// on this column.
	pub value: Channel,
}

/// The gamemode this BMS file is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Mode {
	/// 5 Keys, 1 turntable.
	Beat5,
	/// 7 Keys, 1 turntable.
	Beat7,
	/// 10 Keys, two turntables.
	Beat10,
	/// 14 keys, two turntables.
	Beat14,
	/// 5 Button Pop Charts may be represented in two ways.
	/// This mode uses the channels 13, 14, 15, 18, 19.
	Pop5SP,
	/// 9 Button Pop Charts may be represented in two ways.
	/// This mode uses the channels 11, 12, 13, 14, 15, 18, 19, 16, 17.
	Pop9SP,
	/// 5 Button Pop Charts may be represented in two ways.
	/// This mode uses the channels 13, 14, 15, 22, 23.
	Pop5DP,
	/// 9 Button Pop Charts may be represented in two ways.
	/// This mode uses the channels 11, 12, 13, 14, 15, 22, 23, 24, 25
	Pop9DP,

	/// 24 Keys on a keyboard.
	Kb24,
	/// 48 Keys on a keyboard.
	Kb48,
}

impl Mode {
	#[tracing::instrument(skip_all, level = Level::DEBUG, ret)]
	fn figure_out(events: &[Event], ext: &str) -> Self {
		let is_pms = ext == "pms";
		let mut has_sp_extend = false;
		let mut has_dp = false;
		let mut has_dp_extend = false;
		let mut has_pms_extend = false;

		for ev in events {
			use channels::*;

			// any events on 2X or 4X channels imply DP
			if ev.channel.0 == b'2' || ev.channel.0 == b'4' || ev.channel.0 == b'6' {
				tracing::debug!("sp extend because of notes on dp {}", ev.channel);
				has_dp = true;
			}

			if is_pms {
				match ev.channel {
					POP_1 | POP_2 | POP_8_DP | POP_9_DP | POP_8_SP | POP_9_SP
						if !has_pms_extend =>
					{
						tracing::debug!("pop extend because of notes on b1, b2, b8 or b9");
						has_pms_extend = true;
					}
					_ => {}
				}
			} else {
				match ev.channel {
					BEAT_K6 | BEAT_K7 | HOLD_K6 | HOLD_K7 | INVIS_BEAT_K6 | INVIS_BEAT_K7
						if !has_sp_extend =>
					{
						tracing::debug!("sp extend because of notes on k6 or k7");
						has_sp_extend = true;
					}
					BEAT_DP_K6 | BEAT_DP_K7 | HOLD_DP_K6 | HOLD_DP_K7 | INVIS_BEAT_DP_K6
					| INVIS_BEAT_DP_K7
						if !has_dp_extend =>
					{
						tracing::debug!("dp extend because of notes on k13 or k14");
						has_dp_extend = true;
					}

					_ => {}
				}
			}
		}

		if is_pms {
			return match (has_dp, has_pms_extend) {
				(true, false) => Mode::Pop5DP,
				(true, true) => Mode::Pop9DP,

				(false, false) => Mode::Pop5SP,
				(false, true) => Mode::Pop9SP,
			};
		}

		if has_dp_extend {
			return Mode::Beat14;
		} else if has_dp {
			return Mode::Beat10;
		} else if has_sp_extend {
			return Mode::Beat7;
		} else {
			return Mode::Beat5;
		}
	}
}

/// How hard the judgement windows should be for this chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(missing_docs)]
pub enum JudgeRank {
	VeryHard = 0,
	Hard = 1,
	#[default]
	Normal = 2,
	Easy = 3,
	VeryEasy = 4,
}

/// Metadata for a BMS chart.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Metadata {
	/// What gamemode this chart is for. This determines which channels should be
	/// considered for gameplay.
	pub mode: Mode,
	/// The bpm this chart starts at.
	pub initial_bpm: f64,

	/// The title for this chart.
	pub title: Option<String>,
	/// The subtitle for this chart. This is frequently (ab)used to indicate charter or
	/// difficulty name.
	pub subtitle: Option<String>,
	/// The genre for this chart.
	pub genre: Option<String>,
	/// The author of the song. This is frequently (ab)used to indicate charter.
	pub artist: Option<String>,
	/// Other artists in the song. This is sometimes used to indicate charter.
	pub subartist: Option<String>,

	/// A 300x80 banner image file to display.
	pub banner: Option<String>,
	/// A 640x480 image displayed while loading the chart.
	pub stagefile: Option<String>,
	/// Used to display the song title and artist with a custom image.
	pub backbmp: Option<String>,
	/// A path to a music file to play as preview on song select.
	pub preview: Option<String>,

	/// How tight the judgement windows should be. Defaults to [`JudgeRank::Normal`].
	pub rank: JudgeRank,
	/// How much gauge you can gain in the chart (in total). A total below 60.0 is
	/// unclearable on normal or easy gauge.
	pub total: f64,

	/// A lookup table for image changes.
	pub bmp: HashMap<Channel, String>,
	/// A lookup table for keysounds.
	pub wav: HashMap<Channel, String>,
	/// A lookup table for all bpm changes.
	pub bpm: HashMap<Channel, f64>,
}

impl Default for Metadata {
	fn default() -> Self {
		const DEFAULT_TOTAL: f64 = 100.0;

		Self {
			mode: Mode::Beat5,
			rank: JudgeRank::Normal,
			total: DEFAULT_TOTAL,
			initial_bpm: 130.0,

			artist: None,
			title: None,
			backbmp: None,
			banner: None,
			genre: None,
			preview: None,
			stagefile: None,
			subartist: None,
			subtitle: None,
			bmp: HashMap::default(),
			bpm: HashMap::default(),
			wav: HashMap::default(),
		}
	}
}

/// A bms chart is composed of metadata and some event information.
///
/// Metadata contains things like "ARTIST" and "TITLE" information; events are things
/// that occur on a channel with a value, at a certain time.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Chart {
	/// The relevant metadata for this chart.
	pub metadata: Metadata,

	/// The underlying metadata for this chart. This contains every key, mapped to
	/// every value, preserving source order and duplicates.
	#[cfg_attr(feature = "serde", serde(skip))]
	pub raw_metadata: RawBmsHeaders,

	/// All of the events that occur in this chart.
	pub events: Vec<Event>,
}

impl Chart {
	/// Return the lookup IDs of WAV definitions that are used by chart events.
	pub fn used_wav_lookup_ids(&self) -> impl Iterator<Item = Channel> + '_ {
		self.events
			.iter()
			.filter(|event| channels::uses_wav(event.channel))
			.map(|event| event.value)
	}

	/// Return the lookup IDs of BMP definitions that are used by chart events.
	pub fn used_bmp_lookup_ids(&self) -> impl Iterator<Item = Channel> + '_ {
		self.events
			.iter()
			.filter(|event| channels::uses_bmp(event.channel))
			.map(|event| event.value)
	}

	/// Get the value at this tag, if one exists.
	pub fn get_tag_raw(&self, tag: &str) -> Option<&str> {
		self.raw_metadata.get_first_tag(tag)
	}

	/// Get this tag as decoded text.
	pub fn get_tag_maybestr(&self, tag: &str) -> Option<&str> {
		self.raw_metadata.get_first_tag(tag)
	}

	/// Get this tag, parse it as UTF8, trim it, and treat it as "" if not defined.
	pub fn get_tag_pretty(&self, tag: &str) -> String {
		self.raw_metadata
			.get_first_tag(tag)
			.unwrap_or_default()
			.trim()
			.to_string()
	}

	/// Get a list of all bpm changes and where they occur.
	pub fn get_tempo_changes(&self) -> BTreeMap<EventPosition, f64> {
		let mut out = BTreeMap::new();

		out.insert(EventPosition::ZERO, self.metadata.initial_bpm);

		for ev in &self.events {
			if ev.channel == channels::BPM {
				let Some(&bpm) = self.metadata.bpm.get(&ev.value) else {
					tracing::warn!("reference to nonexistent BPM{}", ev.value);
					continue;
				};

				out.insert(ev.pos, bpm);
			} else if ev.channel == channels::INCHANNEL_BPM {
				let Some(bpm) = parse_b16(&[ev.value.0, ev.value.1]) else {
					tracing::warn!("invalid bpm {}", ev.value);
					continue;
				};

				out.insert(ev.pos, bpm as f64);
			}
		}

		out
	}
}

/// A channel is a "lane" on which events can happen in BMS.
/// They are limited to 36 lanes (0 to 9, A to Z giving an additional 26 values).
///
/// This wrapper struct provides nicer interfaces to working with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Channel(u8, u8);

#[cfg(feature = "serde")]
impl serde::Serialize for Channel {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serializer.serialize_str(str::from_utf8(&[self.0, self.1]).unwrap())
	}
}

#[cfg(feature = "serde")]
struct ChannelVisitor;

#[cfg(feature = "serde")]
impl serde::de::Visitor<'_> for ChannelVisitor {
	type Value = Channel;

	fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
		formatter.write_str("a bms channel between 00 and ZZ")
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: serde::de::Error,
	{
		Channel::from_str(v)
			.ok_or_else(|| serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self))
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Channel {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		deserializer.deserialize_str(ChannelVisitor)
	}
}

impl Display for Channel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(str::from_utf8(&[self.0, self.1]).unwrap())
	}
}

impl Channel {
	/// Get the string value of this channel.
	// todo: make this not allocate
	pub fn as_str(&self) -> String {
		String::from_utf8(vec![self.0, self.1]).expect("only valid utf8 can get in here")
	}

	/// Parse a channel from some bytes. THe bytes should be two characters long and
	/// correspond to "[0-9A-Za-z][0-9][A-Z][a-z]".
	fn from_bytes(bytes: &[u8]) -> Option<Self> {
		match str::from_utf8(bytes) {
			Ok(v) => Self::from_str(v),
			Err(_) => {
				tracing::warn!("invalid channel");
				None
			}
		}
	}

	/// Parse a channel from a string. The string should be two characters long and
	/// match `[0-9A-Za-z]`.
	pub const fn from_str(str: &str) -> Option<Self> {
		let bytes = str.as_bytes();

		if bytes.len() != 2 {
			// invalid channel
			return None;
		}

		Some(Self(bytes[0], bytes[1]))
	}

	/// Figure out the significance of this channel for this mode.
	pub fn as_lane(self, mode: Mode) -> Option<Lane> {
		Lane::new(self, mode)
	}
}

/// Parse an events line: this looks like
///
/// ```bms
/// #00619:00000000000000000047000000000000
/// ```
///
/// where
/// ```bms
/// #MMMCC:EVEVEVEVEV
/// ```
/// MMM -> measure number
/// CC -> channel number
/// [EV] -> events and their positions.
fn parse_measure(line: &str, events: &mut Vec<Event>) {
	let Some((position, values)) = line.split_once(':') else {
		tracing::warn!("not a measure");
		return;
	};

	let position = position.as_bytes();
	if position.len() != 6 {
		tracing::warn!("invalid measure position");
		return;
	}

	let Some(measure_num) = parse_b10(&position[1..4]) else {
		tracing::warn!("invalid measure num");
		return;
	};

	let Some(channel) = Channel::from_bytes(&position[4..6]) else {
		tracing::warn!("invalid channel num");
		return;
	};

	let values = values.as_bytes();
	let denominator = (values.len() / 2) as u32;

	for (numerator, chunk) in values.chunks_exact(2).enumerate() {
		let &[b1, b2] = chunk else { unreachable!() };
		let s = &[b1, b2];
		let Ok(value_str) = str::from_utf8(s) else {
			tracing::warn!(
				"invalid (nonascii) event channel {}",
				String::from_utf8_lossy(&[b1, b2])
			);
			continue;
		};

		let Some(event_value) = Channel::from_str(value_str) else {
			tracing::warn!("invalid event channel {value_str}");
			continue;
		};

		if event_value == channels::ZERO {
			continue;
		}

		events.push(Event {
			channel,
			value: event_value,
			pos: EventPosition {
				denominator,
				measure_num,
				numerator: numerator as u32,
			},
		});
	}
}

/// Is this line a measure?
///
/// This checks if the first character in the tag is 0-9. This is silly, but it's how
/// beatoraja does it, and therefore is de facto how measures should be parsed in bms.
fn is_measure(line: &str) -> bool {
	let bytes = line.as_bytes();
	bytes.first() == Some(&b'#')
		&& bytes.get(1).is_some_and(|ch| *ch >= b'0' && *ch <= b'9')
		&& bytes.len() > 6
}

/// Is this line metadata?
///
/// This checks if the line starts with #, % or @, and then contains whitespace.
fn parse_metadata(line: &str) -> Option<(&str, &str)> {
	let first = line.as_bytes().first()?;

	match *first {
		// @ and % might be beatoraja-specific extensions. Quite a couple of bms files have
		// metadata starting with this, but the metadata seems irrelevant to the chart.
		// Stuff like @EMAIL and %URL.
		b'#' | b'@' | b'%' => line[1..].split_once(' '),
		_ => None,
	}
}

fn is_random_control_header(key: &str) -> bool {
	matches!(
		key,
		"RANDOM" | "RONDAM" | "IF" | "SWITCH" | "SETSWITCH" | "SETRANDOM" | "ENDIF" | "ENDRANDOM"
	)
}

/// Parse the raw headers (i.e. no random evaluation) from a `.bms`, `.bme`, `.bml` or `.pms` file.
pub fn raw_headers_from_file(path: impl AsRef<Path>) -> Result<RawBmsHeaders, LoadError> {
	let bytes = fs::read(&path)?;
	let ext = crate::utils::safe_extension(path.as_ref()).unwrap_or("bms");

	raw_headers_from_bytes(&bytes, ext)
}

/// Parse the raw headers from `.bms`, `.bme`, `.bml` or `.pms` bytes.
pub fn raw_headers_from_bytes(bytes: &[u8], _ext: &str) -> Result<RawBmsHeaders, LoadError> {
	let text = ms932_decode(bytes);
	Ok(raw_headers_from_text(&text))
}

fn raw_headers_from_text(text: &str) -> RawBmsHeaders {
	let mut headers = RawBmsHeaders::default();

	for line in text.split(['\n', '\r']) {
		let line = line.trim_start_matches([' ', '\t']);

		if is_measure(line) {
			continue;
		}

		let Some((key, value)) = parse_metadata(line) else {
			continue;
		};

		let key = key.to_ascii_uppercase();

		if is_random_control_header(&key) {
			continue;
		}

		headers.push(key, value.to_owned());
	}

	headers
}

/// Parse a `.bms`, `.bme`, `.bml` or `.pms` file.
pub fn from_file(
	path: impl AsRef<Path>,
	random_strategy: BmsRandomStrategy,
) -> io::Result<Result<Chart, LoadError>> {
	let bytes = fs::read(&path)?;

	let ext = crate::utils::safe_extension(path.as_ref()).unwrap_or("bms");

	Ok(from_bytes(&bytes, ext, random_strategy))
}

/// Parse a `.bms`, `.bme` or `.pms` file from bytes.
#[tracing::instrument(skip_all, err)]
pub fn from_bytes(
	bytes: &[u8],
	ext: &str,
	random_strategy: BmsRandomStrategy,
) -> Result<Chart, LoadError> {
	let text = ms932_decode(bytes);
	let text = random_strategy.preprocess(&text);
	let mut metadata = Metadata::default();
	let raw_metadata = raw_headers_from_text(&text);
	let mut events = vec![];

	for (i, line) in text.split(['\n', '\r']).enumerate() {
		let line = line.trim_start_matches([' ', '\t']);

		let _span = tracing::info_span!("parse_line", line_number = i + 1, line,);
		let _span = _span.enter();

		let Some(kind) = line.chars().next() else {
			tracing::trace!("ignoring empty line");
			continue;
		};

		if is_measure(line) {
			// this is note data
			parse_measure(line, &mut events);
		} else if let Some((key, value)) = parse_metadata(line) {
			// all keys are implicitly uppercased...
			let key = key.to_ascii_uppercase();

			// Handle lookup table types (i.e. #BPM73 100.30)
			if let Some(ch) = get_lookup_channel(&key, "BPM") {
				match lexical::parse(value) {
					Ok(v) if v > 0.0 => {
						metadata.bpm.insert(ch, v);
					}
					_ => {
						tracing::warn!("invalid bpm");
					}
				}
				continue;
			} else if let Some(ch) = get_lookup_channel(&key, "WAV") {
				metadata.wav.insert(ch, normalise_asset_path(value));
				continue;
			} else if let Some(ch) = get_lookup_channel(&key, "BMP") {
				metadata.bmp.insert(ch, normalise_asset_path(value));
				continue;
			}

			match key.as_str() {
				"ARTIST" => metadata.artist = Some(value.to_owned()),
				"TITLE" => metadata.title = Some(value.to_owned()),
				"GENRE" => metadata.genre = Some(value.to_owned()),
				"SUBARTIST" => metadata.subartist = Some(value.to_owned()),
				"SUBTITLE" => metadata.subtitle = Some(value.to_owned()),

				"PLAYER" => {
					// we should ignore this, it's misleading and potentially broken.
				}

				"BPM" => match lexical::parse(value) {
					Ok(bpm) if bpm > 0.0 => metadata.initial_bpm = bpm,
					_ => {
						tracing::warn!("invalid BPM")
					}
				},

				"TOTAL" => match lexical::parse(value) {
					Ok(total) if total > 0.0 => metadata.total = total,
					_ => {
						tracing::warn!("invalid TOTAL")
					}
				},

				"RANK" => match lexical::parse(value) {
					Ok(0) => metadata.rank = JudgeRank::VeryHard,
					Ok(1) => metadata.rank = JudgeRank::Hard,
					Ok(2) => metadata.rank = JudgeRank::Normal,
					Ok(3) => metadata.rank = JudgeRank::Easy,
					Ok(4) => metadata.rank = JudgeRank::VeryEasy,
					_ => {
						tracing::warn!("unrecognised #RANK value. Defaulting to 2 (Normal Judge)");
					}
				},

				"COMMENT" | "PLAYLEVEL" | "STAGEFILE" => {
					// tags that are not "unknown" but do not need any handling
				}

				_ => {
					tracing::debug!("unknown metadata key");
				}
			}
		} else {
			// it's a comment
			if !kind.is_ascii_whitespace() {
				tracing::debug!("ignoring comment")
			} else {
				tracing::trace!("ignoring empty line")
			}
		}
	}

	let mode = Mode::figure_out(&events, ext);

	metadata.mode = mode;

	Ok(Chart {
		metadata,
		raw_metadata,
		events,
	})
}

/// Parse a series of bytes as a base 10 integer.
fn parse_b10(bytes: &[u8]) -> Option<u32> {
	let mut output = 0;

	let len = bytes.len() - 1;

	for (index, &byte) in bytes.iter().enumerate() {
		let position = (len - index) as u32;
		let magnitude = 10u32.pow(position);

		let value = match byte {
			b'0'..=b'9' => byte - b'0',
			_ => {
				return None;
			}
		};

		let value = value as u32;

		output += value * magnitude
	}

	Some(output)
}

/// Parse a series of bytes as a base 16 (0-9A-F) integer.
fn parse_b16(bytes: &[u8]) -> Option<u32> {
	let mut output = 0;

	let len = bytes.len() - 1;

	for (index, &byte) in bytes.iter().enumerate() {
		let position = (len - index) as u32;
		let magnitude = 16u32.pow(position);

		let value = match byte {
			b'0'..=b'9' => byte - b'0',
			b'A'..=b'F' => byte - b'A' + 10,
			_ => {
				return None;
			}
		};

		let value = value as u32;

		output += value * magnitude
	}

	Some(output)
}

fn ms932_decode(bytes: &[u8]) -> String {
	let (text, _, was_errors) = SHIFT_JIS.decode(bytes);

	if was_errors {
		tracing::warn!("there were errors when decoding")
	}

	text.into_owned()
}

fn normalise_asset_path(path: &str) -> String {
	path.trim().replace('\\', "/")
}

/// Checks for keys in the form `BPM00`.
///
/// Returns Channel("00"), etc.
fn get_lookup_channel(key: &str, prefix: &str) -> Option<Channel> {
	Channel::from_str(key.strip_prefix(prefix)?)
}

/// Constants to refer to channels in the BMS format.
///
/// Different [`Mode`]s might choose to interpret the same channel differently.
pub mod channels {
	use super::Channel;

	const fn channel_or_die(str: &str) -> Channel {
		match Channel::from_str(str) {
			Some(v) => v,
			None => {
				panic!("invalid channel")
			}
		}
	}

	macro_rules! ch {
		($name:ident, $doc:expr, $value:expr) => {
			#[doc = $doc]
			pub const $name: Channel = channel_or_die($value);
		};
	}

	ch!(ZERO, "Rest, do nothing here.", "00");

	ch!(BGM, "Play a sound of wavXX here", "01");
	ch!(
		METER,
		"Change how long a measure is in beats. I don't understand this.",
		"02"
	);

	ch!(
		INCHANNEL_BPM,
		"Change the bpm here to the in-channel value.",
		"03"
	);
	ch!(BGA_BASE, "Display BMPxx here.", "04");
	ch!(EXTCHR, "Unused.", "05");
	ch!(BGA_POOR, "Display BMPxx if the player misses a note.", "06");
	ch!(
		BGA_LAYER,
		"Display BMPxx if the player misses a note.",
		"07"
	);
	ch!(
		BPM,
		"Change the bpm to the bpm referenced at BPMxx here.
This is how most BPM changes are done, as INCHANNEL_BPM only supports integers between 0 and 255.",
		"08"
	);
	ch!(STOP, "Run the stop instruction defined at STOPxx.", "09");

	ch!(BEAT_K1, "KEY1.", "11");
	ch!(BEAT_K2, "KEY2.", "12");
	ch!(BEAT_K3, "KEY3.", "13");
	ch!(BEAT_K4, "KEY4.", "14");
	ch!(BEAT_K5, "KEY5.", "15");
	ch!(BEAT_K6, "KEY6.", "18");
	ch!(BEAT_K7, "KEY7.", "19");
	ch!(BEAT_SCR, "Scratch Lane.", "16");
	ch!(FREE_ZONE_SCR, "Free Zone notes in the scratch lane.", "17");

	ch!(HOLD_K1, "A hold note on KEY1.", "51");
	ch!(HOLD_K2, "A hold note on KEY2.", "52");
	ch!(HOLD_K3, "A hold note on KEY3.", "53");
	ch!(HOLD_K4, "A hold note on KEY4.", "54");
	ch!(HOLD_K5, "A hold note on KEY5.", "55");
	ch!(HOLD_K6, "A hold note on KEY6.", "58");
	ch!(HOLD_K7, "A hold note on KEY7.", "59");
	ch!(HOLD_SCR, "A spin on the scratch lane.", "56");

	ch!(BEAT_DP_K1, "Double play's Right-Hand KEY1.", "21");
	ch!(BEAT_DP_K2, "Double play's Right-Hand KEY2.", "22");
	ch!(BEAT_DP_K3, "Double play's Right-Hand KEY3.", "23");
	ch!(BEAT_DP_K4, "Double play's Right-Hand KEY4.", "24");
	ch!(BEAT_DP_K5, "Double play's Right-Hand KEY5.", "25");
	ch!(BEAT_DP_K6, "Double play's Right-Hand KEY6.", "28");
	ch!(BEAT_DP_K7, "Double play's Right-Hand KEY7.", "29");
	ch!(BEAT_DP_SCR, "The right-hand scratch lane.", "26");
	ch!(
		FREE_ZONE_DP_SCR,
		"'Free Zone' notes in the right-hand scratch lane.",
		"27"
	);

	ch!(HOLD_DP_K1, "A hold note on DP's KEY1.", "61");
	ch!(HOLD_DP_K2, "A hold note on DP's KEY2.", "62");
	ch!(HOLD_DP_K3, "A hold note on DP's KEY3.", "63");
	ch!(HOLD_DP_K4, "A hold note on DP's KEY4.", "64");
	ch!(HOLD_DP_K5, "A hold note on DP's KEY5.", "65");
	ch!(HOLD_DP_K6, "A hold note on DP's KEY6.", "68");
	ch!(HOLD_DP_K7, "A hold note on DP's KEY7.", "69");
	ch!(HOLD_DP_SCR, "A spin on DP's scratch lane.", "66");

	ch!(INVIS_BEAT_K1, "Change keysound.", "31");
	ch!(INVIS_BEAT_K2, "Change keysound.", "32");
	ch!(INVIS_BEAT_K3, "Change keysound.", "33");
	ch!(INVIS_BEAT_K4, "Change keysound.", "34");
	ch!(INVIS_BEAT_K5, "Change keysound.", "35");
	ch!(INVIS_BEAT_K6, "Change keysound.", "38");
	ch!(INVIS_BEAT_K7, "Change keysound.", "39");
	ch!(INVIS_BEAT_SCR, "Change keysound.", "36");

	ch!(INVIS_BEAT_DP_K1, "Change keysound.", "41");
	ch!(INVIS_BEAT_DP_K2, "Change keysound.", "42");
	ch!(INVIS_BEAT_DP_K3, "Change keysound.", "43");
	ch!(INVIS_BEAT_DP_K4, "Change keysound.", "44");
	ch!(INVIS_BEAT_DP_K5, "Change keysound.", "45");
	ch!(INVIS_BEAT_DP_K6, "Change keysound.", "48");
	ch!(INVIS_BEAT_DP_K7, "Change keysound.", "49");
	ch!(INVIS_BEAT_DP_SCR, "Change keysound.", "46");

	// -- esoteric stuff --
	ch!(
		TEXT,
		"Change the text on screen to the text defined by TEXTxx",
		"99"
	);
	ch!(
		EXRANK_CHANGE,
		"Change the judgement windows to the windows defined by EXRANKxx",
		"A0"
	);

	ch!(POP_1, "A note on 1.", "11");
	ch!(POP_2, "A note on 2.", "12");
	ch!(POP_3, "A note on 3.", "13");
	ch!(POP_4, "A note on 4.", "14");
	ch!(POP_5, "A note on 5.", "15");
	ch!(POP_6_DP, "A note on 6 (DP variant).", "22");
	ch!(POP_7_DP, "A note on 7 (DP variant).", "23");
	ch!(POP_8_DP, "A note on 8 (DP variant).", "24");
	ch!(POP_9_DP, "A note on 9 (DP variant).", "25");

	ch!(POP_6_SP, "A note on 6 (SP variant).", "18");
	ch!(POP_7_SP, "A note on 7 (SP variant).", "19");
	ch!(POP_8_SP, "A note on 8 (SP variant).", "16");
	ch!(POP_9_SP, "A note on 9 (SP variant).", "17");

	/// Whether events on this channel refer to an entry in the `#WAVxx` table.
	pub const fn uses_wav(channel: Channel) -> bool {
		matches!(
			channel,
			BGM | BEAT_K1
				| BEAT_K2 | BEAT_K3
				| BEAT_K4 | BEAT_K5
				| BEAT_K6 | BEAT_K7
				| BEAT_SCR | FREE_ZONE_SCR
				| BEAT_DP_K1 | BEAT_DP_K2
				| BEAT_DP_K3 | BEAT_DP_K4
				| BEAT_DP_K5 | BEAT_DP_K6
				| BEAT_DP_K7 | BEAT_DP_SCR
				| FREE_ZONE_DP_SCR
				| INVIS_BEAT_K1
				| INVIS_BEAT_K2
				| INVIS_BEAT_K3
				| INVIS_BEAT_K4
				| INVIS_BEAT_K5
				| INVIS_BEAT_K6
				| INVIS_BEAT_K7
				| INVIS_BEAT_SCR
				| INVIS_BEAT_DP_K1
				| INVIS_BEAT_DP_K2
				| INVIS_BEAT_DP_K3
				| INVIS_BEAT_DP_K4
				| INVIS_BEAT_DP_K5
				| INVIS_BEAT_DP_K6
				| INVIS_BEAT_DP_K7
				| INVIS_BEAT_DP_SCR
				| HOLD_K1 | HOLD_K2
				| HOLD_K3 | HOLD_K4
				| HOLD_K5 | HOLD_K6
				| HOLD_K7 | HOLD_SCR
				| HOLD_DP_K1 | HOLD_DP_K2
				| HOLD_DP_K3 | HOLD_DP_K4
				| HOLD_DP_K5 | HOLD_DP_K6
				| HOLD_DP_K7 | HOLD_DP_SCR
		)
	}

	/// Whether events on this channel refer to an entry in the `#BMPxx` table.
	pub const fn uses_bmp(channel: Channel) -> bool {
		matches!(channel, BGA_BASE | BGA_POOR | BGA_LAYER)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn channel_serde() {
		assert_eq!(
			serde_json::to_string(&Channel::from_str("A9").unwrap()).unwrap(),
			"\"A9\""
		);
		assert_eq!(
			serde_json::from_str::<Channel>("\"A9\"").unwrap(),
			Channel::from_str("A9").unwrap()
		)
	}

	#[test]
	fn headers_from_bytes_preserves_duplicates() {
		let headers =
			raw_headers_from_bytes(b"#TITLE First\n#TITLE Second\n#00111:0100\n", "bms").unwrap();

		assert_eq!(headers.len(), 2);
		assert_eq!(
			headers.iter().collect::<Vec<_>>(),
			vec![("TITLE", "First"), ("TITLE", "Second")]
		);
	}

	#[test]
	fn headers_from_bytes_ignores_random_controls() {
		let headers = raw_headers_from_bytes(
			b"#RANDOM 2\n#IF 1\n#TITLE First\n#ENDIF\n#ENDRANDOM\n",
			"bms",
		)
		.unwrap();

		assert_eq!(headers.iter().collect::<Vec<_>>(), vec![("TITLE", "First")]);
	}

	#[test]
	fn asset_paths_are_decoded_as_ms932() {
		let chart = from_bytes(
			b"#WAV01 music-\x82\xa0.wav\n#BMP01 image\\cover-\x82\xa0.png\n",
			"bms",
			BmsRandomStrategy::AlwaysFirstBranch,
		)
		.unwrap();
		let channel = Channel::from_str("01").unwrap();

		assert_eq!(
			chart.metadata.wav.get(&channel).map(String::as_str),
			Some("music-あ.wav")
		);
		assert_eq!(
			chart.metadata.bmp.get(&channel).map(String::as_str),
			Some("image/cover-あ.png")
		);
	}

	#[test]
	fn used_asset_lookup_ids_come_from_their_respective_event_channels() {
		let chart = from_bytes(
			b"#00101:0100\n#00111:0200\n#00104:0300\n#00106:0400\n#00108:0500\n",
			"bms",
			BmsRandomStrategy::AlwaysFirstBranch,
		)
		.unwrap();

		let mut wav_ids = chart
			.used_wav_lookup_ids()
			.map(|id| id.as_str())
			.collect::<Vec<_>>();
		wav_ids.sort();
		assert_eq!(wav_ids, ["01", "02"]);

		let mut bmp_ids = chart
			.used_bmp_lookup_ids()
			.map(|id| id.as_str())
			.collect::<Vec<_>>();
		bmp_ids.sort();
		assert_eq!(bmp_ids, ["03", "04"]);
	}

	#[test]
	fn malformed_measure_positions_are_ignored() {
		for bytes in [
			b"#0:0000".as_slice(),
			b"#001:0000",
			b"#0011:0000",
			b"#001111:0000",
		] {
			let chart = from_bytes(bytes, "bms", BmsRandomStrategy::AlwaysFirstBranch).unwrap();
			assert!(chart.events.is_empty(), "{bytes:?}");
		}
	}

	#[test]
	fn _99_strange_parses() {
		let bytes = include_bytes!("../test_files/bms/_99_strange_chaotic_s.bms");

		let chart = from_bytes(bytes, "bms", BmsRandomStrategy::AlwaysFirstBranch).unwrap();

		assert_eq!(
			chart.metadata.title.as_deref(),
			Some("$trange Attraktor [Instable Scratch Simulator]")
		);
		assert!(!chart.events.is_empty());
	}
}
