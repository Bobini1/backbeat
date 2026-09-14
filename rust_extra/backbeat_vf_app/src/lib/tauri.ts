import { invoke } from "@tauri-apps/api/core";

export type VirtualFolderKind = "bms" | "kshoot" | "stepmania";

export interface VirtualFolderStatus {
	enabled: boolean;
	error: null | string;
	mount_path: null | string;
	mount_supported: boolean;
	mounted: boolean;
}

export interface VirtualFolder {
	folder: VirtualFolderKind;
	status: VirtualFolderStatus;
}

export interface FuseInstaller {
	name: string;
	url: string;
}

export const listVirtualFolders = () => invoke<VirtualFolder[]>("list_virtual_folders");

export const setVirtualFolderPath = (folder: VirtualFolderKind, mountPath: string) =>
	invoke<VirtualFolderStatus>("set_virtual_folder_path", { folder, mountPath });

export const setVirtualFolderEnabled = (folder: VirtualFolderKind, enabled: boolean) =>
	invoke<VirtualFolderStatus>("set_virtual_folder_enabled", { folder, enabled });

export const openVirtualFolder = (folder: VirtualFolderKind) =>
	invoke<void>("open_virtual_folder", { folder });

export const openConfigFolder = () => invoke<void>("open_config_folder");

export const getHomeDir = () => invoke<string>("get_home_dir");

export const getFuseInstaller = () => invoke<FuseInstaller>("get_fuse_installer");

export const openFuseInstaller = () => invoke<void>("open_fuse_installer");
