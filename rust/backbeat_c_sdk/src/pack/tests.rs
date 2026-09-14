use std::ptr;
use std::slice;

use backbeat_core::Pack;
use backbeat_sdk::collections::{PackContents, PackContentsBundle};

use super::*;
use crate::types::{bkb_str, bkb_tag};

const PACK: &[u8] = br#"{
  "name": "Test Pack",
  "gamemode": "dance-single",
  "updated": "2026-01-01T00:00:00Z",
  "tags": {"genre": "dance"},
  "assets": {"banner.png": "0000000000000000000000000000000000000000000000000000000000000000"},
  "bundles": [{
    "id": "b-1111111111111111111111111111111111111111111111111111111111111111",
    "desc": "First song",
    "tags": {"folder": "one"}
  }]
}"#;

unsafe fn str_view<'a>(value: bkb_str) -> &'a str {
	let bytes = unsafe { slice::from_raw_parts(value.ptr.cast(), value.len) };
	str::from_utf8(bytes).unwrap()
}

unsafe fn tags<'a>(ptr: *const bkb_tag, len: usize) -> &'a [bkb_tag] {
	unsafe { slice::from_raw_parts(ptr, len) }
}

#[test]
fn fields_are_directly_traversable() {
	let pack = Pack::from_json(PACK).unwrap();
	let pack = bkb_pack::alloc(PackContents {
		name: pack.name,
		gamemode: pack.gamemode,
		updated: pack.updated,
		tags: pack.tags,
		assets: pack.assets,
		bundles: pack
			.bundles
			.into_iter()
			.map(|bundle| PackContentsBundle {
				id: bundle.id,
				desc: bundle.desc,
				tags: bundle.tags,
				installed: true,
			})
			.collect(),
	});
	assert_eq!(unsafe { str_view((*pack).name) }, "Test Pack");
	assert_eq!(unsafe { str_view((*pack).gamemode) }, "dance-single");
	assert_eq!(unsafe { (*pack).updated.nanoseconds }, 0);
	assert_eq!(unsafe { (*pack).tags_len }, 1);
	let tags = unsafe { tags((*pack).tags, (*pack).tags_len) };
	assert_eq!(unsafe { str_view(tags[0].key) }, "genre");
	assert_eq!(unsafe { str_view(tags[0].value) }, "dance");
	assert_eq!(unsafe { (*pack).assets_len }, 1);
	let asset = unsafe { &*(*pack).assets };
	assert_eq!(unsafe { str_view(asset.path) }, "banner.png");
	assert_eq!(unsafe { (*pack).bundles_len }, 1);
	let bundle = unsafe { &*(*pack).bundles };
	assert_eq!(unsafe { str_view(bundle.desc) }, "First song");
	assert_eq!(bundle.tags_len, 1);
	assert!(bundle.installed);
	unsafe { bkb_pack_free(pack) };
}

#[test]
fn free_accepts_null() {
	unsafe { bkb_pack_free(ptr::null_mut()) };
}
