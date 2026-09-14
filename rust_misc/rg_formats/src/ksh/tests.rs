use super::*;
use body::{BodyOption, MeasureEvent, TiltValue};
use definition::DefinitionKind;
use header::Difficulty;
use row::{BtNote, FxNote, LaneSpinKind, LaserCell, Row};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a minimal valid KSH string: header lines, then one measure with one
/// empty row.
fn minimal(header: &str) -> String {
	format!("{header}\n--\n0000|00|--\n--\n")
}

/// Parse `src` and return the first row in the first measure.
fn first_row(src: &str) -> Row {
	let chart = from_bytes(src.as_bytes()).expect("parse failed");
	chart.measures[0]
		.events
		.iter()
		.find_map(|e| {
			if let MeasureEvent::Row(r) = e {
				Some(r.clone())
			} else {
				None
			}
		})
		.expect("no row found")
}

// ── Header ────────────────────────────────────────────────────────────────────

#[test]
fn ksh_encoding_is_detected_from_bom() {
	assert_eq!(KshEncoding::detect(b"title=Song"), KshEncoding::ShiftJis);
	assert_eq!(
		KshEncoding::detect(b"\xef\xbb\xbftitle=Song"),
		KshEncoding::Utf8
	);
}

#[test]
fn header_uses_detected_encoding() {
	let shift_jis = b"title=\x83\x65\n--\n0000|00|--\n--\n";
	assert_eq!(from_bytes(shift_jis).unwrap().header.title, "テ");

	let utf8 = b"\xef\xbb\xbftitle=\xe3\x83\x86\n--\n0000|00|--\n--\n";
	assert_eq!(from_bytes(utf8).unwrap().header.title, "テ");
}

#[test]
fn header_basic_fields() {
	let src = minimal(
		"title=Test Song\nartist=Test Artist\neffect=Charter\njacket=cover.jpg\ndifficulty=infinite\nlevel=18\nt=180",
	);
	let h = from_bytes(src.as_bytes()).unwrap().header;
	assert_eq!(h.title, "Test Song");
	assert_eq!(h.artist, "Test Artist");
	assert_eq!(h.effect, "Charter");
	assert_eq!(h.jacket, "cover.jpg");
	assert_eq!(h.difficulty, Difficulty::Infinite);
	assert_eq!(h.level, 18);
	assert!((h.bpm_init - 180.0).abs() < f64::EPSILON);
	assert_eq!(h.bpm, "180");
}

#[test]
fn header_difficulty_variants() {
	for (s, expected) in [
		("light", Difficulty::Light),
		("challenge", Difficulty::Challenge),
		("extended", Difficulty::Extended),
		("infinite", Difficulty::Infinite),
		("anything_else", Difficulty::Infinite),
	] {
		let h = from_bytes(minimal(&format!("difficulty={s}")).as_bytes())
			.unwrap()
			.header;
		assert_eq!(h.difficulty, expected, "difficulty={s}");
	}
}

#[test]
fn header_audio_split_on_semicolon() {
	let h = from_bytes(minimal("m=song.ogg;song_f.ogg;song_p.ogg;song_fp.ogg").as_bytes())
		.unwrap()
		.header;
	assert_eq!(
		h.audio.iter().map(String::as_str).collect::<Vec<_>>(),
		vec!["song.ogg", "song_f.ogg", "song_p.ogg", "song_fp.ogg",]
	);
}

#[test]
fn header_audio_single() {
	let h = from_bytes(minimal("m=track.ogg").as_bytes())
		.unwrap()
		.header;
	assert_eq!(
		h.audio.iter().map(String::as_str).collect::<Vec<_>>(),
		vec!["track.ogg"]
	);
}

#[test]
fn header_icon() {
	let h = from_bytes(minimal("icon=../../banner.jpg").as_bytes())
		.unwrap()
		.header;
	assert_eq!(h.unknown.get("icon"), Some(&"../../banner.jpg".to_owned()));
}

#[test]
fn header_bg_split_on_semicolon() {
	let h = from_bytes(minimal("bg=bg_lo.jpg;bg_hi.jpg").as_bytes())
		.unwrap()
		.header;
	assert_eq!(
		h.bg.iter().map(String::as_str).collect::<Vec<_>>(),
		vec!["bg_lo.jpg", "bg_hi.jpg"]
	);
}

#[test]
fn header_resource_paths_are_decoded() {
	let src = b"title=Legacy\n\
jacket=cover-\x82\xa0.jpg\n\
title_img=title-\x82\xa0.png\n\
artist_img=artist-\x82\xa0.png\n\
m=music-\x82\xa0.ogg;music-\x82\xa0-f.ogg\n\
bg=bg-\x82\xa0.jpg\n\
layer=layer-\x82\xa0.gif;1200;0\n\
v=movie-\x82\xa0.mp4\n\
--\n\
0000|00|--\n\
--\n";
	let h = from_bytes(src).unwrap().header;

	assert_eq!(h.jacket, "cover-あ.jpg");
	assert_eq!(h.title_img, "title-あ.png");
	assert_eq!(h.artist_img, "artist-あ.png");
	assert_eq!(h.audio, ["music-あ.ogg", "music-あ-f.ogg"]);
	assert_eq!(h.bg, ["bg-あ.jpg"]);
	assert_eq!(h.layer, "layer-あ.gif;1200;0");
	assert_eq!(h.video, "movie-あ.mp4");
}

#[test]
fn chart_exposes_raw_headers_in_source_order() {
	let chart =
		from_bytes(b"title=First\ntitle=Second\nm=track.ogg\n--\n0000|00|--\n--\n").unwrap();

	assert_eq!(chart.raw_headers.get_first("title"), Some("First"));
	assert_eq!(chart.raw_headers.len(), 3);
	assert_eq!(
		chart.raw_headers.iter().collect::<Vec<_>>(),
		vec![("title", "First"), ("title", "Second"), ("m", "track.ogg"),]
	);
}

#[test]
fn rejects_invalid_utf8_when_bom_is_present() {
	assert_eq!(
		from_bytes(b"\xef\xbb\xbftitle=\x80\n--\n0000|00|--\n--\n").unwrap_err(),
		LoadError::InvalidUtf8
	);
}

#[test]
fn rejects_invalid_shift_jis_without_bom() {
	assert_eq!(
		from_bytes(b"title=\x82\n--\n0000|00|--\n--\n").unwrap_err(),
		LoadError::InvalidShiftJis
	);
}

#[test]
fn header_unknown_keys_collected() {
	let h = from_bytes(minimal("mycustomkey=somevalue").as_bytes())
		.unwrap()
		.header;
	assert_eq!(h.unknown.get("mycustomkey"), Some(&"somevalue".to_owned()));
}

#[test]
fn header_crlf_lines() {
	let src = "title=CRLF Song\r\nartist=CRLF Artist\r\n--\r\n0000|00|--\r\n--\r\n";
	let h = from_bytes(src.as_bytes()).unwrap().header;
	assert_eq!(h.title, "CRLF Song");
	assert_eq!(h.artist, "CRLF Artist");
}

#[test]
fn header_comments_skipped() {
	let h = from_bytes(minimal("// comment\ntitle=Commented").as_bytes())
		.unwrap()
		.header;
	assert_eq!(h.title, "Commented");
	assert!(!h.unknown.contains_key("// comment"));
}

#[test]
fn header_value_with_equals_sign() {
	let h = from_bytes(minimal("title=1+1=2").as_bytes())
		.unwrap()
		.header;
	assert_eq!(h.title, "1+1=2");
}

#[test]
fn header_beat_default() {
	let h = from_bytes(minimal("").as_bytes()).unwrap().header;
	assert_eq!(h.beat, (4, 4));
}

#[test]
fn header_beat_parsed() {
	let h = from_bytes(minimal("beat=3/4").as_bytes()).unwrap().header;
	assert_eq!(h.beat, (3, 4));
}

#[test]
fn header_bpm_range() {
	let h = from_bytes(minimal("t=120-220").as_bytes()).unwrap().header;
	assert_eq!(h.bpm, "120-220");
	assert!((h.bpm_init - 120.0).abs() < f64::EPSILON);
}

#[test]
fn no_bar_line_error() {
	assert_eq!(
		from_bytes(b"title=foo\n").unwrap_err(),
		LoadError::NoBarLine
	);
}

// ── Chart rows ────────────────────────────────────────────────────────────────

#[test]
fn empty_row() {
	let row = first_row("--\n0000|00|--\n--\n");
	assert_eq!(row.bt, [BtNote::None; 4]);
	assert_eq!(row.fx, [FxNote::None; 2]);
	assert_eq!(row.laser, [LaserCell::None; 2]);
	assert!(row.spin.is_none());
}

#[test]
fn chip_bt_notes() {
	let row = first_row("--\n1111|00|--\n--\n");
	assert_eq!(row.bt, [BtNote::Chip; 4]);
}

#[test]
fn hold_bt_notes() {
	let row = first_row("--\n2222|00|--\n--\n");
	assert_eq!(row.bt, [BtNote::Hold; 4]);
}

#[test]
fn chip_fx_notes() {
	let row = first_row("--\n0000|22|--\n--\n");
	assert_eq!(row.fx, [FxNote::Chip; 2]);
}

#[test]
fn hold_fx_notes() {
	let row = first_row("--\n0000|11|--\n--\n");
	assert_eq!(row.fx, [FxNote::Hold; 2]);
}

#[test]
fn legacy_fx_chars_map_to_hold() {
	for ch in b"SVTWUGHKILJFPBQXADs" {
		let src = format!("--\n0000|{c}{c}|--\n--\n", c = *ch as char);
		let row = first_row(&src);
		assert_eq!(row.fx[0], FxNote::Hold, "char '{}'", *ch as char);
	}
}

#[test]
fn laser_none_and_continuation() {
	let row = first_row("--\n0000|00|--\n--\n");
	assert_eq!(row.laser, [LaserCell::None; 2]);

	let row = first_row("--\n0000|00|::\n--\n");
	assert_eq!(row.laser, [LaserCell::Continuation; 2]);
}

#[test]
fn laser_all_51_positions() {
	let chars = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmno";
	for (i, &c) in chars.iter().enumerate() {
		let src = format!("--\n0000|00|{c}-\n--\n", c = c as char);
		let row = first_row(&src);
		assert_eq!(
			row.laser[0],
			LaserCell::Position(i as u8),
			"char '{}' → pos {i}",
			c as char
		);
	}
}

#[test]
fn laser_position_extremes() {
	let row = first_row("--\n0000|00|0o\n--\n");
	assert_eq!(row.laser[0], LaserCell::Position(0));
	assert_eq!(row.laser[1], LaserCell::Position(50));
}

#[test]
fn malformed_unicode_spin_suffixes_do_not_panic() {
	for row in ["0000|00|\u{feff}:", "0000|00|--\u{feff}"] {
		let src = format!("\u{feff}--\n{row}\n--\n");
		assert!(from_bytes(src.as_bytes()).is_ok(), "row={row:?}");
	}
}

// ── Lane spin ─────────────────────────────────────────────────────────────────

#[test]
fn spin_normal_left() {
	let row = first_row("--\n0000|00|0-@(192\n--\n");
	let spin = row.spin.unwrap();
	assert_eq!(spin.kind, LaneSpinKind::NormalLeft);
	assert_eq!(spin.length, 192);
	assert!(spin.scale.is_none());
}

#[test]
fn spin_all_kinds() {
	for (suffix, expected) in [
		("@(192", LaneSpinKind::NormalLeft),
		("@)192", LaneSpinKind::NormalRight),
		("@<192", LaneSpinKind::HalfLeft),
		("@>192", LaneSpinKind::HalfRight),
		("S<192", LaneSpinKind::SwingLeft),
		("S>192", LaneSpinKind::SwingRight),
	] {
		let src = format!("--\n0000|00|0-{suffix}\n--\n");
		let spin = first_row(&src).spin.unwrap();
		assert_eq!(spin.kind, expected, "suffix={suffix}");
		assert_eq!(spin.length, 192);
	}
}

#[test]
fn spin_swing_all_params() {
	let row = first_row("--\n0000|00|0-S<192;500;3;1\n--\n");
	let spin = row.spin.unwrap();
	assert_eq!(spin.kind, LaneSpinKind::SwingLeft);
	assert_eq!(spin.length, 192);
	assert_eq!(spin.scale, Some(500));
	assert_eq!(spin.repetitions, Some(3));
	assert_eq!(spin.decay_order, Some(1));
}

#[test]
fn spin_swing_partial_params() {
	let row = first_row("--\n0000|00|0-S>192;250\n--\n");
	let spin = row.spin.unwrap();
	assert_eq!(spin.scale, Some(250));
	assert!(spin.repetitions.is_none());
}

// ── Body options ──────────────────────────────────────────────────────────────

fn body_events(src: &str) -> Vec<MeasureEvent> {
	from_bytes(src.as_bytes()).unwrap().measures[0]
		.events
		.clone()
}

#[test]
fn body_bpm_change() {
	let evts = body_events("--\nt=150.5\n0000|00|--\n--\n");
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::Bpm(150.5))));
}

#[test]
fn body_beat_change() {
	let evts = body_events("--\nbeat=3/8\n0000|00|--\n--\n");
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::Beat(3, 8))));
}

#[test]
fn body_fx_effects() {
	let evts = body_events("--\nfx-l=Retrigger;8\nfx-r=Gate;4\n0000|00|--\n--\n");
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::FxLeft(
		"Retrigger;8".to_owned()
	))));
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::FxRight(
		"Gate;4".to_owned()
	))));
}

#[test]
fn body_filtertype() {
	let evts = body_events("--\nfiltertype=hpf1\n0000|00|--\n--\n");
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::FilterType(
		"hpf1".to_owned()
	))));
}

#[test]
fn body_laserrange() {
	let evts = body_events("--\nlaserrange_l=2x\nlaserrange_r=2x\n0000|00|--\n--\n");
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::LaserRangeLeft)));
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::LaserRangeRight)));
}

#[test]
fn body_zoom_options() {
	let evts = body_events("--\nzoom_top=100\nzoom_bottom=-50\nzoom_side=200\n0000|00|--\n--\n");
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::ZoomTop(100))));
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::ZoomBottom(-50))));
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::ZoomSide(200))));
}

#[test]
fn body_tilt_named_values() {
	for (s, expected) in [
		("normal", TiltValue::Normal),
		("bigger", TiltValue::Bigger),
		("big", TiltValue::Bigger),
		("biggest", TiltValue::Biggest),
		("keep_normal", TiltValue::KeepNormal),
		("keep_bigger", TiltValue::KeepBigger),
		("keep", TiltValue::KeepBigger),
		("keep_biggest", TiltValue::KeepBiggest),
		("zero", TiltValue::Zero),
	] {
		let src = format!("--\ntilt={s}\n0000|00|--\n--\n");
		let evts = body_events(&src);
		assert!(
			evts.contains(&MeasureEvent::Option(BodyOption::Tilt(expected.clone()))),
			"tilt={s}"
		);
	}
}

#[test]
fn body_tilt_manual() {
	let evts = body_events("--\ntilt=1.5\n0000|00|--\n--\n");
	assert!(
		evts.contains(&MeasureEvent::Option(BodyOption::Tilt(TiltValue::Manual(
			1.5
		))))
	);
}

#[test]
fn body_unknown_option() {
	let evts = body_events("--\nfuturekey=futurevalue\n0000|00|--\n--\n");
	assert!(evts.contains(&MeasureEvent::Option(BodyOption::Unknown {
		key: "futurekey".to_owned(),
		value: "futurevalue".to_owned(),
	})));
}

// ── Definitions ───────────────────────────────────────────────────────────────

#[test]
fn definition_fx() {
	let src = "--\n0000|00|--\n--\n#define_fx LoFl type=Flanger;delay=80samples;depth=60samples\n";
	let chart = from_bytes(src.as_bytes()).unwrap();
	assert_eq!(chart.definitions.len(), 1);
	let def = &chart.definitions[0];
	assert_eq!(def.kind, DefinitionKind::Fx);
	assert_eq!(def.name, "LoFl");
	assert_eq!(def.value, "type=Flanger;delay=80samples;depth=60samples");
}

#[test]
fn definition_filter() {
	let src = "--\n0000|00|--\n--\n#define_filter TSTP type=TapeStop;trigger=off>on;speed=20%\n";
	let chart = from_bytes(src.as_bytes()).unwrap();
	assert_eq!(chart.definitions.len(), 1);
	let def = &chart.definitions[0];
	assert_eq!(def.kind, DefinitionKind::Filter);
	assert_eq!(def.name, "TSTP");
	assert_eq!(def.value, "type=TapeStop;trigger=off>on;speed=20%");
}

// ── Multiple measures ─────────────────────────────────────────────────────────

#[test]
fn multiple_measures_count() {
	let src = "--\n0000|00|--\n--\n1000|00|--\n--\n0001|00|--\n--\n";
	let chart = from_bytes(src.as_bytes()).unwrap();
	assert_eq!(chart.measures.len(), 3);
}

#[test]
fn options_in_specific_measure() {
	let src = "t=120\n--\n0000|00|--\n--\nt=180\n0000|00|--\n--\n";
	let chart = from_bytes(src.as_bytes()).unwrap();
	let m1_evts = &chart.measures[1].events;
	assert!(m1_evts.contains(&MeasureEvent::Option(BodyOption::Bpm(180.0))));
}

// ── Integration ───────────────────────────────────────────────────────────────

#[test]
fn real_ksh_frums() {
	let bytes = crate::test_utils::test_file_read("usc/mxm.ksh");
	let chart = from_bytes(&bytes).unwrap();
	assert!(!chart.header.title.is_empty(), "title should be non-empty");
	assert!(
		!chart.header.artist.is_empty(),
		"artist should be non-empty"
	);
	assert_eq!(chart.header.difficulty, Difficulty::Infinite);
	assert!(
		!chart.measures.is_empty(),
		"should have at least one measure"
	);
}
