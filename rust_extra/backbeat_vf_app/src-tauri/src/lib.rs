#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![allow(unreachable_pub)]
mod daemon;
mod mounts;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tauri::Builder::default()
		.plugin(tauri_plugin_dialog::init())
		.setup(|_| {
			tauri::async_runtime::spawn(async move {
				if let Err(err) = daemon::ensure_running().await {
					eprintln!("failed to start Backbeat Virtual Folders mount daemon: {err}");
				}
			});
			Ok(())
		})
		.invoke_handler(tauri::generate_handler![
			mounts::list_virtual_folders,
			mounts::set_virtual_folder_path,
			mounts::set_virtual_folder_enabled,
			mounts::open_virtual_folder,
			mounts::open_config_folder,
			mounts::get_home_dir,
			mounts::get_fuse_installer,
			mounts::open_fuse_installer,
		])
		.run(tauri::generate_context!())
		.expect("error while running Backbeat Virtual Folders");
}
