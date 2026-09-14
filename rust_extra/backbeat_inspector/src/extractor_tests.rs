use backbeat_core::ChartFilename;
use pretty_assertions::assert_eq;

use crate::{extract, parsed::ParsedChart};

fn parse(extension: &str, bytes: &[u8]) -> ParsedChart {
	ParsedChart::parse(bytes, &filename(&format!("chart.{extension}"))).expect("parse")
}

fn filename(value: &str) -> ChartFilename {
	ChartFilename::from_path(value).expect("valid filename")
}

#[test]
fn sm_banner_and_song_fields() {
	let bytes = b"#TITLE:My Song;\n\
		#ARTIST:Cool Artist;\n\
		#SUBTITLE:remix;\n\
		#BANNER:bn.png;\n\
		#BACKGROUND:bg.jpg;\n\
		#CDTITLE:sig.png;\n\
		#JACKET:jk.png;\n\
		#MUSIC:song.ogg;\n\
		#REALBANNER:real-bn.png;\n\
		#REALBACKGROUND:real-bg.jpg;\n\
		#REALCDTITLE:real-sig.png;\n\
		#REALJACKET:real-jk.png;\n\
		#REALMUSIC:real-song.ogg;\n\
		#SAMPLESTART:14.801;\n\
		#SAMPLELENGTH:12.000;\n\
		#CREDIT:Charter;\n\
		#BPMS:0.000=140.000;\n\
		#NOTES:\n\
		     dance-single:\n\
		     Author:\n\
		     Hard:\n\
		     13:\n\
		     0,0,0,0,0:\n\
		0000\n\
		;\n";
	let e = extract(&parse("sm", bytes), &filename("chart.sm"));
	assert_eq!(e.song.title.as_deref(), Some("My Song"));
	assert_eq!(e.song.artist.as_deref(), Some("Cool Artist"));
	assert_eq!(e.song.subtitle, None);
	assert_eq!(e.chart.charter_note.as_deref(), Some("remix"));
	assert_eq!(e.chart.credit.as_deref(), Some("Charter"));
	assert_eq!(e.chart.difficulty_label.as_deref(), Some("Hard"));
	assert_eq!(e.chart.level.as_deref(), Some("13"));
	assert_eq!(e.chart.chart_name.as_deref(), Some("Hard 13"));
	assert_eq!(e.assets.banner.as_deref(), Some("real-bn.png"));
	assert_eq!(e.assets.background.as_deref(), Some("real-bg.jpg"));
	assert_eq!(e.assets.jacket.as_deref(), Some("real-jk.png"));
	assert_eq!(e.assets.signature.as_deref(), Some("real-sig.png"));
	assert_eq!(e.assets.music.as_deref(), Some("real-song.ogg"));
	assert_eq!(e.song.preview, Some(14_801));
}

#[test]
fn descriptions_come_from_the_extract() {
	let mut extract = extract(
		&parse(
			"sm",
			b"#TITLE:Friday ~Splittercore Remix~;\n\
		  #ARTIST:Hayoreo;\n\
		  #NOTES:dance-single::Challenge:15:0,0,0,0,0:0000;\n",
		),
		&filename("chart.sm"),
	);
	assert_eq!(
		crate::describe(&extract).as_str(),
		"Hayoreo - Friday ~Splittercore Remix~ (Challenge 15)"
	);

	extract.chart.credit = Some("IcyWorld".to_owned());
	assert_eq!(
		crate::describe(&extract).as_str(),
		"Hayoreo - Friday ~Splittercore Remix~ (IcyWorld's Challenge 15)"
	);

	extract.chart.chart_name = Some("Challenge".to_owned());
	assert_eq!(
		crate::describe(&extract).as_str(),
		"Hayoreo - Friday ~Splittercore Remix~ (IcyWorld's Challenge)"
	);

	extract.chart.credit = None;
	assert_eq!(
		crate::describe(&extract).as_str(),
		"Hayoreo - Friday ~Splittercore Remix~ (Challenge)"
	);

	extract.song.artist = Some("  ".to_owned());
	extract.song.title = None;
	extract.chart.chart_name = None;
	assert_eq!(
		crate::describe(&extract).as_str(),
		"Unknown Artist - Unknown Title"
	);
}

#[test]
fn dwi_uses_chart_metadata_in_descriptions() {
	let dwi_extract = extract(
		&parse(
			"dwi",
			b"#TITLE:DWI Song;\n\
			  #ARTIST:DWI Artist;\n\
			  #CREDIT:Charter;\n\
			  #CHARTNAME:Custom Maniac;\n\
			  #SINGLE:MANIAC:9:0011;\n",
		),
		&filename("chart.dwi"),
	);
	assert_eq!(dwi_extract.chart.credit.as_deref(), Some("Charter"));
	assert_eq!(
		dwi_extract.chart.chart_name.as_deref(),
		Some("Custom Maniac")
	);
	assert_eq!(
		crate::describe(&dwi_extract).as_str(),
		"DWI Artist - DWI Song (Charter's Custom Maniac)"
	);

	let extract = extract(
		&parse(
			"dwi",
			b"#TITLE:DWI Song;#ARTIST:DWI Artist;#SINGLE:MANIAC:9:0011;",
		),
		&filename("chart.dwi"),
	);
	assert_eq!(extract.chart.chart_name.as_deref(), Some("Hard"));
	assert_eq!(
		crate::describe(&extract).as_str(),
		"DWI Artist - DWI Song (Hard)"
	);
}

#[test]
fn bms_has_no_music_uses_raw_image_tags() {
	let bytes = b"#TITLE song\n#ARTIST artist\n#BANNER bn.png\n#BACKBMP bg.bmp\n#STAGEFILE load.png\n#PLAYLEVEL 12\n#SUBARTIST charter\n";
	let e = extract(&parse("bms", bytes), &filename("chart.bms"));
	assert_eq!(e.song.title.as_deref(), Some("song"));
	assert_eq!(e.assets.music, None);
	assert_eq!(e.assets.banner.as_deref(), Some("bn.png"));
	assert_eq!(e.assets.background.as_deref(), Some("bg.bmp"));
	assert_eq!(e.assets.signature, None);
	assert_eq!(e.chart.level.as_deref(), Some("12"));
	assert_eq!(e.chart.credit.as_deref(), Some("charter"));
}

#[test]
fn bmson_has_no_music() {
	let bytes = br#"{
		"version":"1.0.0",
		"info":{
			"title":"T",
			"artist":"A",
			"init_bpm":130.0,
			"banner_image":"bn.png",
			"back_image":"bg.png",
			"level":10,
			"chart_name":"HYPER",
			"mode_hint":"beat-7k"
		},
		"sound_channels":[{"name":"a.wav","notes":[]}]
	}"#;
	let e = extract(&parse("bmson", bytes), &filename("chart.bmson"));
	assert_eq!(e.assets.music, None);
	assert_eq!(e.assets.banner.as_deref(), Some("bn.png"));
	assert_eq!(e.assets.background.as_deref(), Some("bg.png"));
	assert_eq!(e.assets.signature, None);
	assert_eq!(e.chart.difficulty_label.as_deref(), Some("HYPER"));
	assert_eq!(e.chart.level.as_deref(), Some("10"));
}

#[test]
fn ssc_song_and_primary_chart_meta() {
	let bytes = b"#TITLE:Test Song;\n\
		#ARTIST:Test Artist;\n\
		#SUBTITLE:remix;\n\
		#GENRE:Trance;\n\
		#TITLETRANSLIT:Test;\n\
		#ARTISTTRANSLIT:Tester;\n\
		#BANNER:bn.png;\n\
		#BACKGROUND:bg.jpg;\n\
		#CDTITLE:sig.png;\n\
		#JACKET:jk.png;\n\
		#MUSIC:song.ogg;\n\
		#REALBANNER:real-bn.png;\n\
		#REALBACKGROUND:real-bg.jpg;\n\
		#REALCDTITLE:real-sig.png;\n\
		#REALJACKET:real-jk.png;\n\
		#REALMUSIC:real-song.ogg;\n\
		#SAMPLESTART:9.500;\n\
		#SAMPLELENGTH:8.000;\n\
		#BPMS:0.000=120.000;\n\
		#NOTEDATA:;\n\
		#STEPSTYPE:dance-single;\n\
		#DESCRIPTION:Basic;\n\
		#DIFFICULTY:easy;\n\
		#METER:3;\n\
		#CREDIT:Someone;\n\
		#NOTES:\n0000\n;\n";
	let e = extract(&parse("ssc", bytes), &filename("chart.ssc"));
	assert_eq!(e.song.title.as_deref(), Some("Test Song"));
	assert_eq!(e.song.artist.as_deref(), Some("Test Artist"));
	assert_eq!(e.song.subtitle.as_deref(), Some("remix"));
	assert_eq!(e.song.genre.as_deref(), Some("Trance"));
	assert_eq!(e.song.title_translit.as_deref(), Some("Test"));
	assert_eq!(e.song.artist_translit.as_deref(), Some("Tester"));
	assert_eq!(e.assets.music.as_deref(), Some("real-song.ogg"));
	assert_eq!(e.song.preview, Some(9_500));
	assert_eq!(e.assets.banner.as_deref(), Some("real-bn.png"));
	assert_eq!(e.assets.background.as_deref(), Some("real-bg.jpg"));
	assert_eq!(e.assets.jacket.as_deref(), Some("real-jk.png"));
	assert_eq!(e.assets.signature.as_deref(), Some("real-sig.png"));
	assert_eq!(e.chart.credit.as_deref(), Some("Someone"));
	assert_eq!(e.chart.difficulty_label.as_deref(), Some("Easy"));
	assert_eq!(e.chart.level.as_deref(), Some("3"));
	assert_eq!(e.chart.chart_name.as_deref(), Some("Basic"));
}

#[test]
fn ksh_jacket_and_bg_not_used_as_background() {
	let bytes = b"title=My Song\nartist=Cool Artist\neffect=Charter\njacket=cover.jpg\nbg=desert;custom_bg.png\nm=song.ogg\npo=93833\nplength=15000\ndifficulty=challenge\nlevel=10\nt=180\n--\n--\n";
	let e = extract(&parse("ksh", bytes), &filename("adv.ksh"));
	assert_eq!(e.song.title.as_deref(), Some("My Song"));
	assert_eq!(e.chart.credit.as_deref(), Some("Charter"));
	assert_eq!(e.chart.chart_name.as_deref(), Some("ADV"));
	assert_eq!(
		crate::describe(&e).as_str(),
		"Cool Artist - My Song (Charter's ADV)"
	);
	assert_eq!(e.assets.jacket.as_deref(), Some("cover.jpg"));
	assert_eq!(e.assets.background, None);
	assert_eq!(e.assets.music.as_deref(), Some("song.ogg"));
	assert_eq!(e.song.preview, Some(93_833));
	assert_eq!(e.chart.difficulty_label.as_deref(), Some("Challenge"));
}

#[test]
fn ksh_descriptions_use_filename_difficulties() {
	let chart = parse(
		"ksh",
		b"title=My Song\nartist=Cool Artist\neffect=Charter\n--\n0000|00|--\n--\n",
	);

	for (path, expected) in [
		("bsc.ksh", "BSC"),
		("adv.ksh", "ADV"),
		("exh.ksh", "EXH"),
		("inf.ksh", "INF"),
		("grv.ksh", "GRV"),
		("hvn.ksh", "HVN"),
		("vvd.ksh", "VVD"),
		("xcd.ksh", "XCD"),
		("ult.ksh", "ULT"),
		("mxm.ksh", "MXM"),
		("custom.ksh", "custom"),
	] {
		let extract = extract(&chart, &filename(path));
		assert_eq!(extract.chart.chart_name.as_deref(), Some(expected));
		assert_eq!(
			crate::describe(&extract).as_str(),
			format!("Cool Artist - My Song (Charter's {expected})")
		);
	}
}

#[test]
fn kson_basic() {
	let bytes = br#"{
		"format_version": 1,
		"meta": {
			"title": "My Song",
			"artist": "Cool Artist",
			"chart_author": "Charter",
			"difficulty": 1,
			"level": 10,
			"disp_bpm": "180",
			"jacket_filename": "jk.png"
		},
		"beat": { "bpm": [[0, 180.0]] },
		"audio": { "bgm": {
			"filename": "song.ogg",
			"offset": -100,
			"preview": { "offset": 93833, "duration": 15000 }
		} },
		"bg": {
			"legacy": {
				"bg": [{ "filename": "desert" }, { "filename": "custom_bg.png" }]
			}
		}
	}"#;
	let e = extract(&parse("kson", bytes), &filename("chart.kson"));
	assert_eq!(e.song.title.as_deref(), Some("My Song"));
	assert_eq!(e.assets.music.as_deref(), Some("song.ogg"));
	assert_eq!(e.song.preview, Some(93_833));
	assert_eq!(e.assets.jacket.as_deref(), Some("jk.png"));
	assert_eq!(e.assets.background, None);
	assert_eq!(e.assets.signature, None);
	assert_eq!(e.chart.credit.as_deref(), Some("Charter"));
}
