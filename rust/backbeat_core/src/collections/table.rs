//! Tables are mappings of charts to difficulty levels. They are analogous to rating systems.

use std::io;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Assets, ChartId, ValidGamemodeIdentifier};

define_tags!(TableTags);
define_tags!(LevelTags);
define_tags!(TableFolderTags);
define_tags!(TableChartTags);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
	/// Display name.
	pub name: String,
	/// The symbol used to prefix level information with. If this is "L", then charts will be
	/// displayed as "L5", "L7+", etc.
	pub symbol: String,

	/// The single gamemode this table is organised around.
	pub gamemode: ValidGamemodeIdentifier,

	/// When this table was updated.
	pub updated: DateTime<Utc>,

	pub tags: TableTags,
	pub assets: Assets,

	/// The levels in this table, in the order they should be displayed.
	///
	/// It is an error for a chart to have a level that does not appear in the list of level
	/// definitions.
	///
	/// It is an error for the table to define the same level twice.
	pub levels: Vec<TableLevel>,

	/// Other than the implicit folders - "Level 5" and so on, what extra folders come with this table?
	///
	/// This is a useful way of defining collative folders, like Level 1-10 or so.
	pub folders: Vec<TableFolder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableLevel {
	pub level: String,
	pub tags: LevelTags,
	pub charts: Vec<TableChart>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableFolder {
	pub name: String,
	pub query: String,
	pub tags: TableFolderTags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TableChart {
	pub id: ChartId,
	pub desc: String,
	pub tags: TableChartTags,
}

impl Table {
	pub const EXTENSION: &str = "bbtable";

	pub fn charts(&self) -> impl Iterator<Item = (&TableLevel, &TableChart)> + '_ {
		self.levels
			.iter()
			.flat_map(|level| level.charts.iter().map(move |chart| (level, chart)))
	}

	pub fn chart_count(&self) -> usize {
		self.levels.iter().map(|level| level.charts.len()).sum()
	}

	/// Take a folder expression in tinyfilter syntax and evaluate it on each chart in this table.
	pub fn evaluate_folder_expr(&self, expression: &str) -> Vec<(&TableLevel, &TableChart)> {
		self.charts()
			.filter(|(level, chart)| {
				let context = Self::folder_context(level, chart);
				tinyfilter::evaluate(expression, &context).unwrap_or(false)
			})
			.collect()
	}

	fn folder_context(level: &TableLevel, chart: &TableChart) -> tinyfilter::Context {
		tinyfilter::Context::from_values([
			("level".into(), tinyfilter::Value::string(&level.level)),
			("level_tags".into(), Self::tag_value(&level.tags)),
			("tags".into(), Self::tag_value(&chart.tags)),
		])
	}

	fn tag_value(tags: &std::collections::HashMap<String, String>) -> tinyfilter::Value {
		tinyfilter::Value::map(
			tags.iter()
				.map(|(key, value)| (key.clone(), tinyfilter::Value::string(value))),
		)
	}

	pub fn to_json(&self) -> Vec<u8> {
		serde_json::to_vec_pretty(self).expect("must ser")
	}

	pub fn from_json(s: &[u8]) -> io::Result<Self> {
		Ok(serde_json::from_slice(s)?)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ValidGamemodeIdentifier;

	#[test]
	fn charts_iterator_pairs_each_chart_with_its_level() {
		let table = Table {
			name: "Test".into(),
			symbol: "★".into(),
			gamemode: ValidGamemodeIdentifier::new("bms-7k").unwrap(),
			updated: Utc::now(),
			tags: Default::default(),
			assets: Default::default(),
			levels: vec![
				TableLevel {
					level: "1".into(),
					tags: Default::default(),
					charts: vec![TableChart {
						id: "md5/abc".parse().unwrap(),
						desc: "Hey".into(),
						tags: Default::default(),
					}],
				},
				TableLevel {
					level: "2".into(),
					tags: Default::default(),
					charts: vec![TableChart {
						id: "md5/def".parse().unwrap(),
						desc: "hey".into(),
						tags: Default::default(),
					}],
				},
			],
			folders: Vec::new(),
		};

		let pairs = table.charts().collect::<Vec<_>>();
		assert_eq!(pairs.len(), 2);
		assert_eq!(pairs[0].0.level, "1");
		assert_eq!(pairs[1].0.level, "2");
		assert_eq!(pairs[0].1.id.to_string(), "md5/abc");
		assert_eq!(pairs[1].1.id.to_string(), "md5/def");
	}

	#[test]
	fn folder_expression_filters_charts_using_string_levels_and_chart_tags() {
		let table = Table {
			name: "Test".into(),
			symbol: "★".into(),
			gamemode: ValidGamemodeIdentifier::new("bms-7k").unwrap(),
			updated: Utc::now(),
			tags: Default::default(),
			assets: Default::default(),
			levels: vec![
				TableLevel {
					level: "10".into(),
					tags: Default::default(),
					charts: vec![TableChart {
						id: "md5/abc".parse().unwrap(),
						desc: "Included".into(),
						tags: std::collections::HashMap::from([("kind".into(), "included".into())])
							.into(),
					}],
				},
				TableLevel {
					level: "2".into(),
					tags: Default::default(),
					charts: vec![TableChart {
						id: "md5/def".parse().unwrap(),
						desc: "Excluded".into(),
						tags: Default::default(),
					}],
				},
			],
			folders: Vec::new(),
		};

		let matches = table.evaluate_folder_expr(r#"level == "10" and tags.kind == "included""#);

		assert_eq!(matches.len(), 1);
		assert_eq!(matches[0].0.level, "10");
		assert_eq!(matches[0].1.desc, "Included");
	}

	#[test]
	fn invalid_or_non_boolean_folder_expressions_match_no_charts() {
		let table = Table {
			name: "Test".into(),
			symbol: "★".into(),
			gamemode: ValidGamemodeIdentifier::new("bms-7k").unwrap(),
			updated: Utc::now(),
			tags: Default::default(),
			assets: Default::default(),
			levels: vec![TableLevel {
				level: "10".into(),
				tags: Default::default(),
				charts: vec![TableChart {
					id: "md5/abc".parse().unwrap(),
					desc: "Chart".into(),
					tags: Default::default(),
				}],
			}],
			folders: Vec::new(),
		};

		assert!(table.evaluate_folder_expr("level ==").is_empty());
		assert!(table.evaluate_folder_expr("level").is_empty());
		assert!(table.evaluate_folder_expr("missing == 1").is_empty());
	}
}
