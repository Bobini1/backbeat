//! Helper for turning a failed [`Result`] into a `500 Internal Server Error`
//! response while leaving a real trail behind.
//!
//! Every 500 an endpoint returns should be paired with an `ERROR`-level
//! trace event naming exactly what failed, so a `curl` returning a bare
//! `500` is never a dead end. We deliberately don't put the error detail in
//! the HTTP response body (callers shouldn't need to parse arbitrary
//! internal error strings, and we shouldn't leak implementation details),
//! but it must always be visible in the server's logs.
//!
//! Implemented as a `macro_rules!` (rather than a function) so `tracing`
//! attributes the event to the actual call site instead of this module.

/// Log `$err` at `ERROR` with any extra `field = value` context, then
/// produce a `500 Internal Server Error` response.
///
/// ```ignore
/// Err(err) => internal_error!(err, bundle_id = %bundle_id),
/// ```
macro_rules! internal_error {
	($err:expr) => {{
		let err = &$err;
		::tracing::error!(
			error = %err,
			error.debug = ?err,
			"request failed with 500 Internal Server Error"
		);
		::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
	}};
	($err:expr, $($fields:tt)+) => {{
		let err = &$err;
		::tracing::error!(
			error = %err,
			error.debug = ?err,
			$($fields)+,
			"request failed with 500 Internal Server Error"
		);
		::axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
	}};
}

pub(crate) use internal_error;
