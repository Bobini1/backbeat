//! Backbeat collection files; for arranging and representing content.

macro_rules! define_tags {
	($name:ident) => {
		#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
		#[serde(transparent)]
		pub struct $name(pub ::std::collections::HashMap<String, String>);

		impl $name {
			pub fn is_empty(this: &Self) -> bool {
				this.0.is_empty()
			}
		}

		impl ::std::ops::Deref for $name {
			type Target = ::std::collections::HashMap<String, String>;

			fn deref(&self) -> &Self::Target {
				&self.0
			}
		}

		impl ::std::ops::DerefMut for $name {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self.0
			}
		}

		impl From<::std::collections::HashMap<String, String>> for $name {
			fn from(tags: ::std::collections::HashMap<String, String>) -> Self {
				Self(tags)
			}
		}
	};
}

pub mod course;
pub mod gamemode;
pub mod pack;
pub mod table;

pub use self::course::{Course, CourseChart, CourseChartTags, CourseTags};
pub use self::gamemode::{ValidGamemodeIdentifier, ValidGamemodeIdentifierError};
pub use self::pack::{Pack, PackBundle, PackBundleTags, PackTags};
pub use self::table::{
	LevelTags, Table, TableChart, TableChartTags, TableFolder, TableFolderTags, TableLevel,
	TableTags,
};

use std::io;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CollectionKind {
	Table,
	Course,
	Pack,
}

impl CollectionKind {
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Table => "table",
			Self::Course => "course",
			Self::Pack => "pack",
		}
	}

	pub const fn extension(&self) -> &'static str {
		match self {
			Self::Table => Table::EXTENSION,
			Self::Course => Course::EXTENSION,
			Self::Pack => Pack::EXTENSION,
		}
	}
}

impl std::fmt::Display for CollectionKind {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_str())
	}
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectionHeader {
	pub timestamp: DateTime<Utc>,
	pub kind: CollectionKind,
}

impl CollectionHeader {
	pub fn from_data(data: &[u8], kind: CollectionKind) -> io::Result<Self> {
		let last_modified = match kind {
			CollectionKind::Table => Table::from_json(data)?.updated,
			CollectionKind::Course => Course::from_json(data)?.updated,
			CollectionKind::Pack => Pack::from_json(data)?.updated,
		};
		Ok(Self {
			timestamp: last_modified,
			kind,
		})
	}

	pub fn data_filename(kind: CollectionKind) -> String {
		format!("data.{}", kind.extension())
	}

	pub fn where_to_look(&self, base_url: &str) -> String {
		format!("{base_url}/{}", Self::data_filename(self.kind))
	}

	pub fn to_json(&self) -> Vec<u8> {
		serde_json::to_vec_pretty(self).expect("must ser")
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn collection_header_rejects_unknown_fields() {
		let json = br#"{
			"timestamp": "2026-09-14T12:00:00Z",
			"kind": "table",
			"unknown": true
		}"#;

		assert!(serde_json::from_slice::<CollectionHeader>(json).is_err());
	}
}
