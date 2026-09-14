//! Parsing of `.dwi` files.

use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, io};

use crate::sm::{
	Bpm, Chart, ChartData, Difficulty, Event, Measure, NoteVariant, SongMetadata, StepsType, Stop,
};
use crate::sm_msd::{self, MsdElement, MsdFile};
use crate::utils::ByteString;

const ROWS_PER_MEASURE: usize = 192;
const DEFAULT_INCREMENT: usize = 24;

#[derive(Clone, Copy)]
enum DwiMode {
	Single,
	Double,
	Couple,
	Solo,
}

impl DwiMode {
	fn from_tag(tag: &[u8]) -> Option<Self> {
		match tag {
			b"SINGLE" => Some(Self::Single),
			b"DOUBLE" => Some(Self::Double),
			b"COUPLE" => Some(Self::Couple),
			b"SOLO" => Some(Self::Solo),
			_ => None,
		}
	}

	fn steps_type(self) -> StepsType {
		match self {
			Self::Single => StepsType::DanceSingle,
			Self::Double => StepsType::DanceDouble,
			Self::Couple => StepsType::DanceCouple,
			Self::Solo => StepsType::DanceSolo,
		}
	}

	fn column(self, pad: usize, direction: Direction) -> Option<usize> {
		match self {
			Self::Single if pad == 0 => direction.four_panel_column(),
			Self::Double | Self::Couple => {
				direction.four_panel_column().map(|column| column + pad * 4)
			}
			Self::Solo if pad == 0 => direction.solo_column(),
			_ => None,
		}
	}

	fn uses_second_pad(self) -> bool {
		matches!(self, Self::Double | Self::Couple)
	}
}

#[derive(Clone, Copy)]
enum Direction {
	Left,
	UpLeft,
	Down,
	Up,
	UpRight,
	Right,
}

impl Direction {
	fn four_panel_column(self) -> Option<usize> {
		match self {
			Self::Left => Some(0),
			Self::Down => Some(1),
			Self::Up => Some(2),
			Self::Right => Some(3),
			Self::UpLeft | Self::UpRight => None,
		}
	}

	fn solo_column(self) -> Option<usize> {
		match self {
			Self::Left => Some(0),
			Self::UpLeft => Some(1),
			Self::Down => Some(2),
			Self::Up => Some(3),
			Self::UpRight => Some(4),
			Self::Right => Some(5),
		}
	}
}

struct NoteDataBuilder {
	columns: Vec<BTreeMap<usize, NoteVariant>>,
	end_tick: usize,
}

impl NoteDataBuilder {
	fn new(column_count: usize) -> Self {
		Self {
			columns: (0..column_count).map(|_| BTreeMap::new()).collect(),
			end_tick: 0,
		}
	}

	fn write(&mut self, column: usize, tick: usize, variant: NoteVariant) {
		if let Some(events) = self.columns.get_mut(column) {
			events.insert(tick, variant);
		}
	}

	fn record_end_tick(&mut self, tick: usize) {
		self.end_tick = self.end_tick.max(tick);
	}

	fn finish(mut self) -> Vec<Measure> {
		for events in &mut self.columns {
			let ticks: Vec<_> = events.keys().copied().collect();
			for (index, tick) in ticks.iter().copied().enumerate() {
				if events.get(&tick) != Some(&NoteVariant::HoldStart) {
					continue;
				}

				if let Some(tail) = ticks.get(index + 1) {
					events.insert(*tail, NoteVariant::HoldOrRollEnd);
				} else {
					events.remove(&tick);
				}
			}
		}

		let measure_count = self.end_tick.div_ceil(ROWS_PER_MEASURE);
		let mut measures = (0..measure_count)
			.map(|_| Measure {
				size: ROWS_PER_MEASURE,
				events: vec![],
			})
			.collect::<Vec<_>>();

		for (column, events) in self.columns.into_iter().enumerate() {
			for (tick, variant) in events {
				if let Some(measure) = measures.get_mut(tick / ROWS_PER_MEASURE) {
					measure.events.push(Event {
						row: tick % ROWS_PER_MEASURE,
						column,
						variant,
					});
				}
			}
		}

		for measure in &mut measures {
			measure
				.events
				.sort_by_key(|event| (event.row, event.column));
		}

		measures
	}
}

/// Load and parse a DWI file from disk.
pub fn from_path(dwi_path: impl AsRef<Path>) -> io::Result<Vec<Chart>> {
	let path = dwi_path.as_ref();
	let bytes = fs::read(path)?;
	Ok(from_bytes(&bytes, path))
}

/// Parse a DWI file from raw bytes.
pub fn from_bytes(bytes: &[u8], dwi_path: impl AsRef<Path>) -> Vec<Chart> {
	let path = dwi_path.as_ref();
	let tags = sm_msd::from_bytes_preserving_escapes(bytes);
	let song_info = parse_song(&tags, path);

	tags.elements
		.iter()
		.filter_map(parse_chart)
		.map(|chart_data| Chart {
			tags: tags.clone(),
			song_info: song_info.clone(),
			chart_data,
			path: path.to_path_buf(),
		})
		.collect()
}

fn parse_song(tags: &MsdFile, path: &Path) -> SongMetadata {
	let mut bpms = vec![];
	let mut stops = vec![];

	for element in &tags.elements {
		match element.tag.as_ref() {
			b"BPM" => {
				for value in &element.values {
					if let Some(bpm) = parse_number(value).filter(|bpm| *bpm > 0.0) {
						bpms.push(Bpm {
							bpm,
							offset_beats: 0.0,
						});
					}
				}
			}
			b"CHANGEBPM" | b"BPMCHANGE" => {
				for value in &element.values {
					for (offset, bpm) in parse_pairs(value) {
						if bpm > 0.0 {
							bpms.push(Bpm {
								bpm,
								offset_beats: offset / 4.0,
							});
						}
					}
				}
			}
			b"FREEZE" => {
				for value in &element.values {
					for (offset, duration) in parse_pairs(value) {
						stops.push(Stop {
							duration: duration / 1000.0,
							offset_beats: offset / 4.0,
						});
					}
				}
			}
			_ => {}
		}
	}

	if bpms.is_empty() {
		bpms.push(Bpm {
			bpm: 60.0,
			offset_beats: 0.0,
		});
	}

	let offset_secs = first_value(tags, b"GAP")
		.and_then(|gap| std::str::from_utf8(gap).ok())
		.and_then(|gap| gap.trim().parse::<i64>().ok())
		.and_then(|gap| (gap != 0).then_some(-(gap as f64) / 1000.0));

	SongMetadata {
		offset_secs,
		bpms,
		stops,
		subtitle: None,
		artist: first_value(tags, b"ARTIST")
			.map(|value| String::from_utf8_lossy(value).into_owned())
			.unwrap_or_else(|| "Unknown Artist".to_owned()),
		title: first_value(tags, b"TITLE")
			.map(|value| String::from_utf8_lossy(value).into_owned())
			.unwrap_or_else(|| title_from_path(path)),
		music: first_value(tags, b"FILE").map(crate::utils::os_string_from_bytes),
	}
}

fn first_value<'a>(tags: &'a MsdFile, tag: &[u8]) -> Option<&'a [u8]> {
	tags.elements
		.iter()
		.find(|element| element.tag.as_ref() == tag)
		.and_then(|element| element.values.first())
		.filter(|value| !value.is_empty())
		.map(AsRef::as_ref)
}

fn title_from_path(path: &Path) -> String {
	path.parent()
		.and_then(Path::file_name)
		.map(|name| name.to_string_lossy().into_owned())
		.unwrap_or_else(|| "Untitled Song".to_owned())
}

fn parse_number(value: &[u8]) -> Option<f64> {
	let value = std::str::from_utf8(value).ok()?;
	let (value, _) = lexical::parse_partial::<f64, &str>(value.trim()).ok()?;
	value.is_finite().then_some(value)
}

fn parse_pairs(value: &[u8]) -> impl Iterator<Item = (f64, f64)> + '_ {
	value.split(|byte| *byte == b',').filter_map(|entry| {
		let mut values = entry.split(|byte| *byte == b'=');
		let (Some(offset), Some(value), None) = (values.next(), values.next(), values.next())
		else {
			return None;
		};
		Some((parse_number(offset)?, parse_number(value)?))
	})
}

fn parse_chart(element: &MsdElement) -> Option<ChartData> {
	let mode = DwiMode::from_tag(&element.tag)?;
	let difficulty = parse_difficulty(element.values.first()?)?;
	let level = element
		.values
		.get(1)
		.and_then(|value| std::str::from_utf8(value).ok())
		.and_then(|value| value.trim().parse::<usize>().ok())
		.filter(|level| *level > 0)
		.unwrap_or(1);
	let step1 = element.values.get(2)?;
	let step2 = mode
		.uses_second_pad()
		.then(|| element.values.get(3))
		.flatten();

	if !is_usable_step_data(step1) && !step2.is_some_and(|value| is_usable_step_data(value)) {
		return None;
	}

	let mut notedata = NoteDataBuilder::new(match mode {
		DwiMode::Single => 4,
		DwiMode::Double | DwiMode::Couple => 8,
		DwiMode::Solo => 6,
	});
	parse_step_data(step1, mode, 0, &mut notedata);
	if let Some(step2) = step2 {
		parse_step_data(step2, mode, 1, &mut notedata);
	}

	Some(ChartData {
		steps_type: mode.steps_type(),
		author: ByteString::default(),
		difficulty,
		level,
		notedata: notedata.finish(),
	})
}

fn parse_difficulty(value: &[u8]) -> Option<Difficulty> {
	match value.trim_ascii().to_ascii_lowercase().as_slice() {
		b"beginner" => Some(Difficulty::Beginner),
		b"easy" | b"basic" | b"light" => Some(Difficulty::Easy),
		b"medium" | b"another" | b"trick" | b"standard" | b"difficult" => Some(Difficulty::Medium),
		b"hard" | b"ssr" | b"maniac" | b"heavy" => Some(Difficulty::Hard),
		b"smaniac" | b"challenge" | b"expert" | b"oni" => Some(Difficulty::Challenge),
		b"edit" => Some(Difficulty::Edit(ByteString::default())),
		_ => None,
	}
}

fn is_usable_step_data(value: &[u8]) -> bool {
	value
		.iter()
		.filter(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
		.nth(1)
		.is_some()
}

fn parse_step_data(value: &[u8], mode: DwiMode, pad: usize, notedata: &mut NoteDataBuilder) {
	let data: Vec<_> = value
		.iter()
		.copied()
		.filter(|byte| !matches!(byte, b' ' | b'\t' | b'\r' | b'\n'))
		.collect();
	let mut increment = DEFAULT_INCREMENT;
	let mut tick = 0;
	let mut index = 0;

	while let Some(byte) = data.get(index).copied() {
		index += 1;
		match byte {
			b'(' => increment = 12,
			b'[' => increment = 8,
			b'{' => increment = 3,
			b'`' => increment = 1,
			b')' | b']' | b'}' | b'\'' | b'>' => increment = DEFAULT_INCREMENT,
			b'!' => {}
			b'<' if legacy_192nd(&data, index) => increment = 1,
			b'<' => {
				while let Some(glyph) = data.get(index).copied() {
					index += 1;
					if glyph == b'>' {
						break;
					}
					write_glyph(glyph, &data, &mut index, mode, pad, tick, notedata);
				}
				tick += increment;
			}
			glyph => {
				write_glyph(glyph, &data, &mut index, mode, pad, tick, notedata);
				tick += increment;
			}
		}
	}

	notedata.record_end_tick(tick);
}

fn legacy_192nd(data: &[u8], mut index: usize) -> bool {
	while let Some(byte) = data.get(index) {
		if *byte == b'>' {
			return false;
		}
		if *byte == b'0' {
			return true;
		}
		index += 1;
	}
	false
}

fn write_glyph(
	glyph: u8,
	data: &[u8],
	index: &mut usize,
	mode: DwiMode,
	pad: usize,
	tick: usize,
	notedata: &mut NoteDataBuilder,
) {
	for direction in glyph_directions(glyph).into_iter().flatten() {
		if let Some(column) = mode.column(pad, direction) {
			notedata.write(column, tick, NoteVariant::Note);
		}
	}

	if data.get(*index) != Some(&b'!') {
		return;
	}
	*index += 1;
	let Some(hold_glyph) = data.get(*index).copied() else {
		return;
	};
	*index += 1;

	for direction in glyph_directions(hold_glyph).into_iter().flatten() {
		if let Some(column) = mode.column(pad, direction) {
			notedata.write(column, tick, NoteVariant::HoldStart);
		}
	}
}

fn glyph_directions(glyph: u8) -> [Option<Direction>; 2] {
	use Direction::*;

	match glyph {
		b'1' => [Some(Down), Some(Left)],
		b'2' => [Some(Down), None],
		b'3' => [Some(Down), Some(Right)],
		b'4' => [Some(Left), None],
		b'6' => [Some(Right), None],
		b'7' => [Some(Up), Some(Left)],
		b'8' => [Some(Up), None],
		b'9' => [Some(Up), Some(Right)],
		b'A' => [Some(Up), Some(Down)],
		b'B' => [Some(Left), Some(Right)],
		b'C' => [Some(UpLeft), None],
		b'D' => [Some(UpRight), None],
		b'E' => [Some(Left), Some(UpLeft)],
		b'F' => [Some(UpLeft), Some(Down)],
		b'G' => [Some(UpLeft), Some(Up)],
		b'H' => [Some(UpLeft), Some(Right)],
		b'I' => [Some(Left), Some(UpRight)],
		b'J' => [Some(Down), Some(UpRight)],
		b'K' => [Some(Up), Some(UpRight)],
		b'L' => [Some(UpRight), Some(Right)],
		b'M' => [Some(UpLeft), Some(UpRight)],
		_ => [None, None],
	}
}

#[cfg(test)]
mod tests {
	use pretty_assertions::assert_eq;

	use super::*;
	use crate::test_utils::test_file_read;

	#[test]
	fn parses_dwi_fixture() {
		let bytes = test_file_read("dwi/minimal.dwi");
		let charts = from_bytes(&bytes, "fixture.dwi");
		assert_eq!(charts.len(), 4);

		let single = &charts[0];
		assert_eq!(single.song_info.title, "DWI Test");
		assert_eq!(single.song_info.artist, "Example Artist");
		assert_eq!(
			single.song_info.music.as_deref().unwrap().to_string_lossy(),
			"music\\track.ogg"
		);
		assert_eq!(single.song_info.offset_secs, Some(-0.25));
		assert_eq!(
			single.song_info.bpms,
			vec![
				Bpm {
					bpm: 120.0,
					offset_beats: 0.0,
				},
				Bpm {
					bpm: 240.0,
					offset_beats: 4.0,
				},
				Bpm {
					bpm: 180.0,
					offset_beats: 5.0,
				},
			]
		);
		assert_eq!(
			single.song_info.stops,
			vec![Stop {
				duration: 1.5,
				offset_beats: 2.0,
			}]
		);
		assert_eq!(single.chart_data.steps_type, StepsType::DanceSingle);
		assert_eq!(single.chart_data.difficulty, Difficulty::Challenge);
		assert_eq!(single.chart_data.level, 7);
		assert_eq!(
			single.chart_data.notedata,
			vec![Measure {
				size: 192,
				events: vec![
					Event {
						row: 0,
						column: 0,
						variant: NoteVariant::HoldStart,
					},
					Event {
						row: 12,
						column: 0,
						variant: NoteVariant::HoldOrRollEnd,
					},
					Event {
						row: 36,
						column: 2,
						variant: NoteVariant::Note,
					},
					Event {
						row: 36,
						column: 3,
						variant: NoteVariant::Note,
					},
					Event {
						row: 60,
						column: 1,
						variant: NoteVariant::Note,
					},
					Event {
						row: 68,
						column: 1,
						variant: NoteVariant::Note,
					},
					Event {
						row: 68,
						column: 3,
						variant: NoteVariant::Note,
					},
					Event {
						row: 70,
						column: 2,
						variant: NoteVariant::Note,
					},
				],
			}]
		);

		let double = &charts[1].chart_data;
		assert_eq!(double.steps_type, StepsType::DanceDouble);
		assert!(
			double.notedata[0]
				.events
				.iter()
				.any(|event| event.column == 0)
		);
		assert!(
			double.notedata[0]
				.events
				.iter()
				.any(|event| event.column == 6)
		);

		let couple = &charts[2].chart_data;
		assert_eq!(couple.steps_type, StepsType::DanceCouple);
		assert!(
			couple.notedata[0]
				.events
				.iter()
				.any(|event| event.column == 3)
		);
		assert!(
			couple.notedata[0]
				.events
				.iter()
				.any(|event| event.column == 5)
		);

		let solo = &charts[3].chart_data;
		assert_eq!(solo.steps_type, StepsType::DanceSolo);
		assert!(
			solo.notedata[0]
				.events
				.iter()
				.any(|event| event.column == 1)
		);
		assert!(
			solo.notedata[0]
				.events
				.iter()
				.any(|event| event.column == 4)
		);
	}

	#[test]
	fn loads_dwi_fixture_from_path() {
		let charts = from_path(crate::test_utils::test_path("dwi/minimal.dwi")).unwrap();
		assert_eq!(charts.len(), 4);
	}

	#[test]
	fn skips_invalid_chart_tags() {
		let charts = from_bytes(
			b"#BPM:120;#SINGLE:UNKNOWN:9:44;#SINGLE:MANIAC:9:;#SINGLE:MANIAC:9:44;",
			"fixture.dwi",
		);
		assert_eq!(charts.len(), 1);
	}
}
