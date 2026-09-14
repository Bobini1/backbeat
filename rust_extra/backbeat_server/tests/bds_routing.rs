#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

//! BDS routing tests.

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use backbeat_core::asset_id::AssetId;
use backbeat_core::{Assets, BackbeatFile, ChartData, ChartDesc};
use backbeat_core::{BundleId, ChartFilename, Sha256};
use backbeat_sdk::Backbeat;
use backbeat_store_config::{BackbeatConfig, BackbeatServerInfo};
use tower::ServiceExt;

mod support;
use support::{new_test_store, new_test_store_with};

const TEST_ADDR: &str = "127.0.0.1:0";

fn test_addr() -> std::net::SocketAddr {
	TEST_ADDR.parse().unwrap()
}

fn test_store() -> (tempfile::TempDir, Backbeat) {
	new_test_store_with(
		"bds_serve",
		BackbeatConfig {
			info: Some(BackbeatServerInfo {
				name: "Local Backbeat".into(),
				contact: Some("ops@example.com".into()),
			}),
			..Default::default()
		},
	)
}

fn router_app(store: Backbeat) -> axum::Router {
	backbeat_server::router(backbeat_server::AppState::new(store, test_addr()).unwrap())
}

#[tokio::test(flavor = "multi_thread")]
async fn info_returns_name_and_contact() {
	let (_tmp, store) = test_store();
	let app = router_app(store);

	let response = app
		.oneshot(
			Request::builder()
				.uri("/backbeat/info")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(response.status(), StatusCode::OK);
	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.unwrap();
	let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
	assert_eq!(json["name"], "Local Backbeat");
	assert_eq!(json["contact"], "ops@example.com");
	assert!(json.get("virtualFolders").is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn info_works_without_publish_config() {
	let (_tmp, store) = new_test_store("bds_serve_no_publish");
	let app = router_app(store);

	let response = app
		.oneshot(
			Request::builder()
				.uri("/backbeat/info")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();

	assert_eq!(response.status(), StatusCode::OK);
	let body = axum::body::to_bytes(response.into_body(), usize::MAX)
		.await
		.unwrap();
	let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
	assert_eq!(json["name"], "Unnamed Server");
	assert!(json.get("virtualFolders").is_none());
}

fn sample_bb(filename: &str, chart_bytes: &[u8]) -> BackbeatFile {
	BackbeatFile {
		filename: ChartFilename::from_path(filename).unwrap(),
		assets: Assets::default(),
		desc: ChartDesc::new("test").unwrap(),
		chart: ChartData::compress(chart_bytes).unwrap(),
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn asset_route_serves_inline_and_rejects_invalid_hashes() {
	let (_tmp, store) = test_store();
	let data = b"fixture asset payload";
	let sha256 = Sha256::checksum_bytes(data);
	backbeat_sdk::test_support::store_asset_unchecked(&store, AssetId(sha256), data)
		.expect("store asset");

	let app = router_app(store);

	let ok = app
		.clone()
		.oneshot(
			Request::builder()
				.uri(format!("/assets/{sha256}"))
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(ok.status(), StatusCode::OK);
	assert_eq!(
		ok.headers()
			.get("content-length")
			.and_then(|v| v.to_str().ok()),
		Some("21")
	);
	let body = axum::body::to_bytes(ok.into_body(), usize::MAX)
		.await
		.unwrap();
	assert_eq!(body.as_ref(), data);

	let head = app
		.clone()
		.oneshot(
			Request::builder()
				.method(Method::HEAD)
				.uri(format!("/assets/{sha256}"))
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(head.status(), StatusCode::OK);
	assert_eq!(
		head.headers()
			.get("content-length")
			.and_then(|v| v.to_str().ok()),
		Some("21")
	);

	let missing = app
		.clone()
		.oneshot(
			Request::builder()
				.uri(format!("/assets/{}", "0".repeat(64)))
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(missing.status(), StatusCode::NOT_FOUND);

	for invalid in ["deadbeef", &"g".repeat(64), &"A".repeat(64).to_string()] {
		let resp = app
			.clone()
			.oneshot(
				Request::builder()
					.uri(format!("/assets/{invalid}"))
					.body(Body::empty())
					.unwrap(),
			)
			.await
			.unwrap();
		assert_eq!(
			resp.status(),
			StatusCode::BAD_REQUEST,
			"expected 400 for invalid hash {invalid:?}"
		);
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn bundle_route_fetches_exact_bundle_and_404s_unknown() {
	let (_tmp, store) = test_store();
	let bb = sample_bb("song.bms", b"#TITLE:one;");
	store.import_bundle(&bb).expect("import");
	let bundle_id = bb.bundle_id();

	let app = router_app(store);

	let ok = app
		.clone()
		.oneshot(
			Request::builder()
				.uri(format!("/bundles/{bundle_id}"))
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(ok.status(), StatusCode::OK);

	let body = axum::body::to_bytes(ok.into_body(), usize::MAX)
		.await
		.unwrap();
	let fetched = BackbeatFile::from_json(&body).expect("parse .bb");
	assert_eq!(fetched.bundle_id(), bundle_id);

	let head = app
		.clone()
		.oneshot(
			Request::builder()
				.method(Method::HEAD)
				.uri(format!("/bundles/{bundle_id}"))
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(head.status(), StatusCode::OK);

	let unknown_id: BundleId = format!("b-{}", "0".repeat(64)).parse().unwrap();
	let missing = app
		.clone()
		.oneshot(
			Request::builder()
				.uri(format!("/bundles/{unknown_id}"))
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(missing.status(), StatusCode::NOT_FOUND);

	let invalid = app
		.oneshot(
			Request::builder()
				.uri("/bundles/not-a-bundle-id")
				.body(Body::empty())
				.unwrap(),
		)
		.await
		.unwrap();
	assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
}
