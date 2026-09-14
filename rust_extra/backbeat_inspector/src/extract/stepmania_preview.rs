#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
use rg_formats::sm::{Bpm, Measure, Stop};
use rg_formats::sm_msd::MsdFile;

const DEFAULT_SAMPLE_LENGTH: f64 = 10.0;
pub(crate) fn preview_start(
	tags: &MsdFile,
	bpms: &[Bpm],
	stops: &[Stop],
	offset_secs: Option<f64>,
	measures: &[Measure],
) -> u64 {
	let sample_start = tag_number(tags, "SAMPLESTART");
	let sample_length = tag_number(tags, "SAMPLELENGTH");

	if let (Some(start), Some(length)) = (sample_start, sample_length)
		&& start >= 0.0
		&& length > 0.0
	{
		return seconds_to_millis(start);
	}

	let last_beat = last_beat(measures);
	let beat_100 = elapsed_at_beat(100.0, bpms, stops, offset_secs);
	let song_end = elapsed_at_beat(last_beat, bpms, stops, offset_secs);

	if beat_100 + DEFAULT_SAMPLE_LENGTH <= song_end {
		seconds_to_millis(beat_100)
	} else {
		let midpoint = (last_beat / 2.0).max(0.0);
		let measure = (midpoint / 4.0).floor() * 4.0;
		seconds_to_millis(elapsed_at_beat(measure, bpms, stops, offset_secs))
	}
}

fn tag_number(tags: &MsdFile, tag: &str) -> Option<f64> {
	tags.get_first_value(tag).and_then(|value| {
		std::str::from_utf8(value.as_ref())
			.ok()?
			.trim()
			.parse()
			.ok()
	})
}

fn seconds_to_millis(seconds: f64) -> u64 {
	(seconds.max(0.0) * 1000.0).round() as u64
}

fn last_beat(measures: &[Measure]) -> f64 {
	measures
		.iter()
		.enumerate()
		.flat_map(|(measure_idx, measure)| {
			measure.events.iter().map(move |event| {
				(measure_idx as f64)
					.mul_add(4.0, event.row as f64 * 4.0 / measure.size.max(1) as f64)
			})
		})
		.max_by(f64::total_cmp)
		.unwrap_or(0.0)
}

fn elapsed_at_beat(beat: f64, bpms: &[Bpm], stops: &[Stop], offset_secs: Option<f64>) -> f64 {
	let mut elapsed = offset_secs.unwrap_or(0.0);
	let mut previous_beat = 0.0;
	let mut bpm = bpms.first().map_or(120.0, |bpm| bpm.bpm);

	let mut changes: Vec<(f64, Option<f64>, f64)> = bpms
		.iter()
		.filter(|bpm| bpm.offset_beats > 0.0 && bpm.offset_beats <= beat)
		.map(|bpm| (bpm.offset_beats, Some(bpm.bpm), 0.0))
		.chain(
			stops
				.iter()
				.filter(|stop| stop.offset_beats >= 0.0 && stop.offset_beats <= beat)
				.map(|stop| (stop.offset_beats, None, stop.duration.max(0.0))),
		)
		.collect();
	changes.sort_by(|a, b| a.0.total_cmp(&b.0));

	for (change_beat, new_bpm, stop_duration) in changes {
		elapsed += (change_beat - previous_beat) * 60.0 / bpm;
		elapsed += stop_duration;
		if let Some(new_bpm) = new_bpm {
			bpm = new_bpm;
		}
		previous_beat = change_beat;
	}

	elapsed + (beat - previous_beat) * 60.0 / bpm
}

#[cfg(test)]
mod tests {
	use super::*;
	use rg_formats::sm::{Event, NoteVariant};
	use rg_formats::sm_msd;

	fn tags(raw: &[u8]) -> MsdFile {
		sm_msd::from_bytes(raw)
	}

	fn bpm() -> Vec<Bpm> {
		vec![Bpm {
			bpm: 120.0,
			offset_beats: 0.0,
		}]
	}

	#[test]
	fn uses_explicit_sample_start() {
		assert_eq!(
			preview_start(
				&tags(b"#SAMPLESTART:14.801;#SAMPLELENGTH:12;"),
				&bpm(),
				&[],
				None,
				&[],
			),
			14_801
		);
	}

	#[test]
	fn falls_back_to_beat_100_for_a_long_chart() {
		let measure = Measure {
			size: 1,
			events: vec![Event {
				row: 0,
				column: 0,
				variant: NoteVariant::Note,
			}],
		};
		let measures = vec![measure; 60];
		assert_eq!(
			preview_start(&tags(b""), &bpm(), &[], None, &measures),
			50_000
		);
	}
}
