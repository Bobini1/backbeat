use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use backbeat_vf_mountd::{Request, Response, client};

const DAEMON_ARG: &str = "--mount-daemon";

pub async fn ensure_running() -> Result<(), String> {
	if ping().await {
		return Ok(());
	}

	#[cfg(debug_assertions)]
	spawn(std::env::current_exe().map_err(|err| err.to_string())?)?;

	#[cfg(not(debug_assertions))]
	install_and_start_service()?;

	for _ in 0..30 {
		tokio::time::sleep(Duration::from_millis(100)).await;
		if ping().await {
			return Ok(());
		}
	}
	Err("the Backbeat Virtual Folders mount daemon did not start".into())
}

async fn ping() -> bool {
	matches!(
		client::request(Request::Ping).await,
		Ok(Response::Ok { .. })
	)
}

#[cfg(debug_assertions)]
fn spawn(executable: PathBuf) -> Result<(), String> {
	Command::new(executable)
		.arg(DAEMON_ARG)
		.spawn()
		.map(|_| ())
		.map_err(|err| format!("failed to launch mount daemon: {err}"))
}

#[cfg(not(debug_assertions))]
fn install_and_start_service() -> Result<(), String> {
	let executable = install_daemon_executable()?;
	#[cfg(target_os = "macos")]
	return install_launch_agent(&executable);
	#[cfg(target_os = "linux")]
	return install_systemd_service(&executable);
	#[cfg(windows)]
	return install_scheduled_task(&executable);
}

#[cfg(not(debug_assertions))]
fn install_daemon_executable() -> Result<PathBuf, String> {
	let source = std::env::current_exe().map_err(|err| err.to_string())?;
	let version = env!("CARGO_PKG_VERSION");
	let dirs = directories::BaseDirs::new()
		.ok_or_else(|| "could not determine a local application data directory".to_owned())?;
	let destination_dir = dirs
		.data_local_dir()
		.join("backbeat-vf")
		.join("mountd")
		.join(version);
	std::fs::create_dir_all(&destination_dir).map_err(|err| err.to_string())?;
	let file_name = source
		.file_name()
		.ok_or_else(|| "Backbeat Virtual Folders executable has no filename".to_owned())?;
	let destination = destination_dir.join(file_name);
	let source_is_newer = match (
		std::fs::metadata(&source).and_then(|metadata| metadata.modified()),
		std::fs::metadata(&destination).and_then(|metadata| metadata.modified()),
	) {
		(Ok(source), Ok(destination)) => source > destination,
		_ => true,
	};
	if !destination.is_file() || source_is_newer {
		std::fs::copy(&source, &destination).map_err(|err| err.to_string())?;
		#[cfg(unix)]
		{
			use std::os::unix::fs::PermissionsExt;
			std::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o755))
				.map_err(|err| err.to_string())?;
		}
	}
	Ok(destination)
}

#[cfg(all(not(debug_assertions), target_os = "macos"))]
fn install_launch_agent(executable: &std::path::Path) -> Result<(), String> {
	let dirs = directories::BaseDirs::new()
		.ok_or_else(|| "could not determine home directory".to_owned())?;
	let plist = dirs
		.home_dir()
		.join("Library/LaunchAgents/ac.backbeat.vf.mountd.plist");
	let contents = format!(
		"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>Label</key><string>ac.backbeat.vf.mountd</string><key>ProgramArguments</key><array><string>{}</string><string>{DAEMON_ARG}</string></array><key>RunAtLoad</key><true/><key>KeepAlive</key><true/></dict></plist>\n",
		executable.display()
	);
	std::fs::create_dir_all(plist.parent().expect("LaunchAgents has a parent"))
		.map_err(|err| err.to_string())?;
	std::fs::write(&plist, contents).map_err(|err| err.to_string())?;
	let domain = format!("gui/{}", current_user_id()?);
	let _ = Command::new("launchctl")
		.args(["bootout", &domain, &plist.display().to_string()])
		.status();
	Command::new("launchctl")
		.args(["bootstrap", &domain, &plist.display().to_string()])
		.status()
		.map_err(|err| format!("failed to install LaunchAgent: {err}"))?
		.success()
		.then_some(())
		.ok_or_else(|| {
			"launchctl could not install the Backbeat Virtual Folders service".to_owned()
		})
}

#[cfg(all(not(debug_assertions), target_os = "macos"))]
fn current_user_id() -> Result<String, String> {
	let output = Command::new("id")
		.arg("-u")
		.output()
		.map_err(|err| format!("failed to identify current user: {err}"))?;
	if !output.status.success() {
		return Err("could not identify current user for LaunchAgent".into());
	}
	String::from_utf8(output.stdout)
		.map_err(|err| err.to_string())
		.map(|value| value.trim().to_owned())
}

#[cfg(all(not(debug_assertions), target_os = "linux"))]
fn install_systemd_service(executable: &std::path::Path) -> Result<(), String> {
	let dirs = directories::BaseDirs::new()
		.ok_or_else(|| "could not determine config directory".to_owned())?;
	let unit = dirs
		.config_dir()
		.join("systemd/user/backbeat-vf-mountd.service");
	let contents = format!(
		"[Unit]\nDescription=Backbeat Virtual Folders mount daemon\n\n[Service]\nExecStart={} {DAEMON_ARG}\nRestart=on-failure\n\n[Install]\nWantedBy=default.target\n",
		executable.display()
	);
	std::fs::create_dir_all(unit.parent().expect("systemd user unit has a parent"))
		.map_err(|err| err.to_string())?;
	std::fs::write(&unit, contents).map_err(|err| err.to_string())?;
	for args in [
		["--user", "daemon-reload"].as_slice(),
		["--user", "enable", "--now", "backbeat-vf-mountd.service"].as_slice(),
	] {
		let status = Command::new("systemctl")
			.args(args)
			.status()
			.map_err(|err| err.to_string())?;
		if !status.success() {
			return Err("systemctl could not install the Backbeat Virtual Folders service".into());
		}
	}
	Ok(())
}

#[cfg(all(not(debug_assertions), windows))]
fn install_scheduled_task(executable: &std::path::Path) -> Result<(), String> {
	let task_command = format!("\\\"{}\\\" {DAEMON_ARG}", executable.display());
	let status = Command::new("schtasks")
		.args([
			"/create",
			"/tn",
			"Backbeat Virtual Folders Mount Daemon",
			"/tr",
			&task_command,
			"/sc",
			"onlogon",
			"/rl",
			"limited",
			"/f",
		])
		.status()
		.map_err(|err| format!("failed to install scheduled task: {err}"))?;
	if !status.success() {
		return Err("schtasks could not install the Backbeat Virtual Folders service".into());
	}
	let _ = Command::new("schtasks")
		.args(["/run", "/tn", "Backbeat Virtual Folders Mount Daemon"])
		.status();
	Ok(())
}
