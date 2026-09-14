use axum::http::StatusCode;
use backbeat_sdk::StoreError;
use backbeat_store_config::ConfigError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServeError {
	#[error("I/O error: {0}")]
	Io(#[from] std::io::Error),

	#[error("store error: {0}")]
	Store(#[from] StoreError),

	#[error("config error: {0}")]
	Config(#[from] ConfigError),

	#[error("invalid SHA-256: {0}")]
	InvalidSha256(String),
}

impl ServeError {
	pub fn status_code(&self) -> StatusCode {
		match self {
			Self::Store(StoreError::NotFound(_)) => StatusCode::NOT_FOUND,
			_ => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
}
