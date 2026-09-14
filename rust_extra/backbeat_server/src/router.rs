use axum::Router;
use axum::routing::get;

use crate::endpoints::{assets, bundles, charts, info, up};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
	Router::new()
		.route("/backbeat/up", get(up::get))
		.route("/backbeat/info", get(info::get))
		.route("/assets/{sha256}", get(assets::get).head(assets::head))
		.route(
			"/bundles/{bundle_id}",
			get(bundles::get).head(bundles::head),
		)
		.route("/charts/{*chart_id}", get(charts::get).head(charts::head))
		.with_state(state)
}
