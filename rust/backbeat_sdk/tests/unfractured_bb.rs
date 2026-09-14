#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

//! An unfractured multi-chart source bundled as one `.bb` is corrupt:
//! a Backbeat file must wrap exactly one playable chart.

use backbeat_core::ChartDesc;
use backbeat_core::ChartFilename;
use backbeat_core::{Assets, BackbeatFile, ChartData};
use backbeat_inspector::inspect_bundle;

mod support;
use support::new_test_store;

fn unfractured_sm_bb() -> BackbeatFile {
	let chart = b"#TITLE:Test;\n\
		#BPMS:0=120;\n\
		#NOTES:dance-single:A:Easy:1:0:0000;\n\
		#NOTES:dance-single:B:Hard:9:0:0000;\n";
	BackbeatFile {
		filename: ChartFilename::from_path("song.sm").unwrap(),
		assets: Assets::default(),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(chart).unwrap(),
	}
}

#[test]
fn import_bb_accepts_unfractured_multi_chart_sm() {
	let (_tmp, store) = new_test_store("bb_unfractured_sm");

	let bb = unfractured_sm_bb();
	let notes_count = String::from_utf8_lossy(&bb.chart.decompress())
		.matches("#NOTES")
		.count();
	assert_eq!(
		notes_count, 2,
		"fixture must embed an unfractured multi-chart .sm"
	);

	assert!(inspect_bundle(&bb).is_err());

	let bundle_id = store
		.import_bundle(&bb)
		.expect("store import does not inspect chart contents");
	assert!(store.has_bundle(bundle_id).unwrap());
}
