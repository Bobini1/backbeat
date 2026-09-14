#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use backbeat_sdk::Backbeat;
use backbeat_vf_config::{MountsConfig, VfConfig, default_config_dir};
use backbeat_vf_fs::BackbeatFilesystem;
use serde::{Deserialize, Serialize};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[cfg(unix)]
use backbeat_vf_fuse as backend;
#[cfg(windows)]
use backbeat_vf_winfsp as backend;

#[cfg(unix)]
pub type ActiveMount = backend::MountHandle;
#[cfg(windows)]
pub type ActiveMount = backend::MountHandle;

const SOCKET_NAME: &str = "mountd.sock";
const LOG_PREFIX: &str = "mountd";
const LOG_RETAIN_DAYS: u64 = 7;
const STORE_REFRESH_POLL_INTERVAL: Duration = Duration::from_millis(100);
const RELOAD_DEBOUNCE: Duration = Duration::from_millis(500);
#[cfg(windows)]
const PIPE_NAME: &str = r"\\.\pipe\backbeat-mountd";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VirtualFolderKind {
	Bms,
	Kshoot,
	Stepmania,
}

impl VirtualFolderKind {
	pub const ALL: [Self; 3] = [Self::Bms, Self::Kshoot, Self::Stepmania];

	fn configured_path(self, config: &MountsConfig) -> Option<&PathBuf> {
		match self {
			Self::Bms => config.bms_path.as_ref(),
			Self::Kshoot => config.kshoot_path.as_ref(),
			Self::Stepmania => config.stepmania_path.as_ref(),
		}
	}

	fn set_configured_path(self, config: &mut MountsConfig, path: PathBuf) {
		match self {
			Self::Bms => config.bms_path = Some(path),
			Self::Kshoot => config.kshoot_path = Some(path),
			Self::Stepmania => config.stepmania_path = Some(path),
		}
	}

	fn enabled(self, config: &MountsConfig) -> bool {
		match self {
			Self::Bms => config.bms_enabled,
			Self::Kshoot => config.kshoot_enabled,
			Self::Stepmania => config.stepmania_enabled,
		}
	}

	fn set_enabled(self, config: &mut MountsConfig, enabled: bool) {
		match self {
			Self::Bms => config.bms_enabled = enabled,
			Self::Kshoot => config.kshoot_enabled = enabled,
			Self::Stepmania => config.stepmania_enabled = enabled,
		}
	}

	async fn build_filesystem(self, store: Backbeat) -> Result<BackbeatFilesystem, String> {
		match self {
			Self::Bms => backbeat_vf_fs::bms::build_filesystem(store)
				.await
				.map_err(|err| err.to_string()),
			Self::Kshoot => backbeat_vf_fs::kshoot::build_filesystem(store)
				.await
				.map_err(|err| err.to_string()),
			Self::Stepmania => backbeat_vf_fs::stepmania::build_filesystem(store)
				.await
				.map_err(|err| err.to_string()),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VirtualFolderStatus {
	pub mount_path: Option<String>,
	pub enabled: bool,
	pub mounted: bool,
	pub mount_supported: bool,
	pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Request {
	Status {
		folder: VirtualFolderKind,
	},
	SetPath {
		folder: VirtualFolderKind,
		mount_path: String,
	},
	Mount {
		folder: VirtualFolderKind,
	},
	Unmount {
		folder: VirtualFolderKind,
	},
	Ping,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
	Ok { folder: Option<VirtualFolderStatus> },
	Error { message: String },
}

pub fn socket_path() -> PathBuf {
	default_config_dir().join(SOCKET_NAME)
}

pub async fn run() -> Result<(), String> {
	let mut daemon = MountDaemon::open()?;
	daemon.restore_configured().await;

	#[cfg(unix)]
	return run_unix(&mut daemon).await;

	#[cfg(windows)]
	return run_windows(&mut daemon).await;
}

pub fn run_process() -> i32 {
	init_logging();

	let runtime = match tokio::runtime::Runtime::new() {
		Ok(runtime) => runtime,
		Err(err) => {
			eprintln!("backbeat-mountd: failed to start runtime: {err}");
			return 1;
		}
	};
	match runtime.block_on(run()) {
		Ok(()) => 0,
		Err(err) => {
			eprintln!("backbeat-mountd: {err}");
			1
		}
	}
}

fn init_logging() {
	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
	let stderr = fmt::layer()
		.with_writer(std::io::stderr)
		.with_filter(filter.clone());
	let log_dir = default_config_dir().join("logs");
	let Ok(()) = std::fs::create_dir_all(&log_dir) else {
		tracing_subscriber::registry().with(stderr).init();
		return;
	};
	let Ok(appender) = RollingFileAppender::builder()
		.rotation(Rotation::DAILY)
		.filename_prefix(LOG_PREFIX)
		.filename_suffix("log")
		.build(&log_dir)
	else {
		tracing_subscriber::registry().with(stderr).init();
		return;
	};

	let (writer, guard) = tracing_appender::non_blocking(appender);
	std::mem::forget(guard);
	tracing_subscriber::registry()
		.with(stderr)
		.with(
			fmt::layer()
				.with_ansi(false)
				.with_writer(writer)
				.with_filter(filter),
		)
		.init();
	prune_logs(&log_dir);
}

fn prune_logs(log_dir: &Path) {
	let cutoff = SystemTime::now()
		.checked_sub(Duration::from_secs(LOG_RETAIN_DAYS * 86_400))
		.unwrap_or(SystemTime::UNIX_EPOCH);
	let Ok(entries) = std::fs::read_dir(log_dir) else {
		return;
	};
	for entry in entries.flatten() {
		let path = entry.path();
		let is_mountd_log = path
			.file_name()
			.and_then(|name| name.to_str())
			.is_some_and(|name| {
				name.starts_with(LOG_PREFIX)
					&& path.extension().is_some_and(|extension| extension == "log")
			});
		if !is_mountd_log {
			continue;
		}
		let Ok(modified) = entry.metadata().and_then(|metadata| metadata.modified()) else {
			continue;
		};
		if modified < cutoff {
			let _ = std::fs::remove_file(path);
		}
	}
}

struct MountDaemon {
	store: Backbeat,
	store_revision: i64,
	mounts: HashMap<VirtualFolderKind, ActiveMount>,
	errors: HashMap<VirtualFolderKind, String>,
}

struct ReloadDebounce {
	deadline: Option<Instant>,
}

impl ReloadDebounce {
	fn new() -> Self {
		Self { deadline: None }
	}

	fn note_change(&mut self, now: Instant) {
		self.deadline = Some(now + RELOAD_DEBOUNCE);
	}

	fn take_due(&mut self, now: Instant) -> bool {
		if self.deadline.is_some_and(|deadline| deadline <= now) {
			self.deadline = None;
			true
		} else {
			false
		}
	}
}

impl MountDaemon {
	fn open() -> Result<Self, String> {
		let store = Backbeat::open().map_err(|err| err.to_string())?;
		let (_, store_revision) = store.should_refresh(-1).map_err(|err| err.to_string())?;
		Ok(Self {
			store,
			store_revision,
			mounts: HashMap::new(),
			errors: HashMap::new(),
		})
	}

	async fn restore_configured(&mut self) {
		let config = match load_config() {
			Ok(config) => config,
			Err(err) => {
				tracing::error!(%err, "failed to load virtual-folder configuration");
				return;
			}
		};

		for folder in VirtualFolderKind::ALL {
			if folder.enabled(&config.mounts)
				&& let Err(err) = self.mount(folder).await
			{
				self.errors.insert(folder, err.clone());
				tracing::warn!(?folder, %err, "failed to restore virtual folder");
			}
		}
	}

	async fn handle(&mut self, request: Request) -> Response {
		let result = match request {
			Request::Status { folder } => self.status(folder).map(Some),
			Request::SetPath { folder, mount_path } => self.set_path(folder, mount_path).map(Some),
			Request::Mount { folder } => self.mount(folder).await.map(Some),
			Request::Unmount { folder } => self.unmount(folder).map(Some),
			Request::Ping => Ok(None),
		};

		match result {
			Ok(folder) => Response::Ok { folder },
			Err(message) => Response::Error { message },
		}
	}

	fn status(&self, folder: VirtualFolderKind) -> Result<VirtualFolderStatus, String> {
		let config = load_config()?;
		let mount_path = folder.configured_path(&config.mounts);
		let mounted = mount_path
			.map(|path| is_mounted(path))
			.transpose()?
			.unwrap_or(false);
		Ok(VirtualFolderStatus {
			mount_path: mount_path.map(|path| path.display().to_string()),
			enabled: folder.enabled(&config.mounts),
			mounted,
			mount_supported: backend::fuse_available(),
			error: self.errors.get(&folder).cloned(),
		})
	}

	fn set_path(
		&mut self,
		folder: VirtualFolderKind,
		mount_path: String,
	) -> Result<VirtualFolderStatus, String> {
		let mount_path = PathBuf::from(mount_path.trim());
		validate_mount_path(&mount_path)?;

		let mut config = load_config()?;
		if let Some(path) = folder.configured_path(&config.mounts)
			&& (is_mounted(path)? || self.mounts.contains_key(&folder))
		{
			return Err("unmount this virtual folder before changing its mount folder".into());
		}
		folder.set_configured_path(&mut config.mounts, mount_path);
		write_config(&config)?;
		self.errors.remove(&folder);
		self.status(folder)
	}

	async fn mount(&mut self, folder: VirtualFolderKind) -> Result<VirtualFolderStatus, String> {
		if !backend::fuse_available() {
			let message = mount_unavailable_message();
			self.errors.insert(folder, message.clone());
			return Err(message);
		}

		let config = load_config()?;
		let mount_path = required_path(folder, &config)?.to_path_buf();
		std::fs::create_dir_all(&mount_path).map_err(|err| {
			format!(
				"failed to create mount folder {}: {err}",
				mount_path.display()
			)
		})?;
		if self.mounts.contains_key(&folder) || is_mounted(&mount_path)? {
			if !folder.enabled(&config.mounts) {
				let mut config = config;
				folder.set_enabled(&mut config.mounts, true);
				write_config(&config)?;
			}
			return self.status(folder);
		}

		let filesystem = folder.build_filesystem(self.store.clone()).await?;
		match backend::mount(filesystem, &mount_path, backend::MountConfig::default()) {
			Ok(mount) => {
				self.mounts.insert(folder, mount);
				let mut config = config;
				folder.set_enabled(&mut config.mounts, true);
				if let Err(err) = write_config(&config) {
					let _ = unmount_path(&mount_path);
					self.mounts.remove(&folder);
					return Err(err);
				}
				self.errors.remove(&folder);
				self.status(folder)
			}
			Err(err) => {
				let message = err.to_string();
				self.errors.insert(folder, message.clone());
				Err(message)
			}
		}
	}

	fn unmount(&mut self, folder: VirtualFolderKind) -> Result<VirtualFolderStatus, String> {
		let config = load_config()?;
		let mount_path = required_path(folder, &config)?.to_path_buf();
		unmount_path(&mount_path)?;
		self.mounts.remove(&folder);
		let mut config = config;
		folder.set_enabled(&mut config.mounts, false);
		write_config(&config)?;
		self.errors.remove(&folder);
		self.status(folder)
	}

	fn store_changed(&mut self) -> Result<bool, String> {
		let (changed, revision) = self
			.store
			.should_refresh(self.store_revision)
			.map_err(|err| err.to_string())?;
		self.store_revision = revision;
		Ok(changed)
	}

	async fn reload_enabled(&mut self) {
		let config = match load_config() {
			Ok(config) => config,
			Err(err) => {
				tracing::warn!(%err, "failed to reload virtual folders");
				return;
			}
		};

		for folder in VirtualFolderKind::ALL {
			if !folder.enabled(&config.mounts) {
				continue;
			}
			match self.reload(folder, &config).await {
				Ok(()) => {
					self.errors.remove(&folder);
					tracing::info!(?folder, "reloaded virtual folder after store change");
				}
				Err(err) => {
					self.errors.insert(folder, err.clone());
					tracing::warn!(?folder, %err, "failed to reload virtual folder after store change");
				}
			}
		}
	}

	async fn reload(&mut self, folder: VirtualFolderKind, config: &VfConfig) -> Result<(), String> {
		let mount_path = required_path(folder, config)?.to_path_buf();
		if self.mounts.contains_key(&folder) {
			unmount_path(&mount_path)?;
			self.mounts.remove(&folder);
		} else if is_mounted(&mount_path)? {
			return Err(format!(
				"cannot reload unmanaged mount at {}",
				mount_path.display()
			));
		}

		let filesystem = folder.build_filesystem(self.store.clone()).await?;
		let mount = backend::mount(filesystem, &mount_path, backend::MountConfig::default())
			.map_err(|err| err.to_string())?;
		self.mounts.insert(folder, mount);
		Ok(())
	}
}

async fn process_store_refresh(daemon: &mut MountDaemon, debounce: &mut ReloadDebounce) {
	let now = Instant::now();
	match daemon.store_changed() {
		Ok(true) => debounce.note_change(now),
		Ok(false) => {}
		Err(err) => tracing::warn!(%err, "failed to read store revision"),
	}
	if debounce.take_due(now) {
		daemon.reload_enabled().await;
	}
}

fn validate_mount_path(path: &Path) -> Result<(), String> {
	if path.as_os_str().is_empty() {
		return Err("mount folder cannot be empty".into());
	}
	if !path.is_absolute() {
		return Err("mount folder must be an absolute path".into());
	}
	match path.try_exists() {
		Ok(false) => Ok(()),
		Ok(true) if path.is_dir() => Ok(()),
		Ok(true) => Err(format!(
			"mount folder is not a directory: {}",
			path.display()
		)),
		Err(err) => Err(format!(
			"cannot access mount folder {}: {err}",
			path.display()
		)),
	}
}

fn load_config() -> Result<VfConfig, String> {
	VfConfig::load_with_overridden_dir(&default_config_dir()).map_err(|err| err.to_string())
}

fn write_config(config: &VfConfig) -> Result<(), String> {
	config
		.write_to_dir(&default_config_dir())
		.map(|_| ())
		.map_err(|err| err.to_string())
}

fn required_path(folder: VirtualFolderKind, config: &VfConfig) -> Result<&Path, String> {
	folder
		.configured_path(&config.mounts)
		.map(PathBuf::as_path)
		.ok_or_else(|| "choose a mount folder first".to_owned())
}

fn mount_unavailable_message() -> String {
	#[cfg(unix)]
	return "FUSE is not installed on this system. Install macFUSE on macOS or libfuse3 on Linux."
		.into();
	#[cfg(windows)]
	return "WinFsp is not installed on this system. Install from https://winfsp.dev.".into();
}

fn is_mounted(path: &Path) -> Result<bool, String> {
	#[cfg(unix)]
	{
		match backend::is_mount_point(path) {
			Ok(mounted) => Ok(mounted),
			Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
			Err(err) => Err(err.to_string()),
		}
	}
	#[cfg(windows)]
	{
		let _ = path;
		Ok(false)
	}
}

fn unmount_path(path: &Path) -> Result<(), String> {
	#[cfg(unix)]
	{
		if !is_mounted(path)? {
			return Ok(());
		}

		let path_string = path.display().to_string();
		let mut failures = Vec::new();
		#[cfg(target_os = "macos")]
		if try_unmount_command("diskutil", &["unmount", &path_string], &mut failures)? {
			return Ok(());
		}
		#[cfg(target_os = "linux")]
		for program in ["fusermount3", "fusermount"] {
			if try_unmount_command(program, &["-u", &path_string], &mut failures)? {
				return Ok(());
			}
		}
		if try_unmount_command("umount", &[&path_string], &mut failures)? {
			return Ok(());
		}
		Err(format!(
			"failed to unmount {}:\n{}",
			path.display(),
			failures.join("\n")
		))
	}

	#[cfg(windows)]
	{
		let _ = path;
		Ok(())
	}
}

#[cfg(unix)]
fn try_unmount_command(
	program: &str,
	args: &[&str],
	failures: &mut Vec<String>,
) -> Result<bool, String> {
	let output = match std::process::Command::new(program).args(args).output() {
		Ok(output) => output,
		Err(err) => {
			failures.push(format!("failed to run {program}: {err}"));
			return Ok(false);
		}
	};
	if output.status.success() {
		return Ok(true);
	}
	let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
	if detail.is_empty() {
		failures.push(format!("{program} exited with {}", output.status));
	} else {
		failures.push(format!("{program}: {detail}"));
	}
	Ok(false)
}

#[cfg(unix)]
async fn run_unix(daemon: &mut MountDaemon) -> Result<(), String> {
	use std::os::unix::fs::PermissionsExt;

	use tokio::net::UnixListener;

	let path = socket_path();
	if path.exists() {
		match tokio::net::UnixStream::connect(&path).await {
			Ok(_) => return Err("another backbeat-mountd instance is already running".into()),
			Err(_) => std::fs::remove_file(&path).map_err(|err| err.to_string())?,
		}
	}

	let listener = UnixListener::bind(&path).map_err(|err| err.to_string())?;
	std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
		.map_err(|err| err.to_string())?;
	tracing::info!(socket = %path.display(), "mount daemon ready");
	let mut store_refresh_poll = tokio::time::interval(STORE_REFRESH_POLL_INTERVAL);
	store_refresh_poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	let mut reload_debounce = ReloadDebounce::new();

	loop {
		tokio::select! {
			result = listener.accept() => {
				let (stream, _) = result.map_err(|err| err.to_string())?;
				serve_connection(daemon, stream).await;
			}
			_ = store_refresh_poll.tick() => {
				process_store_refresh(daemon, &mut reload_debounce).await;
			}
		}
	}
}

#[cfg(windows)]
async fn run_windows(daemon: &mut MountDaemon) -> Result<(), String> {
	use tokio::net::windows::named_pipe::ServerOptions;

	tracing::info!(pipe = PIPE_NAME, "mount daemon ready");
	let mut first = true;
	let mut store_refresh_poll = tokio::time::interval(STORE_REFRESH_POLL_INTERVAL);
	store_refresh_poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
	let mut reload_debounce = ReloadDebounce::new();
	loop {
		let mut options = ServerOptions::new();
		if first {
			options.first_pipe_instance(true);
			first = false;
		}
		let server = options.create(PIPE_NAME).map_err(|err| err.to_string())?;
		tokio::select! {
			result = server.connect() => {
				result.map_err(|err| err.to_string())?;
				serve_connection(daemon, server).await;
			}
			_ = store_refresh_poll.tick() => {
				process_store_refresh(daemon, &mut reload_debounce).await;
			}
		}
	}
}

async fn serve_connection<S>(daemon: &mut MountDaemon, stream: S)
where
	S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
	use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

	let (reader, mut writer) = tokio::io::split(stream);
	let mut lines = BufReader::new(reader).lines();
	while let Ok(Some(line)) = lines.next_line().await {
		let response = match serde_json::from_str::<Request>(&line) {
			Ok(request) => daemon.handle(request).await,
			Err(err) => Response::Error {
				message: format!("invalid mount daemon request: {err}"),
			},
		};
		let Ok(mut payload) = serde_json::to_vec(&response) else {
			break;
		};
		payload.push(b'\n');
		if writer.write_all(&payload).await.is_err() {
			break;
		}
	}
}

pub mod client {
	use super::{Request, Response, socket_path};

	pub async fn request(request: Request) -> Result<Response, String> {
		#[cfg(unix)]
		return request_unix(request).await;
		#[cfg(windows)]
		return request_windows(request).await;
	}

	#[cfg(unix)]
	async fn request_unix(request: Request) -> Result<Response, String> {
		use tokio::net::UnixStream;

		let stream = UnixStream::connect(socket_path())
			.await
			.map_err(|err| format!("mount daemon is not running: {err}"))?;
		request_over_stream(stream, request).await
	}

	#[cfg(windows)]
	async fn request_windows(request: Request) -> Result<Response, String> {
		use tokio::net::windows::named_pipe::ClientOptions;

		let stream = ClientOptions::new()
			.open(super::PIPE_NAME)
			.map_err(|err| format!("mount daemon is not running: {err}"))?;
		request_over_stream(stream, request).await
	}

	async fn request_over_stream<S>(stream: S, request: Request) -> Result<Response, String>
	where
		S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
	{
		use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

		let (reader, mut writer) = tokio::io::split(stream);
		let mut payload = serde_json::to_vec(&request).map_err(|err| err.to_string())?;
		payload.push(b'\n');
		writer
			.write_all(&payload)
			.await
			.map_err(|err| err.to_string())?;
		let mut lines = BufReader::new(reader).lines();
		let line = lines
			.next_line()
			.await
			.map_err(|err| err.to_string())?
			.ok_or_else(|| "mount daemon closed the connection".to_owned())?;
		serde_json::from_str(&line).map_err(|err| format!("invalid mount daemon response: {err}"))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn protocol_round_trips_a_mount_request() {
		let request = Request::SetPath {
			folder: VirtualFolderKind::Kshoot,
			mount_path: "/games/kshoot".to_owned(),
		};
		let json = serde_json::to_string(&request).unwrap();
		assert_eq!(
			json,
			r#"{"command":"set_path","folder":"kshoot","mount_path":"/games/kshoot"}"#
		);
		let parsed: Request = serde_json::from_str(&json).unwrap();
		assert!(matches!(
			parsed,
			Request::SetPath {
				folder: VirtualFolderKind::Kshoot,
				mount_path
			} if mount_path == "/games/kshoot"
		));
	}

	#[test]
	fn status_serializes_for_the_gui() {
		let status = VirtualFolderStatus {
			mount_path: Some("/games/bms".to_owned()),
			enabled: false,
			mounted: false,
			mount_supported: true,
			error: Some("FUSE is unavailable".to_owned()),
		};
		let response = Response::Ok {
			folder: Some(status),
		};
		let json = serde_json::to_value(response).unwrap();
		assert_eq!(json["status"], "ok");
		assert_eq!(json["folder"]["error"], "FUSE is unavailable");
	}

	#[test]
	fn mount_path_must_be_absolute() {
		assert_eq!(
			validate_mount_path(Path::new("K-Shoot")).unwrap_err(),
			"mount folder must be an absolute path"
		);
	}

	#[test]
	fn reload_debounce_waits_and_extends_for_new_changes() {
		let start = Instant::now();
		let mut debounce = ReloadDebounce::new();

		debounce.note_change(start);
		assert!(!debounce.take_due(start + RELOAD_DEBOUNCE - Duration::from_millis(1)));

		let later = start + Duration::from_millis(300);
		debounce.note_change(later);
		assert!(!debounce.take_due(start + RELOAD_DEBOUNCE));
		assert!(debounce.take_due(later + RELOAD_DEBOUNCE));
		assert!(!debounce.take_due(later + RELOAD_DEBOUNCE));
	}
}
