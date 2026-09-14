use std::ptr;
use std::slice;

use backbeat_core::Course;
use backbeat_sdk::collections::{CourseContents, CourseContentsChart};

use super::*;
use crate::types::bkb_str;

const COURSE: &[u8] = br#"{
  "name": "Test Course",
  "updated": "2026-01-01T00:00:00Z",
  "gamemode": "dance-single",
  "tags": {"grade": "first"},
  "assets": {},
  "charts": [{
    "id": "md5/abc",
    "desc": "Opening chart",
    "tags": {"lives": "4"}
  }]
}"#;

unsafe fn str_view<'a>(value: bkb_str) -> &'a str {
	let bytes = unsafe { slice::from_raw_parts(value.ptr.cast(), value.len) };
	str::from_utf8(bytes).unwrap()
}

#[test]
fn fields_are_directly_traversable() {
	let definition = Course::from_json(COURSE).unwrap();
	let course = bkb_course::alloc(CourseContents {
		name: definition.name,
		updated: definition.updated,
		gamemode: definition.gamemode,
		tags: definition.tags,
		assets: definition.assets,
		charts: definition
			.charts
			.into_iter()
			.map(|chart| CourseContentsChart {
				id: chart.id,
				desc: chart.desc,
				tags: chart.tags,
				bundle_id: None,
			})
			.collect(),
	});
	assert_eq!(unsafe { str_view((*course).name) }, "Test Course");
	assert_eq!(unsafe { str_view((*course).gamemode) }, "dance-single");
	assert_eq!(unsafe { (*course).updated.nanoseconds }, 0);
	assert_eq!(unsafe { (*course).tags_len }, 1);
	assert_eq!(unsafe { (*course).charts_len }, 1);
	let chart = unsafe { &*(*course).charts };
	assert_eq!(unsafe { str_view(chart.id) }, "md5/abc");
	assert_eq!(unsafe { str_view(chart.desc) }, "Opening chart");
	assert_eq!(chart.tags_len, 1);
	assert!(chart.bundle_id.ptr.is_null());
	assert_eq!(chart.bundle_id.len, 0);
	unsafe { bkb_course_free(course) };
}

#[test]
fn free_accepts_null() {
	unsafe { bkb_course_free(ptr::null_mut()) };
}
