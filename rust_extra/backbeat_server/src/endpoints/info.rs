use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

/// `GET /backbeat/info` — returns the server's `name` and optional `contact`.
pub async fn get(State(state): State<AppState>) -> Response {
	let info = state.info.clone().unwrap_or_default();
	Json(info).into_response()
}
