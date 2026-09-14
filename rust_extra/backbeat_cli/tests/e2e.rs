#![allow(unreachable_pub, clippy::all, clippy::restriction)]
#![allow(clippy::pedantic, clippy::nursery, clippy::cargo)]

//! End-to-end CLI tests driven through the real `bkb` binary.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::thread;

use assert_cmd::Command;
use backbeat_core::{AssetId, BackbeatFile, BbZipEntry, BbZipReader, Sha256};
use backbeat_sdk::Backbeat;
use backbeat_store_config::{BackbeatConfig, BackbeatServerInfo, ServerConfig};
use predicates::prelude::*;

struct TestStore {
	config_dir: tempfile::TempDir,
}

impl TestStore {
	fn new() -> Self {
		let config_dir = tempfile::TempDir::new().unwrap();
		let data_dir = config_dir.path().join("data");
		let mut config = BackbeatConfig::default();
		config.store.path = data_dir;
		config.servers = vec![
			ServerConfig {
				url: "https://localhost".into(),
			},
			ServerConfig {
				url: "https://test.example.com".into(),
			},
		];
		config.info = Some(BackbeatServerInfo {
			name: "Test Server".into(),
			contact: None,
		});
		config.write_to_dir(config_dir.path()).unwrap();
		Self { config_dir }
	}

	fn cmd(&self) -> Command {
		let mut command = Command::cargo_bin("bkb").unwrap();
		command.env("BKB_OVERRIDE_CONFIG_DIR", self.config_dir.path());
		command
	}
}

fn fixture(relative: impl AsRef<Path>) -> PathBuf {
	PathBuf::from(env!("CARGO_MANIFEST_DIR"))
		.join("../../fixtures")
		.join(relative)
}

fn spawn_invalid_collection_server() -> (String, thread::JoinHandle<()>) {
	let listener = TcpListener::bind("127.0.0.1:0").unwrap();
	let address = listener.local_addr().unwrap();
	let server = thread::spawn(move || {
		let (mut stream, _) = listener.accept().unwrap();
		let mut request = [0; 4096];
		let size = stream.read(&mut request).unwrap();
		let request = String::from_utf8_lossy(&request[..size]);
		assert!(request.starts_with("GET /header.json "));

		let body = b"<html><body>Not a collection</body></html>";
		write!(
			stream,
			"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
			body.len()
		)
		.unwrap();
		stream.write_all(body).unwrap();
	});

	(format!("http://{address}"), server)
}

#[test]
fn collection_add_identifies_an_invalid_header() {
	let store = TestStore::new();
	let (url, server) = spawn_invalid_collection_server();

	store
		.cmd()
		.args(["collection", "add", &url])
		.assert()
		.failure()
		.stderr(predicate::str::contains("header.json was invalid"))
		.stderr(predicate::str::contains(format!(
			"{url} is not a valid Backbeat collection"
		)))
		.stderr(predicate::str::contains("JSON error:").not());

	server.join().unwrap();
}

#[test]
fn collection_is_a_top_level_command() {
	let store = TestStore::new();

	store
		.cmd()
		.args(["collection", "--help"])
		.assert()
		.success()
		.stdout(predicate::str::contains("install-data"));

	store
		.cmd()
		.args(["store", "collection", "--help"])
		.assert()
		.failure();
}

#[test]
fn info_shows_paths_and_servers() {
	let store = TestStore::new();
	store
		.cmd()
		.args(["info"])
		.assert()
		.success()
		.stdout(predicate::str::contains("Config"))
		.stdout(predicate::str::contains("Data"))
		.stdout(predicate::str::contains("Servers"))
		.stdout(predicate::str::is_match(r"Charts:.*\b0\b").unwrap());
}

#[test]
fn import_bb_then_info_stats() {
	let store = TestStore::new();
	let bb = fixture("bb/0x1311.bb");

	store
		.cmd()
		.args(["import", bb.to_str().unwrap()])
		.assert()
		.success()
		.stdout(predicate::str::contains("added"));

	store
		.cmd()
		.args(["info"])
		.assert()
		.success()
		.stdout(predicate::str::is_match(r"Charts:.*\b1\b").unwrap());
}

#[test]
fn import_bb_with_unrecognised_chart_filename() {
	let store = TestStore::new();
	let bb = fixture("bb/foo.googus.bb");

	store
		.cmd()
		.args(["import", bb.to_str().unwrap()])
		.assert()
		.success()
		.stdout(predicate::str::contains("added"));

	store
		.cmd()
		.args(["info"])
		.assert()
		.success()
		.stdout(predicate::str::is_match(r"Charts:.*\b1\b").unwrap());
}

#[test]
fn import_zero_byte_chart_fixture() {
	let store = TestStore::new();
	let bb = fixture("bb/zero-byte-chart.bb");

	store
		.cmd()
		.args(["import", bb.to_str().unwrap()])
		.assert()
		.success()
		.stdout(predicate::str::contains("added"));

	store
		.cmd()
		.args(["info"])
		.assert()
		.success()
		.stdout(predicate::str::is_match(r"Charts:.*\b1\b").unwrap());
}

#[test]
fn import_dir_recursive() {
	let store = TestStore::new();
	let bundles = fixture("bb");

	store
		.cmd()
		.args(["import", "--recursive", bundles.to_str().unwrap()])
		.assert()
		.success()
		.stdout(predicate::str::contains("Added 3 bundles"));

	store
		.cmd()
		.args(["info"])
		.assert()
		.success()
		.stdout(predicate::str::is_match(r"Charts:.*\b3\b").unwrap());
}

#[test]
fn add_command_is_removed() {
	let store = TestStore::new();
	store
		.cmd()
		.args(["add", fixture("bb/0x1311.bb").to_str().unwrap()])
		.assert()
		.failure()
		.stderr(predicate::str::contains("unrecognized subcommand 'add'"));
}

#[test]
fn import_and_convert_and_import_are_top_level_commands() {
	let store = TestStore::new();
	store
		.cmd()
		.arg("--help")
		.assert()
		.success()
		.stdout(predicate::str::contains("import"))
		.stdout(predicate::str::contains("convert-and-import"));
}

#[test]
fn import_rejects_source_charts() {
	let store = TestStore::new();
	let source = fixture("bms/_AK_minec24.bms");

	store
		.cmd()
		.args(["import", source.to_str().unwrap()])
		.assert()
		.failure()
		.stderr(predicate::str::contains("is not a `.bb` or `.bbzip` file"));
}

#[test]
fn convert_and_import_rejects_bb_files() {
	let store = TestStore::new();
	let bundle = fixture("bb/0x1311.bb");

	store
		.cmd()
		.args(["convert-and-import", bundle.to_str().unwrap()])
		.assert()
		.failure()
		.stderr(predicate::str::contains("failed to convert and import"));
}

#[test]
fn guts_inspect_rejects_an_opaque_bundle() {
	let store = TestStore::new();
	let bb = fixture("bb/foo.googus.bb");

	store
		.cmd()
		.args(["guts", "inspect", "-o", "json", bb.to_str().unwrap()])
		.assert()
		.failure()
		.stderr(predicate::str::contains("Unrecognised file extension"));
}

#[test]
fn guts_media_info_outputs_json() {
	let store = TestStore::new();
	let mp3 = fixture("charts/sm/0x1311/0x1311.mp3");

	store
		.cmd()
		.args(["guts", "media-info", "-o", "json", mp3.to_str().unwrap()])
		.assert()
		.success()
		.stdout(predicate::str::contains("\"kind\": \"audio\""))
		.stdout(predicate::str::contains("\"mime_type\": \"audio/mpeg\""));
}

// #[test]
// fn convert_and_import_bms_then_check() {
// 	let store = TestStore::new();
// 	let bms = fixture("bms/_AK_minec24.bms");

// 	store
// 		.cmd()
// 		.args(["convert-and-import", bms.to_str().unwrap()])
// 		.assert()
// 		.success();

// 	store
// 		.cmd()
// 		.args(["store", "check"])
// 		.assert()
// 		.success()
// 		.stdout(predicate::str::contains("OK"))
// 		.stdout(predicate::str::is_match(r"Charts.*\b1\b").unwrap());
// }

#[test]
fn chart_pack_fractures_sm_and_meld_recombines_bundles() {
	let store = TestStore::new();
	let source = store.config_dir.path().join("song.sm");
	std::fs::write(
		&source,
		b"#TITLE:Test;\n#BPMS:0=120;\n#NOTES:dance-single:Hard:Hard:9:0:0000;\n#NOTES:dance-single:Easy:Easy:3:0:0000;\n",
	)
	.unwrap();

	store
		.cmd()
		.args(["guts", "pack", source.to_str().unwrap()])
		.assert()
		.success();

	let first = source.with_file_name("song.1.bb");
	let second = source.with_file_name("song.2.bb");
	for path in [&first, &second] {
		let bundle = BackbeatFile::from_file(path).unwrap();
		let chart = bundle.chart.decompress();
		assert_eq!(count_notes(&chart), 1);
	}

	let output = source.with_file_name("melded.bb");
	store
		.cmd()
		.args([
			"guts",
			"meld",
			output.to_str().unwrap(),
			second.to_str().unwrap(),
			first.to_str().unwrap(),
		])
		.assert()
		.success();

	let melded = BackbeatFile::from_file(output).unwrap();
	let chart = melded.chart.decompress();
	assert_eq!(count_notes(&chart), 2);
}

#[test]
fn guts_pack_bbzip_includes_the_manifest_and_assets() {
	let store = TestStore::new();
	let source = store.config_dir.path().join("song.bms");
	std::fs::write(&source, b"#TITLE Test;\n#WAV01 kick.wav\n").unwrap();
	std::fs::write(source.with_file_name("kick.wav"), b"audio").unwrap();

	store
		.cmd()
		.args(["guts", "pack", "-z", source.to_str().unwrap()])
		.assert()
		.success();

	let output = source.with_file_name("song.bbzip");
	let mut archive = BbZipReader::new(std::fs::File::open(output).unwrap()).unwrap();
	let mut manifests = 0;
	let mut assets = Vec::new();
	for index in 0..archive.len() {
		match archive.get_entry(index).unwrap().unwrap() {
			BbZipEntry::Bb(_) => manifests += 1,
			BbZipEntry::Asset(mut asset) => {
				let mut bytes = Vec::new();
				asset.read_to_end(&mut bytes).unwrap();
				assets.push(bytes);
			}
		}
	}

	assert_eq!(manifests, 1);
	assert!(assets.contains(&b"audio".to_vec()));
}

fn imported_export_fixture() -> (TestStore, BackbeatFile, &'static [u8]) {
	let store = TestStore::new();
	let source_dir = store.config_dir.path().join("export-source");
	std::fs::create_dir_all(&source_dir).unwrap();
	let source = source_dir.join("song.bms");
	let chart: &'static [u8] = b"#ARTIST Tester\n#TITLE Export\n#WAV01 kick.wav\n";
	std::fs::write(&source, chart).unwrap();
	std::fs::write(source_dir.join("kick.wav"), b"audio").unwrap();
	let bb = backbeat_packager::package(&source).unwrap().remove(0);

	store
		.cmd()
		.args(["convert-and-import", source.to_str().unwrap()])
		.assert()
		.success();

	(store, bb, chart)
}

#[test]
fn export_bundle_writes_a_named_folder_inside_the_output_directory() {
	let (store, bb, chart) = imported_export_fixture();

	let output = store.config_dir.path().join("folder-export");
	store
		.cmd()
		.args([
			"export",
			&bb.bundle_id().to_string(),
			"-o",
			output.to_str().unwrap(),
		])
		.assert()
		.success();
	let name = bb.desc.to_string();
	let folder = output.join(&name);
	assert_eq!(std::fs::read(folder.join("song.bms")).unwrap(), chart);
	assert_eq!(std::fs::read(folder.join("kick.wav")).unwrap(), b"audio");
	assert!(!output.join("song.bms").exists());
}

#[test]
fn export_chart_writes_a_named_bbzip_inside_the_output_directory() {
	let (store, bb, _) = imported_export_fixture();

	let archive_dir = store.config_dir.path().join("chart-export");
	let chart_id = format!("sha256/{}", bb.chart_sha256());
	store
		.cmd()
		.args([
			"export",
			&chart_id,
			"-z",
			"-o",
			archive_dir.to_str().unwrap(),
		])
		.assert()
		.success();

	let name = bb.desc.to_string();
	let mut archive = BbZipReader::new(
		std::fs::File::open(archive_dir.join(name).with_extension("bbzip")).unwrap(),
	)
	.unwrap();
	let mut manifests = 0;
	let mut assets = Vec::new();
	for index in 0..archive.len() {
		match archive.get_entry(index).unwrap().unwrap() {
			BbZipEntry::Bb(_) => manifests += 1,
			BbZipEntry::Asset(mut asset) => {
				let mut bytes = Vec::new();
				asset.read_to_end(&mut bytes).unwrap();
				assets.push(bytes);
			}
		}
	}
	assert_eq!(manifests, 1);
	assert_eq!(assets, vec![b"audio"]);
}

#[test]
fn export_rejects_ids_that_are_neither_bundle_nor_chart_ids() {
	let store = TestStore::new();
	store
		.cmd()
		.args(["export", "not-an-id"])
		.assert()
		.failure()
		.stderr(predicate::str::contains(
			"expected a bundle ID beginning with `b-` or a chart ID containing `/`",
		));
}

#[test]
fn guts_deps_warns_about_missing_references() {
	let store = TestStore::new();
	let source = store.config_dir.path().join("song.sm");
	std::fs::write(
		&source,
		b"#TITLE:Test;\n#MUSIC:missing.ogg;\n#NOTES:dance-single:A:Hard:9:0:0000;\n",
	)
	.unwrap();

	store
		.cmd()
		.args(["guts", "deps", source.to_str().unwrap()])
		.assert()
		.success()
		.stderr(predicate::str::contains(
			"warning: referenced dependency does not exist: missing.ogg",
		));
}

#[test]
fn guts_deps_strict_fails_about_missing_references() {
	let store = TestStore::new();
	let source = store.config_dir.path().join("song.sm");
	std::fs::write(
		&source,
		b"#TITLE:Test;\n#MUSIC:missing.ogg;\n#NOTES:dance-single:A:Hard:9:0:0000;\n",
	)
	.unwrap();

	store
		.cmd()
		.args(["guts", "deps", "--strict", source.to_str().unwrap()])
		.assert()
		.failure()
		.stderr(predicate::str::contains(
			"missing referenced dependencies: missing.ogg",
		));
}

#[test]
fn guts_missingdeps_reports_missing_references_for_one_chart() {
	let store = TestStore::new();
	let source = store.config_dir.path().join("song.sm");
	std::fs::write(
		&source,
		b"#TITLE:Test;\n#MUSIC:missing.ogg;\n#NOTES:dance-single:A:Hard:9:0:0000;\n",
	)
	.unwrap();

	store
		.cmd()
		.args(["guts", "missing-deps", source.to_str().unwrap()])
		.assert()
		.success()
		.stdout(format!("{}\tmissing.ogg\n", source.display()));
}

#[test]
fn guts_missingdeps_ignores_unused_bms_lookup_assets() {
	let store = TestStore::new();
	let source = store.config_dir.path().join("song.bms");
	std::fs::write(
		&source,
		b"#WAV01 used-audio.wav\n#WAV02 unused-audio.wav\n#BMP01 used-image.png\n#BMP02 unused-image.png\n#00111:0100\n#00104:0100\n",
	)
	.unwrap();

	store
		.cmd()
		.args(["guts", "missing-deps", source.to_str().unwrap()])
		.assert()
		.success()
		.stdout(format!(
			"{}\tused-audio.wav\n{}\tused-image.png\n",
			source.display(),
			source.display(),
		));
}

#[test]
fn guts_missingdeps_recursive_reports_the_source_of_each_missing_reference() {
	let store = TestStore::new();
	let root = store.config_dir.path().join("charts");
	let first = root.join("a.sm");
	let second = root.join("nested").join("b.sm");
	std::fs::create_dir_all(second.parent().unwrap()).unwrap();

	for source in [&first, &second] {
		std::fs::write(
			source,
			b"#TITLE:Test;\n#MUSIC:missing.ogg;\n#NOTES:dance-single:A:Hard:9:0:0000;\n",
		)
		.unwrap();
	}
	std::fs::write(root.join("ignored.txt"), b"not a chart").unwrap();

	store
		.cmd()
		.args([
			"guts",
			"missing-deps",
			"--recursive",
			root.to_str().unwrap(),
		])
		.assert()
		.success()
		.stdout(format!(
			"{}\tmissing.ogg\n{}\tmissing.ogg\n",
			first.display(),
			second.display()
		));
}

#[test]
fn guts_missingdeps_recursive_requires_a_directory() {
	let store = TestStore::new();
	let source = store.config_dir.path().join("song.sm");
	std::fs::write(&source, b"#TITLE:Test;\n").unwrap();

	store
		.cmd()
		.args(["guts", "missing-deps", "-r", source.to_str().unwrap()])
		.assert()
		.failure()
		.stderr(predicate::str::contains("--recursive requires a directory"));
}

#[test]
fn guts_missingdeps_recursive_continues_after_an_unpackageable_chart() {
	let store = TestStore::new();
	let root = store.config_dir.path().join("charts");
	std::fs::create_dir_all(&root).unwrap();
	let invalid = root.join("a-invalid.sm");
	let missing = root.join("b-missing.sm");

	std::fs::write(&invalid, b"#TITLE:Invalid;\n#MUSIC:..;\n").unwrap();
	std::fs::write(
		&missing,
		b"#TITLE:Missing;\n#MUSIC:missing.ogg;\n#NOTES:dance-single:A:Hard:9:0:0000;\n",
	)
	.unwrap();

	store
		.cmd()
		.args([
			"guts",
			"missing-deps",
			"--recursive",
			root.to_str().unwrap(),
		])
		.assert()
		.success()
		.stdout(format!("{}\tmissing.ogg\n", missing.display()))
		.stderr(predicate::str::contains(format!(
			"warning: failed to package {}",
			invalid.display()
		)))
		.stderr(predicate::str::contains("asset path .. is disallowed").count(1));
}

#[test]
fn guts_pack_strict_fails_about_missing_references() {
	let store = TestStore::new();
	let source = store.config_dir.path().join("song.sm");
	std::fs::write(
		&source,
		b"#TITLE:Test;\n#MUSIC:missing.ogg;\n#NOTES:dance-single:A:Hard:9:0:0000;\n",
	)
	.unwrap();

	store
		.cmd()
		.args(["guts", "pack", "--strict", source.to_str().unwrap()])
		.assert()
		.failure()
		.stderr(predicate::str::contains(
			"missing referenced dependencies: missing.ogg",
		));

	assert!(!source.with_extension("bb").exists());
}

fn count_notes(chart: &[u8]) -> usize {
	chart
		.windows(b"#NOTES".len())
		.filter(|window| *window == b"#NOTES")
		.count()
}

#[test]
fn convert_and_import_dir_recursive() {
	let store = TestStore::new();
	let charts = fixture("charts");

	store
		.cmd()
		.args(["convert-and-import", "-r", charts.to_str().unwrap()])
		.assert()
		.success()
		.stdout(predicate::str::contains("Converted and imported"));

	store
		.cmd()
		.args(["info"])
		.assert()
		.success()
		.stdout(predicate::str::is_match(r"Charts:.*[1-9]").unwrap());
}

#[test]
fn asset_prune_dry_run_clean_store() {
	let store = TestStore::new();
	store
		.cmd()
		.args(["store", "asset-prune", "--dry-run"])
		.assert()
		.success()
		.stdout(predicate::str::contains("Nothing to prune."));
}

#[test]
fn asset_and_disk_prune() {
	let store = TestStore::new();
	let source = store.config_dir.path().join("unused-asset");
	let data = b"unused asset";
	fs_err::write(&source, data).unwrap();
	let asset_id = AssetId(Sha256::checksum_bytes(data));
	let api = Backbeat::open_with_overridden_config_dir(store.config_dir.path()).unwrap();
	api.import_asset(&source).unwrap();
	let disk_orphan = AssetId(Sha256::checksum_bytes(b"disk orphan"));
	let disk_orphan_path = api
		.store_dir()
		.join(Backbeat::ASSETS_DIR)
		.join(disk_orphan.fanned_path());
	fs_err::create_dir_all(disk_orphan_path.parent().unwrap()).unwrap();
	fs_err::write(&disk_orphan_path, b"disk orphan").unwrap();
	drop(api);

	store
		.cmd()
		.args(["store", "asset-prune", "--dry-run"])
		.assert()
		.success()
		.stdout(predicate::str::contains("Unused assets: 1"))
		.stdout(predicate::str::contains("nothing was removed"));
	assert!(
		Backbeat::open_with_overridden_config_dir(store.config_dir.path())
			.unwrap()
			.has_asset(asset_id)
			.unwrap()
	);

	store
		.cmd()
		.args(["store", "asset-prune", "--yes"])
		.assert()
		.success()
		.stdout(predicate::str::contains("Pruned 1 asset"));
	assert!(
		!Backbeat::open_with_overridden_config_dir(store.config_dir.path())
			.unwrap()
			.has_asset(asset_id)
			.unwrap()
	);
	assert!(disk_orphan_path.exists());

	store
		.cmd()
		.args(["store", "disk-prune", "--yes"])
		.assert()
		.success()
		.stdout(predicate::str::contains("Pruned 1 file"));
	assert!(!disk_orphan_path.exists());
}

#[test]
fn old_prune_command_is_removed() {
	TestStore::new()
		.cmd()
		.args(["store", "prune"])
		.assert()
		.failure()
		.stderr(predicate::str::contains("unrecognized subcommand 'prune'"));
}

#[test]
fn config_get_set_roundtrip() {
	let store = TestStore::new();

	store
		.cmd()
		.args(["config", "set", "store.inline", "32Ki"])
		.assert()
		.success();

	store
		.cmd()
		.args(["config", "get", "store.inline"])
		.assert()
		.success()
		.stdout("32Ki\n");
}

#[test]
fn double_import_is_idempotent() {
	let store = TestStore::new();
	let bb = fixture("bb/0x1311.bb");
	let path = bb.to_str().unwrap();

	store.cmd().args(["import", path]).assert().success();
	store.cmd().args(["import", path]).assert().success();

	store
		.cmd()
		.args(["info"])
		.assert()
		.success()
		.stdout(predicate::str::is_match(r"Charts:.*\b1\b").unwrap());
}

#[test]
fn import_nonexistent_path_fails() {
	let store = TestStore::new();
	store
		.cmd()
		.args(["import", "/does/not/exist/backbeat-missing.bb"])
		.assert()
		.failure()
		.stderr(predicate::str::contains("failed to read .bb file"));
}

#[test]
fn convert_and_import_nonexistent_path_fails() {
	let store = TestStore::new();
	store
		.cmd()
		.args(["convert-and-import", "/does/not/exist/backbeat-missing.sm"])
		.assert()
		.failure()
		.stderr(predicate::str::contains("failed to convert and import"));
}
