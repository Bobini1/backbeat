use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use backbeat_core::AssetId;
use backbeat_sdk::StoreError;
use tokio_util::io::ReaderStream;

use crate::error_response::internal_error;
use crate::state::AppState;
use crate::{AssetServeResponse, serve_asset};

pub async fn get(State(state): State<AppState>, Path(asset_id): Path<AssetId>) -> Response {
	match serve_asset(&state.store, asset_id) {
		Ok(response) => asset_response(response).await,
		Err(StoreError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
		Err(err) => internal_error!(err, %asset_id),
	}
}

pub async fn head(State(state): State<AppState>, Path(asset_id): Path<AssetId>) -> Response {
	match serve_asset(&state.store, asset_id) {
		Ok(response) => asset_head_response(&response),
		Err(backbeat_sdk::StoreError::NotFound(_)) => StatusCode::NOT_FOUND.into_response(),
		Err(err) => internal_error!(err, %asset_id),
	}
}

pub async fn asset_response(response: AssetServeResponse) -> Response {
	match response {
		AssetServeResponse::Bytes {
			data,
			content_type,
			len,
		} => {
			let mut builder = Response::builder()
				.status(StatusCode::OK)
				.header(header::CONTENT_LENGTH, len.to_string());
			if let Some(content_type) = content_type {
				builder = builder.header(header::CONTENT_TYPE, content_type);
			}
			builder.body(Body::from(data)).unwrap()
		}
		AssetServeResponse::File {
			path,
			content_type,
			len,
		} => match tokio::fs::File::open(&path).await {
			Ok(file) => {
				let stream = ReaderStream::new(file);
				let mut builder = Response::builder()
					.status(StatusCode::OK)
					.header(header::CONTENT_LENGTH, len.to_string());
				if let Some(content_type) = content_type {
					builder = builder.header(header::CONTENT_TYPE, content_type);
				}
				builder.body(Body::from_stream(stream)).unwrap()
			}
			// The DB says this asset exists at `path`, but the file isn't
			// there (or isn't readable) -- that's a store inconsistency,
			// not a normal "no such asset". Surface it as a 500 with a
			// loud log rather than a misleading 404.
			Err(err) => {
				tracing::error!(
					error = %err,
					path = %path.display(),
					"asset file missing or unreadable despite a DB row referencing it"
				);
				StatusCode::INTERNAL_SERVER_ERROR.into_response()
			}
		},
	}
}

pub fn asset_head_response(response: &AssetServeResponse) -> Response {
	match response {
		AssetServeResponse::Bytes {
			content_type, len, ..
		}
		| AssetServeResponse::File {
			content_type, len, ..
		} => {
			let mut builder = Response::builder()
				.status(StatusCode::OK)
				.header(header::CONTENT_LENGTH, len.to_string());
			if let Some(content_type) = content_type {
				builder = builder.header(header::CONTENT_TYPE, content_type);
			}
			builder.body(Body::empty()).unwrap()
		}
	}
}
