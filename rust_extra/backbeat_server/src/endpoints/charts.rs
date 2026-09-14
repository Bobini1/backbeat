use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use backbeat_core::ChartId;
use backbeat_sdk::StoreError;
use http::header;

use crate::error_response::internal_error;
use crate::state::AppState;

pub async fn get(State(state): State<AppState>, Path(chart_id): Path<ChartId>) -> Response {
	let bb = match state.store.get_chart(&chart_id) {
		Ok(lookup) => lookup,
		Err(StoreError::NotFound(_)) => return StatusCode::NOT_FOUND.into_response(),
		Err(err) => return internal_error!(err, %chart_id),
	};

	let json = bb.to_json();

	Response::builder()
		.status(StatusCode::OK)
		.header(header::CONTENT_TYPE, "application/json")
		.header(header::CONTENT_LENGTH, json.len().to_string())
		.body(Body::from(json))
		.expect("is a valid http response")
}

pub async fn head(State(state): State<AppState>, Path(chart_id): Path<ChartId>) -> Response {
	let bb = match state.store.get_chart(&chart_id) {
		Ok(lookup) => lookup,
		Err(StoreError::NotFound(_)) => return StatusCode::NOT_FOUND.into_response(),
		Err(err) => return internal_error!(err, %chart_id),
	};

	let json = bb.to_json();

	Response::builder()
		.status(StatusCode::OK)
		.header(header::CONTENT_LENGTH, json.len().to_string())
		.body(Body::empty())
		.expect("is a valid http response")
}
