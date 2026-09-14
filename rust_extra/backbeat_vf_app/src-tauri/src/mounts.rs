use std::path::PathBuf;

use backbeat_vf_config::default_config_dir;
use backbeat_vf_mountd::{Request, Response, VirtualFolderKind, VirtualFolderStatus, client};
use serde::Serialize;

use crate::daemon;

#[derive(Serialize)]
pub struct VirtualFolder {
	pub folder: VirtualFolderKind,
	pub status: VirtualFolderStatus,
}

#[derive(Serialize)]
pub struct FuseInstaller {
	pub name: &'static str,
	pub url: &'static str,
}

#[tauri::command]
pub async fn list_virtual_folders() -> Result<Vec<VirtualFolder>, String> {
	daemon::ensure_running().await?;
	let mut folders = Vec::with_capacity(VirtualFolderKind::ALL.len());
	for folder in VirtualFolderKind::ALL {
		folders.push(VirtualFolder {
			folder,
			status: request_status(folder).await?,
		});
	}
	Ok(folders)
}

#[tauri::command]
pub async fn set_virtual_folder_path(
	folder: VirtualFolderKind,
	mount_path: String,
) -> Result<VirtualFolderStatus, String> {
	daemon::ensure_running().await?;
	request_folder(Request::SetPath { folder, mount_path }).await
}

#[tauri::command]
pub async fn set_virtual_folder_enabled(
	folder: VirtualFolderKind,
	enabled: bool,
) -> Result<VirtualFolderStatus, String> {
	daemon::ensure_running().await?;
	if enabled {
		request_folder(Request::Mount { folder }).await
	} else {
		request_folder(Request::Unmount { folder }).await
	}
}

#[tauri::command]
pub async fn open_virtual_folder(folder: VirtualFolderKind) -> Result<(), String> {
	daemon::ensure_running().await?;
	let status = request_status(folder).await?;
	let path = status
		.mount_path
		.ok_or_else(|| "choose a mount folder first".to_owned())?;
	std::fs::create_dir_all(&path).map_err(|err| err.to_string())?;
	open::that_detached(PathBuf::from(path)).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn open_config_folder() -> Result<(), String> {
	let path = default_config_dir();
	std::fs::create_dir_all(&path).map_err(|err| err.to_string())?;
	open::that_detached(path).map_err(|err| err.to_string())
}

#[tauri::command]
pub fn get_home_dir() -> Result<String, String> {
	directories::BaseDirs::new()
		.map(|dirs| dirs.home_dir().display().to_string())
		.ok_or_else(|| "could not determine the user home directory".to_owned())
}

#[tauri::command]
pub fn get_fuse_installer() -> FuseInstaller {
	#[cfg(target_os = "macos")]
	return FuseInstaller {
		name: "macFUSE",
		url: "https://macfuse.github.io/",
	};
	#[cfg(target_os = "linux")]
	return FuseInstaller {
		name: "libfuse",
		url: "https://github.com/libfuse/libfuse",
	};
	#[cfg(windows)]
	return FuseInstaller {
		name: "WinFsp",
		url: "https://winfsp.dev/",
	};
}

#[tauri::command]
pub fn open_fuse_installer() -> Result<(), String> {
	open::that_detached(get_fuse_installer().url).map_err(|err| err.to_string())
}

async fn request_status(folder: VirtualFolderKind) -> Result<VirtualFolderStatus, String> {
	request_folder(Request::Status { folder }).await
}

async fn request_folder(request: Request) -> Result<VirtualFolderStatus, String> {
	match client::request(request).await? {
		Response::Ok {
			folder: Some(status),
		} => Ok(status),
		Response::Ok { folder: None } => Err("mount daemon returned no folder status".into()),
		Response::Error { message } => Err(message),
	}
}
