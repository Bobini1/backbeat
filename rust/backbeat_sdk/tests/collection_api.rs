#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

use std::collections::HashMap;

use backbeat_core::ValidGamemodeIdentifier;
use backbeat_core::{BackbeatFile, ChartData, ChartDesc, ChartFilename};
use backbeat_sdk::collections::CollectionUpsertStatus;
use backbeat_sdk::{CollectionClientError, StoreError};
use serde_json::json;

mod support;
use support::{new_test_store, spawn_collection_server, spawn_mutable_collection_server};

#[tokio::test(flavor = "multi_thread")]
async fn collection_fetch_rejects_http_error_status() {
	let (_temp, store) = new_test_store("collection_api_http_status");
	let base = spawn_collection_server(Vec::new()).await;

	let error = store.collection_fetch_header(&base).await.unwrap_err();

	assert!(matches!(
		error,
		StoreError::CollectionClient(CollectionClientError::HttpStatus {
			status: 404,
			ref url,
		}) if url == &format!("{base}/header.json")
	));
}

#[tokio::test(flavor = "multi_thread")]
async fn collection_contents_resolve_installed_bundles() {
	let (_temp, store) = new_test_store("collection_contents");
	let chart = ChartData::compress(b"#TITLE Test\n#ARTIST Backbeat\n").unwrap();
	let first = BackbeatFile {
		filename: ChartFilename::new("first.bms").unwrap(),
		assets: HashMap::new(),
		desc: ChartDesc::new("test").unwrap(),
		chart: chart.clone(),
	};
	let second = BackbeatFile {
		filename: ChartFilename::new("second.bms").unwrap(),
		assets: HashMap::new(),
		desc: ChartDesc::new("test").unwrap(),
		chart,
	};
	let first_bundle = store.import_bundle(&first).unwrap();
	let second_bundle = store.import_bundle(&second).unwrap();
	let chart_id = store
		.bundle_detail(first_bundle)
		.unwrap()
		.chart_ids
		.into_iter()
		.next()
		.expect("BMS inspection should produce a chart ID");
	let missing_id = format!("sha256/{}", "f".repeat(64));
	let table_json = json!({
		"name": "Resolved Table",
		"symbol": "L",
		"gamemode": "bms-7k",
		"updated": "2026-07-15T00:00:00Z",
		"tags": {},
		"assets": {},
		"levels": [{
			"level": "1",
			"tags": {},
			"charts": [
				{"id": chart_id.to_string(), "desc": "Installed", "tags": {"kind": "installed"}},
				{"id": missing_id, "desc": "Missing", "tags": {}}
			]
		}],
		"folders": [
			{"name": "Installed", "query": "tags.kind == \"installed\"", "tags": {}},
			{"name": "Invalid", "query": "level ==", "tags": {}}
		]
	});
	let course_json = json!({
		"name": "Resolved Course",
		"gamemode": "bms-7k",
		"updated": "2026-07-15T00:00:00Z",
		"tags": {},
		"assets": {},
		"charts": [
			{"id": format!("sha256/{}", first.chart_sha256()), "desc": "Installed", "tags": {}},
			{"id": missing_id, "desc": "Missing", "tags": {}}
		]
	});
	let pack_json = json!({
		"name": "Resolved Pack",
		"gamemode": "bms-7k",
		"updated": "2026-07-15T00:00:00Z",
		"tags": {},
		"assets": {},
		"bundles": [
			{"id": first_bundle.to_string(), "desc": "Installed", "tags": {}},
			{"id": format!("b-{}", "f".repeat(64)), "desc": "Missing", "tags": {}}
		]
	});
	let routes = vec![
		(
			"/table/header.json".to_owned(),
			json!({"timestamp": "2026-07-15T00:00:00Z", "kind": "table"}).to_string(),
		),
		("/table/data.bbtable".to_owned(), table_json.to_string()),
		(
			"/course/header.json".to_owned(),
			json!({"timestamp": "2026-07-15T00:00:00Z", "kind": "course"}).to_string(),
		),
		("/course/data.bbcourse".to_owned(), course_json.to_string()),
		(
			"/pack/header.json".to_owned(),
			json!({"timestamp": "2026-07-15T00:00:00Z", "kind": "pack"}).to_string(),
		),
		("/pack/data.bbpack".to_owned(), pack_json.to_string()),
	];
	let base = spawn_collection_server(routes).await;
	store
		.collection_fetch_upsert(&format!("{base}/table"))
		.await
		.unwrap();
	store
		.collection_fetch_upsert(&format!("{base}/course"))
		.await
		.unwrap();
	store
		.collection_fetch_upsert(&format!("{base}/pack"))
		.await
		.unwrap();

	let table = store.get_table(&format!("{base}/table")).unwrap();
	let resolved = table.levels[0].charts[0].bundle_id.unwrap();
	assert!([first_bundle, second_bundle].contains(&resolved));
	assert_eq!(table.levels[0].charts[1].bundle_id, None);
	assert_eq!(table.folders[0].charts.len(), 1);
	assert_eq!(table.folders[0].charts[0].id, chart_id);
	assert_eq!(table.folders[0].charts[0].desc, "Installed");
	assert_eq!(table.folders[0].charts[0].bundle_id, Some(resolved));
	assert!(table.folders[1].charts.is_empty());

	let course = store.get_course(&format!("{base}/course")).unwrap();
	assert!([first_bundle, second_bundle].contains(&course.charts[0].bundle_id.unwrap()));
	assert_eq!(course.charts[1].bundle_id, None);
	let pack = store.get_pack(&format!("{base}/pack")).unwrap();
	assert_eq!(pack.bundles[0].id, first_bundle);
	assert!(pack.bundles[0].installed);
	assert!(!pack.bundles[1].installed);

	let table_metadata = store.list_tables(&[]).unwrap();
	assert_eq!(table_metadata[0].gamemode.as_str(), "bms-7k");
	assert_eq!(table_metadata[0].installed, 1);
	assert_eq!(table_metadata[0].total, 2);
	let course_metadata = store.list_courses(&[]).unwrap();
	assert_eq!(course_metadata[0].gamemode.as_str(), "bms-7k");
	assert_eq!(course_metadata[0].installed, 1);
	assert_eq!(course_metadata[0].total, 2);
	let pack_metadata = store.list_packs(&[]).unwrap();
	assert_eq!(pack_metadata[0].gamemode.as_str(), "bms-7k");
	assert_eq!(pack_metadata[0].installed, 1);
	assert_eq!(pack_metadata[0].total, 2);

	let stats = store.stats().unwrap();
	assert_eq!(stats.tables, 1);
	assert_eq!(stats.courses, 1);
	assert_eq!(stats.packs, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn collection_upsert_round_trips_each_kind() {
	let (_temp, store) = new_test_store("collection_api");
	let table_json = json!({
		"name": "Table",
		"symbol": "L",
		"gamemode": "bms-7k",
		"updated": "2026-07-15T00:00:00Z",
		"tags": {"region": "jp"},
		"assets": {"banner.png": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"},
		"levels": [{
			"level": "1",
			"tags": {"color": "red"},
			"charts": [{"id": "sha256/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "desc": "Chart", "tags": {"mode": "normal"}}]
		}],
		"folders": [{"name": "All", "query": "level >= 1", "tags": {"kind": "all"}}]
	});
	let course_json = json!({
		"name": "Course",
		"gamemode": "bms-7k",
		"updated": "2026-07-15T00:00:00Z",
		"tags": {"course": "tag"},
		"assets": {"banner.png": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"},
		"charts": [{"id": "sha256/dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd", "desc": "Course chart", "tags": {"stage": "1"}}]
	});
	let pack_json = json!({
		"name": "Pack",
		"gamemode": "sm-dance-single",
		"updated": "2026-07-15T00:00:00Z",
		"tags": {"series": "one"},
		"assets": {"banner.png": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"},
		"bundles": [{"id": "b-ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", "desc": "Bundle", "tags": {"pack": "tag"}}]
	});
	let routes = vec![
		(
			"/table/header.json".to_owned(),
			json!({"timestamp": "2026-07-15T00:00:00Z", "kind": "table"}).to_string(),
		),
		("/table/data.bbtable".to_owned(), table_json.to_string()),
		(
			"/course/header.json".to_owned(),
			json!({"timestamp": "2026-07-15T00:00:00Z", "kind": "course"}).to_string(),
		),
		("/course/data.bbcourse".to_owned(), course_json.to_string()),
		(
			"/pack/header.json".to_owned(),
			json!({"timestamp": "2026-07-15T00:00:00Z", "kind": "pack"}).to_string(),
		),
		("/pack/data.bbpack".to_owned(), pack_json.to_string()),
	];
	let base = spawn_collection_server(routes).await;

	assert_eq!(
		store
			.collection_fetch_upsert(&format!("{base}/table"))
			.await
			.unwrap()
			.status,
		CollectionUpsertStatus::Inserted
	);
	assert_eq!(
		store
			.collection_fetch_upsert(&format!("{base}/course"))
			.await
			.unwrap()
			.status,
		CollectionUpsertStatus::Inserted
	);
	assert_eq!(
		store
			.collection_fetch_upsert(&format!("{base}/pack"))
			.await
			.unwrap()
			.status,
		CollectionUpsertStatus::Inserted
	);

	let table = store.get_table(&format!("{base}/table")).unwrap();
	assert_eq!(table.name, "Table");
	assert_eq!(table.levels.len(), 1);
	assert_eq!(table.levels[0].charts.len(), 1);
	assert_eq!(table.levels[0].charts[0].bundle_id, None);
	assert_eq!(table.folders.len(), 1);
	let course = store.get_course(&format!("{base}/course")).unwrap();
	assert_eq!(course.name, "Course");
	assert_eq!(course.charts.len(), 1);
	assert_eq!(course.charts[0].bundle_id, None);
	let pack = store.get_pack(&format!("{base}/pack")).unwrap();
	assert_eq!(pack.name, "Pack");
	assert_eq!(pack.bundles.len(), 1);
	assert!(!pack.bundles[0].installed);

	let bms = ValidGamemodeIdentifier::new("bms-7k").unwrap();
	let stepmania = ValidGamemodeIdentifier::new("sm-dance-single").unwrap();

	assert_eq!(store.list_tables(&[]).unwrap().len(), 1);
	assert_eq!(
		store.list_tables(std::slice::from_ref(&bms)).unwrap().len(),
		1
	);
	assert!(
		store
			.list_tables(std::slice::from_ref(&stepmania))
			.unwrap()
			.is_empty()
	);

	assert_eq!(store.list_courses(&[]).unwrap().len(), 1);
	assert_eq!(
		store
			.list_courses(std::slice::from_ref(&bms))
			.unwrap()
			.len(),
		1
	);
	assert!(
		store
			.list_courses(std::slice::from_ref(&stepmania))
			.unwrap()
			.is_empty()
	);

	assert_eq!(store.list_packs(&[]).unwrap().len(), 1);
	assert!(
		store
			.list_packs(std::slice::from_ref(&bms))
			.unwrap()
			.is_empty()
	);
	assert_eq!(store.list_packs(&[bms, stepmania]).unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn collection_upsert_rejects_timestamp_mismatch_without_writing() {
	let (_temp, store) = new_test_store("collection_api_mismatch");
	let base = spawn_collection_server(vec![
		("/table/header.json".to_owned(), json!({"timestamp": "2026-07-15T00:00:00Z", "kind": "table"}).to_string()),
		("/table/data.bbtable".to_owned(), json!({"name": "Table", "symbol": "L", "gamemode": "bms-7k", "updated": "2026-07-15T00:00:01Z", "tags": {}, "assets": {}, "levels": [], "folders": []}).to_string()),
	]).await;
	let error = store
		.collection_fetch_upsert(&format!("{base}/table"))
		.await
		.unwrap_err();
	assert!(error.to_string().contains("timestamp changed"));
	assert!(store.get_table(&format!("{base}/table")).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn collection_upsert_replaces_only_newer_content() {
	let (_temp, store) = new_test_store("collection_api_update");
	let old_body = json!({"name": "Old", "symbol": "L", "gamemode": "bms-7k", "updated": "2026-07-15T00:00:00Z", "tags": {}, "assets": {}, "levels": [], "folders": []}).to_string();
	let (base, routes) = spawn_mutable_collection_server(vec![
		(
			"/table/header.json".to_owned(),
			json!({"timestamp": "2026-07-15T00:00:00Z", "kind": "table"}).to_string(),
		),
		("/table/data.bbtable".to_owned(), old_body),
	])
	.await;
	let url = format!("{base}/table");
	assert_eq!(
		store.collection_fetch_upsert(&url).await.unwrap().status,
		CollectionUpsertStatus::Inserted
	);

	routes.lock().await.insert(
		"/table/header.json".to_owned(),
		json!({"timestamp": "2026-07-16T00:00:00Z", "kind": "table"}).to_string(),
	);
	routes.lock().await.insert(
		"/table/data.bbtable".to_owned(),
		json!({"name": "New", "symbol": "L", "gamemode": "bms-7k", "updated": "2026-07-16T00:00:00Z", "tags": {}, "assets": {}, "levels": [], "folders": []}).to_string(),
	);
	assert_eq!(
		store.collection_fetch_upsert(&url).await.unwrap().status,
		CollectionUpsertStatus::Updated
	);
	assert_eq!(store.get_table(&url).unwrap().name, "New");

	routes.lock().await.insert(
		"/table/header.json".to_owned(),
		json!({"timestamp": "2026-07-14T00:00:00Z", "kind": "table"}).to_string(),
	);
	routes.lock().await.insert(
		"/table/data.bbtable".to_owned(),
		json!({"name": "Older", "symbol": "L", "gamemode": "bms-7k", "updated": "2026-07-14T00:00:00Z", "tags": {}, "assets": {}, "levels": [], "folders": []}).to_string(),
	);
	assert_eq!(
		store.collection_fetch_upsert(&url).await.unwrap().status,
		CollectionUpsertStatus::TimestampUnchanged
	);
	assert_eq!(store.get_table(&url).unwrap().name, "New");
}
