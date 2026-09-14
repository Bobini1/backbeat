use axum::http::StatusCode;
use axum::response::IntoResponse;

pub async fn get() -> impl IntoResponse {
	StatusCode::OK
}
