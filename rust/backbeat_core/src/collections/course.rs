//! Courses are ordered sequences of charts.

use std::io;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Assets, ChartId, ValidGamemodeIdentifier};

define_tags!(CourseTags);
define_tags!(CourseChartTags);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Course {
	pub name: String,
	pub updated: DateTime<Utc>,
	/// The single gamemode charts in this course are played as.
	pub gamemode: ValidGamemodeIdentifier,
	pub tags: CourseTags,
	pub assets: Assets,
	pub charts: Vec<CourseChart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseChart {
	pub id: ChartId,
	pub desc: String,
	pub tags: CourseChartTags,
}

impl Course {
	pub const EXTENSION: &str = "bbcourse";

	pub fn to_json(&self) -> Vec<u8> {
		serde_json::to_vec_pretty(self).expect("must ser")
	}

	pub fn from_json(s: &[u8]) -> io::Result<Self> {
		Ok(serde_json::from_slice(s)?)
	}
}
