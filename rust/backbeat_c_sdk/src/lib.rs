#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![doc = include_str!("../README.md")]
#![allow(
	unsafe_code,
	non_camel_case_types,
	clippy::missing_safety_doc,
	clippy::not_unsafe_ptr_arg_deref,
	rust_2024_compatibility,
	clippy::needless_pass_by_value
)]

mod bb;
mod course;
mod error;
mod functions;
mod pack;
mod sqlite;
mod store;
mod table;
mod types;
mod util;
mod version;

pub use bb::*;
pub use course::*;
pub use error::*;
pub use functions::*;
pub use pack::*;
pub use sqlite::*;
pub use store::*;
pub use table::*;
pub use types::{
	bkb_asset, bkb_bb, bkb_bundle_search_chart, bkb_bundle_search_result, bkb_bytes,
	bkb_collection_metadata, bkb_collection_metadata_list, bkb_course, bkb_course_chart, bkb_pack,
	bkb_pack_bundle, bkb_server_config, bkb_store, bkb_str, bkb_string, bkb_table, bkb_table_chart,
	bkb_table_folder, bkb_table_level, bkb_tag, bkb_timestamp,
};
pub use version::*;
