use std::str::FromStr as _;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use backbeat_core::BundleId;
use http::header;

use crate::error_response::internal_error;
use crate::state::AppState;

pub async fn get(State(state): State<AppState>, Path(bundle_id_str): Path<String>) -> Response {
	let Ok(bundle_id) = BundleId::from_str(&bundle_id_str) else {
		return StatusCode::BAD_REQUEST.into_response();
	};

	match state.store.get_bundle(bundle_id) {
		Ok(bb) => {
			let json = bb.to_json();
			Response::builder()
				.header(header::CONTENT_TYPE, "application/json")
				.header(header::CONTENT_LENGTH, json.len())
				.body(Body::from(json))
				.expect("is a valid http response")
		}
		Err(backbeat_sdk::StoreError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
		Err(err) => internal_error!(err, bundle_id = %bundle_id),
	}
}

pub async fn head(State(state): State<AppState>, Path(bundle_id_str): Path<String>) -> Response {
	let Ok(bundle_id) = BundleId::from_str(&bundle_id_str) else {
		return StatusCode::BAD_REQUEST.into_response();
	};

	match state.store.get_bundle(bundle_id) {
		Ok(bb) => {
			let json = bb.to_json();
			Response::builder()
				.header(header::CONTENT_TYPE, "application/json")
				.header(header::CONTENT_LENGTH, json.len())
				.body(Body::empty())
				.expect("is a valid http response")
		}
		Err(backbeat_sdk::StoreError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
		Err(err) => internal_error!(err, bundle_id = %bundle_id),
	}
}
