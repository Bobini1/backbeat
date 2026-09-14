#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![allow(clippy::needless_pass_by_value)]
#![allow(unreachable_pub)]
#![doc = include_str!("../../README.md")]
mod assets;
mod charts;
mod collections;
mod commands;
mod deep_links;
mod downloads;
mod logging;
mod remotes;
mod settings;

use std::sync::Arc;
use std::time::Instant;

use arc_swap::ArcSwapOption;
use backbeat_sdk::Backbeat;
use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;

/// Shared state handed to every Tauri command via [`tauri::State`].
pub struct AppState {
	pub store: StoreHandle,
	pub deep_link_events: deep_links::DeepLinkEvents,
}

pub struct StoreHandle {
	inner: ArcSwapOption<Backbeat>,
}

impl StoreHandle {
	fn new(store: Backbeat) -> Self {
		Self {
			inner: ArcSwapOption::from(Some(Arc::new(store))),
		}
	}

	pub fn get(&self) -> Result<Arc<Backbeat>, String> {
		self.inner
			.load_full()
			.ok_or_else(|| "store is currently moving; try again when it finishes".to_string())
	}

	pub fn take_for_move(&self) -> Result<Backbeat, String> {
		let Some(store) = self.inner.swap(None) else {
			return Err("store is already moving".to_string());
		};
		match Arc::try_unwrap(store) {
			Ok(store) => Ok(store),
			Err(store) => {
				self.inner.store(Some(store));
				Err("store is busy; try again when current store work finishes".to_string())
			}
		}
	}

	pub fn replace(&self, store: Backbeat) {
		self.inner.store(Some(Arc::new(store)));
	}
}

fn elapsed_ms(start: &Instant) -> u128 {
	start.elapsed().as_millis()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	let startup_started = Instant::now();
	let config_dir = backbeat_store_config::default_config_dir();
	let store_open_started = Instant::now();
	let store = match Backbeat::open() {
		Ok(store) => store,
		Err(err) => {
			eprintln!(
				"failed to open Backbeat store using config {}: {err}",
				config_dir.display()
			);
			panic!("failed to open the Backbeat store: {err}");
		}
	};
	let log_dir = store.logs_dir();
	logging::init(&log_dir);

	tracing::debug!(
		elapsed_ms = elapsed_ms(&startup_started),
		phase_elapsed_ms = elapsed_ms(&store_open_started),
		"startup timing: Backbeat store opened and tracing initialized"
	);
	tracing::info!(
		config_dir = %config_dir.display(),
		store_dir = %store.store_dir().display(),
		log_dir = %log_dir.display(),
		"Backbeat store opened"
	);

	let mut builder = tauri::Builder::default();

	// Must be registered before the deep-link plugin: the single-instance
	// plugin intercepts duplicate launches (Windows/Linux warm launch, where
	// the OS spawns a second process with the `backbeat://` URL as argv) and
	// forwards the URL to the running instance.
	#[cfg(desktop)]
	{
		builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
			if let Some(window) = app.get_webview_window("main") {
				let _ = window.set_focus();
			}
			deep_links::handle_argv(app, &argv);
		}));
	}

	builder
		.plugin(tauri_plugin_dialog::init())
		.plugin(tauri_plugin_deep_link::init())
		.manage(AppState {
			store: StoreHandle::new(store),
			deep_link_events: deep_links::DeepLinkEvents::new(),
		})
		.setup(move |app| {
			tracing::debug!(
				elapsed_ms = elapsed_ms(&startup_started),
				"startup timing: tauri setup start"
			);
			// On Windows/Linux, register the `backbeat` scheme at runtime so
			// the OS routes `backbeat://` clicks to this binary. macOS uses
			// Info.plist (see tauri.conf.json) and ignores `register`.
			#[cfg(any(target_os = "windows", target_os = "linux"))]
			if let Err(err) = app.deep_link().register_all() {
				tracing::warn!(?err, "failed to register deep-link schemes");
			}

			let handle = app.handle();
			let closure_handle = handle.clone();
			handle.deep_link().on_open_url(move |event| {
				for url in event.urls() {
					deep_links::handle(&closure_handle, &url);
				}
			});

			// Cold launch: URLs that triggered this process (Windows/Linux
			// surface the URL via argv, which the deep-link plugin parsed
			// during init and exposed through `get_current`).
			match handle.deep_link().get_current() {
				Ok(Some(urls)) if !urls.is_empty() => {
					for url in urls {
						deep_links::handle(handle, &url);
					}
				}
				Ok(_) => {}
				Err(err) => tracing::warn!(?err, "failed to read current deep-link URLs"),
			}

			tracing::debug!(
				elapsed_ms = elapsed_ms(&startup_started),
				"startup timing: tauri setup end"
			);
			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			commands::startup_mark,
			commands::frontend_log,
			commands::build_info,
			commands::open_url,
			commands::store_summary,
			deep_links::take_pending_events,
			charts::search_charts,
			charts::list_available_extensions,
			charts::get_bundle_detail,
			charts::remove_bundle,
			charts::import_files,
			charts::export_bundle,
			assets::get_asset_detail,
			assets::get_asset_preview,
			assets::open_asset_folder,
			remotes::list_remotes,
			remotes::add_remote,
			remotes::remove_remote,
			remotes::ping_remote,
			collections::list_installed_collection_documents,
			collections::get_table,
			collections::get_pack,
			collections::get_course,
			collections::download_collection_content,
			collections::check_installed_collection_updates,
			collections::install_collection_document,
			collections::remove_collection,
			settings::get_settings,
			settings::open_config_folder,
			settings::open_logs_folder,
			settings::save_settings,
			settings::asset_prune,
			settings::disk_prune,
			settings::corruption_check,
			settings::corruption_repair,
			downloads::download_chart,
			downloads::download_bundle,
			downloads::download_overview,
			downloads::list_downloads,
			downloads::cancel_download,
			downloads::clear_finished_downloads,
		])
		.run(tauri::generate_context!())
		.expect("error while running the Backbeat GUI");
}
