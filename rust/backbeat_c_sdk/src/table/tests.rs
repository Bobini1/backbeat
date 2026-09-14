use std::ptr;
use std::slice;

use backbeat_core::Table;
use backbeat_sdk::collections::{
	TableContents, TableContentsChart, TableContentsFolder, TableContentsLevel,
};

use super::*;
use crate::error::{BKB_ERR_NULL_ARG, BKB_OK};
use crate::types::bkb_str;

const TABLE: &[u8] = br#"{
  "name": "Test Table",
  "symbol": "L",
  "gamemode": "dance-single",
  "updated": "2026-01-01T00:00:00Z",
  "tags": {"source": "community"},
  "assets": {"banner.png": "0000000000000000000000000000000000000000000000000000000000000000"},
  "levels": [
    {
      "level": "10",
      "tags": {"group": "high"},
      "charts": [
        {
          "id": "md5/abc",
          "desc": "Included",
          "tags": {"kind": "included"}
        }
      ]
    },
    {
      "level": "2",
      "tags": {},
      "charts": [
        {
          "id": "md5/def",
          "desc": "Excluded",
          "tags": {}
        }
      ]
    }
  ],
  "folders": [{
    "name": "Featured",
    "query": "tags.kind == \"included\"",
    "tags": {"colour": "red"}
  }]
}"#;

unsafe fn str_view<'a>(value: bkb_str) -> &'a str {
	let bytes = unsafe { slice::from_raw_parts(value.ptr.cast(), value.len) };
	str::from_utf8(bytes).unwrap()
}

#[test]
fn methods_work() {
	let definition = Table::from_json(TABLE).unwrap();
	let bundle_id = "b-1111111111111111111111111111111111111111111111111111111111111111"
		.parse()
		.unwrap();
	let levels = definition
		.levels
		.into_iter()
		.enumerate()
		.map(|(level_index, level)| TableContentsLevel {
			level: level.level,
			tags: level.tags,
			charts: level
				.charts
				.into_iter()
				.map(|chart| TableContentsChart {
					id: chart.id,
					desc: chart.desc,
					tags: chart.tags,
					bundle_id: (level_index == 0).then_some(bundle_id),
				})
				.collect(),
		})
		.collect::<Vec<_>>();
	let featured_chart = levels[0].charts[0].clone();
	let folders = definition
		.folders
		.into_iter()
		.map(|folder| TableContentsFolder {
			name: folder.name,
			query: folder.query,
			tags: folder.tags,
			charts: vec![featured_chart.clone()],
		})
		.collect();
	let table = bkb_table::alloc(TableContents {
		name: definition.name,
		symbol: definition.symbol,
		gamemode: definition.gamemode,
		updated: definition.updated,
		tags: definition.tags,
		assets: definition.assets,
		levels,
		folders,
	});
	assert_eq!(unsafe { str_view((*table).name) }, "Test Table");
	assert_eq!(unsafe { str_view((*table).symbol) }, "L");
	assert_eq!(unsafe { str_view((*table).gamemode) }, "dance-single");
	assert_eq!(unsafe { (*table).updated.nanoseconds }, 0);
	assert_eq!(unsafe { (*table).tags_len }, 1);
	assert_eq!(unsafe { (*table).assets_len }, 1);
	assert_eq!(unsafe { (*table).levels_len }, 2);
	assert_eq!(unsafe { (*table).folders_len }, 1);
	let folder = unsafe { &*(*table).folders };
	assert_eq!(unsafe { str_view(folder.name) }, "Featured");
	assert_eq!(folder.tags_len, 1);
	assert_eq!(folder.charts_len, 1);
	let included = unsafe { &*folder.charts };
	assert_eq!(unsafe { str_view(included.desc) }, "Included");
	assert!(!included.bundle_id.ptr.is_null());
	let excluded = unsafe { &*(*(*table).levels.add(1)).charts };
	assert!(excluded.bundle_id.ptr.is_null());

	let mut count = 0;
	assert_eq!(unsafe { bkb_table_chart_count(table, &mut count) }, BKB_OK);
	assert_eq!(count, 2);

	unsafe { bkb_table_free(table) };
}

#[test]
fn null_arguments_are_rejected() {
	assert_eq!(
		unsafe { bkb_table_chart_count(ptr::null(), ptr::null_mut()) },
		BKB_ERR_NULL_ARG
	);
	unsafe { bkb_table_free(ptr::null_mut()) };
}
