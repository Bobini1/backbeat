//! Listing and searching charts in the store catalogue.

use backbeat_core::BundleId;

use crate::Result;
use crate::store::Backbeat;
use crate::util::BLOCK;

/// One chart bundle in the store.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChartEntry {
	pub bundle_id: BundleId,
	/// Human-readable label derived from extracted metadata at import time.
	pub description: String,
	pub extension: Option<String>,
}

/// One page of [`Backbeat::search_bundles`] results.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BundleSearchResult {
	pub charts: Vec<ChartEntry>,
	pub has_more: bool,
	/// Total number of bundles matching the query, across all pages.
	pub total: u64,
}

pub(crate) fn list_bundles_internal(
	store: &Backbeat,
	offset: u64,
	limit: u32,
	extensions: &[String],
) -> Result<BundleSearchResult> {
	let limit = i64::from(limit);
	let offset = offset.cast_signed();

	if !extensions.is_empty() {
		let extensions = serde_json::to_string(extensions)
			.expect("a list of strings can always be serialized as JSON");
		let rows = BLOCK(
			sqlx::query!(
				"SELECT b.id AS \"id: backbeat_core::BundleId\", b.description AS description, \
				 b.extension AS \"extension?: String\" \
					 FROM bundle b \
					 WHERE b.extension IN (SELECT value FROM json_each(?1)) \
					 ORDER BY COALESCE(b.description, ''), b.id \
					 LIMIT ?2 OFFSET ?3",
				&extensions,
				limit,
				offset,
			)
			.fetch_all(&store.pool),
		)?;
		let total: i64 = BLOCK(
			sqlx::query_scalar!(
				"SELECT COUNT(*) FROM bundle b \
				 WHERE b.extension IN (SELECT value FROM json_each(?1))",
				&extensions,
			)
			.fetch_one(&store.pool),
		)?;
		let charts: Vec<ChartEntry> = rows
			.into_iter()
			.map(|row| ChartEntry {
				bundle_id: row.id,
				description: row.description,
				extension: row.extension,
			})
			.collect();
		return Ok(BundleSearchResult {
			has_more: offset + (charts.len() as i64) < total,
			charts,
			total: total.max(0).cast_unsigned(),
		});
	}

	let rows = BLOCK(
		sqlx::query!(
			"SELECT id AS \"id: backbeat_core::BundleId\", description, \
			 extension AS \"extension?: String\" FROM bundle \
				 ORDER BY COALESCE(description, ''), id \
				 LIMIT ?1 OFFSET ?2",
			limit,
			offset,
		)
		.fetch_all(&store.pool),
	)?;

	let total: i64 =
		BLOCK(sqlx::query_scalar!("SELECT COUNT(*) FROM bundle").fetch_one(&store.pool))?;

	let charts: Vec<ChartEntry> = rows
		.into_iter()
		.map(|row| ChartEntry {
			bundle_id: row.id,
			description: row.description,
			extension: row.extension,
		})
		.collect();
	Ok(BundleSearchResult {
		has_more: offset + (charts.len() as i64) < total,
		charts,
		total: total.max(0).cast_unsigned(),
	})
}

pub(crate) fn normalise_extensions(extensions: &[&str]) -> Vec<String> {
	extensions
		.iter()
		.filter_map(|extension| {
			let extension = extension.trim();
			let extension = extension.strip_prefix('.').unwrap_or(extension);
			(!extension.is_empty()).then(|| extension.to_owned())
		})
		.collect()
}

/// Build a safe FTS5 `MATCH` query from free-form user input.
///
/// Returns `None` for an empty/whitespace-only query.
pub(crate) fn build_fts_query(query: &str) -> Option<String> {
	let terms: Vec<String> = query
		.split_whitespace()
		.map(|term| format!("\"{}\"*", term.replace('"', "\"\"")))
		.collect();
	(!terms.is_empty()).then(|| terms.join(" "))
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use backbeat_core::{
		ChartDesc, ChartFilename, {BackbeatFile, ChartData},
	};

	use super::{build_fts_query, normalise_extensions};
	use crate::store::Backbeat;

	fn import_bms(store: &Backbeat, filename: &str, artist: &str, title: &str) {
		let chart = format!("#ARTIST {artist}\n#TITLE {title}\n");
		let description = format!("{artist} - {title}");
		let bb = BackbeatFile {
			filename: ChartFilename::from_path(filename).unwrap(),
			assets: HashMap::new(),
			desc: ChartDesc::new(&description).unwrap(),
			chart: ChartData::compress(chart.as_bytes()).unwrap(),
		};
		store.import_bundle(&bb).expect("import_bb");
	}

	#[test]
	fn search_charts_without_query_returns_all_bundles_alphabetically() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_list_charts");

		import_bms(&store, "a.bms", "Camellia", "Zebra");
		import_bms(&store, "b.bms", "Camellia", "Alpha");

		let result = store
			.search_bundles(None, 0, 50, &[])
			.expect("search_charts");

		assert_eq!(result.total, 2);
		assert_eq!(result.charts.len(), 2);
		assert_eq!(result.charts[0].description, "Camellia - Alpha");
		assert_eq!(result.charts[1].description, "Camellia - Zebra");
		assert!(!result.has_more);
	}

	#[test]
	fn search_charts_without_query_paginates() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_list_charts_paginate");

		for i in 0..3 {
			import_bms(&store, &format!("{i}.bms"), "Artist", &format!("Song {i}"));
		}

		let result = store
			.search_bundles(None, 0, 2, &[])
			.expect("search_charts");
		assert_eq!(result.charts.len(), 2);
		assert!(result.has_more);
		assert_eq!(result.total, 3);
	}

	#[test]
	fn search_charts_filters_by_extension() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_list_charts_extension");

		import_bms(&store, "a.bms", "Camellia", "Alpha");
		import_bms(&store, "b.bme", "Camellia", "Beta");

		let all = store
			.search_bundles(None, 0, 50, &[])
			.expect("search_charts");
		assert_eq!(all.total, 2);

		let filtered = store
			.search_bundles(None, 0, 50, &[".BMS"])
			.expect("search_charts");
		assert_eq!(filtered.total, 1);

		let multiple = store
			.search_bundles(None, 0, 50, &[".BMS", "bme"])
			.expect("search_charts");
		assert_eq!(multiple.total, 2);

		let none = store
			.search_bundles(None, 0, 50, &["ksh"])
			.expect("search_charts");
		assert_eq!(none.total, 0);
		assert!(none.charts.is_empty());
	}

	#[test]
	fn search_charts_fts_filters_by_extension() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_search_charts_extension");

		import_bms(&store, "a.bms", "Camellia", "Alpha");
		import_bms(&store, "b.bme", "Camellia", "Beta");

		let matching = store
			.search_bundles(Some("Camellia"), 0, 50, &["bms"])
			.expect("search_charts");
		assert_eq!(matching.total, 1);

		let multiple = store
			.search_bundles(Some("Camellia"), 0, 50, &["bms", ".bme"])
			.expect("search_charts");
		assert_eq!(multiple.total, 2);

		let none = store
			.search_bundles(Some("Camellia"), 0, 50, &[".ksh"])
			.expect("search_charts");
		assert_eq!(none.total, 0);
	}

	#[test]
	fn extensionless_charts_do_not_match_an_extension_filter() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_extensionless_chart");

		import_bms(&store, "chart-without-extension", "Camellia", "Alpha");

		let result = store
			.search_bundles(None, 0, 50, &["bms"])
			.expect("search_charts");
		assert_eq!(result.total, 0);
	}

	#[test]
	fn normalise_extensions_accepts_a_leading_dot() {
		assert_eq!(
			normalise_extensions(&[" .BMS ", ".ksh", ".", ""]),
			vec!["BMS", "ksh"]
		);
		assert!(normalise_extensions(&[]).is_empty());
	}
	#[test]
	fn search_charts_matches_words_out_of_order() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_search_out_of_order");

		import_bms(&store, "a.bms", "Camellia", "Hatsune Miku - Songtitle");
		import_bms(&store, "b.bms", "Someone Else", "Unrelated Track");

		let result = store
			.search_bundles(Some("Hatsune Miku Camellia"), 0, 50, &[])
			.expect("search_charts");

		assert_eq!(result.charts.len(), 1);
		assert_eq!(
			result.charts[0].description,
			"Camellia - Hatsune Miku - Songtitle"
		);
		assert!(!result.has_more);
		assert_eq!(result.total, 1);
	}

	/// `total` counts all matches across all pages, not just the ones
	/// returned on the current page.
	#[test]
	fn search_charts_total_counts_all_pages() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_search_total");

		for i in 0..3 {
			import_bms(
				&store,
				&format!("{i}.bms"),
				"Camellia",
				&format!("Song {i}"),
			);
		}

		let result = store
			.search_bundles(Some("Camellia"), 0, 2, &[])
			.expect("search_charts");

		assert_eq!(result.charts.len(), 2);
		assert!(result.has_more);
		assert_eq!(result.total, 3);
	}

	#[test]
	fn search_charts_blank_query_lists_all_bundles() {
		let (_tmp, store) = crate::test_util::new_test_store("bb_search_empty_query");

		import_bms(&store, "a.bms", "Camellia", "Songtitle");

		let result = store
			.search_bundles(Some("   "), 0, 50, &[])
			.expect("search_charts");
		assert_eq!(result.charts.len(), 1);
	}

	#[test]
	fn build_fts_query_quotes_and_prefixes_each_term() {
		assert_eq!(
			build_fts_query("Hatsune Miku Camellia"),
			Some("\"Hatsune\"* \"Miku\"* \"Camellia\"*".to_string())
		);
	}

	#[test]
	fn build_fts_query_escapes_embedded_quotes() {
		assert_eq!(
			build_fts_query("foo\"bar"),
			Some("\"foo\"\"bar\"*".to_string())
		);
	}

	#[test]
	fn build_fts_query_neutralises_query_syntax() {
		// None of these should be interpretable as FTS5 operators once quoted.
		assert_eq!(
			build_fts_query("foo OR bar"),
			Some("\"foo\"* \"OR\"* \"bar\"*".to_string())
		);
	}

	#[test]
	fn build_fts_query_empty_input_is_none() {
		assert_eq!(build_fts_query(""), None);
		assert_eq!(build_fts_query("   "), None);
	}
}
