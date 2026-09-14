use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use backbeat_core::{BundleId, ChartId};
use clap::Args;

use crate::{store_util, style};

#[derive(Debug, Clone)]
pub enum ExportIdentifier {
	Bundle(BundleId),
	Chart(ChartId),
}

impl FromStr for ExportIdentifier {
	type Err = String;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if value.starts_with("b-") {
			value
				.parse::<BundleId>()
				.map(Self::Bundle)
				.map_err(|err| err.to_string())
		} else if value.contains('/') {
			value
				.parse::<ChartId>()
				.map(Self::Chart)
				.map_err(|err| err.to_string())
		} else {
			Err("expected a bundle ID beginning with `b-` or a chart ID containing `/`".to_owned())
		}
	}
}

/// Export a chart from the store and render it to a directory.
#[derive(Debug, Args)]
pub struct ExportCommand {
	/// Bundle ID or canonical `algorithm/value` chart ID to export.
	pub identifier: ExportIdentifier,

	/// Parent directory for the export. Defaults to `Documents/Charts`.
	#[arg(short, long)]
	pub output: Option<PathBuf>,

	/// Write a `.bbzip` file instead of a folder.
	#[arg(short = 'z', long)]
	pub bbzip: bool,
}

impl ExportCommand {
	pub fn run(self) -> anyhow::Result<()> {
		let store = store_util::open()?;
		let bb = match &self.identifier {
			ExportIdentifier::Bundle(id) => store.get_bundle(*id)?,
			ExportIdentifier::Chart(id) => store.get_chart(id)?,
		};
		let name = sanitise_filename(bb.desc.as_str());
		let output = output_path(self.output, &name, self.bbzip)?;

		match &self.identifier {
			ExportIdentifier::Bundle(id) => store.export_bundle(*id, &output, self.bbzip)?,
			ExportIdentifier::Chart(id) => store.export_chart(id, &output, self.bbzip)?,
		}

		println!("{} wrote {}", style::check_mark(), output.display());
		Ok(())
	}
}

fn output_path(output: Option<PathBuf>, name: &str, bbzip: bool) -> anyhow::Result<PathBuf> {
	let parent = match output {
		Some(output) => output,
		None => directories::UserDirs::new()
			.and_then(|dirs| dirs.document_dir().map(Path::to_owned))
			.context("could not determine the user's Documents directory")?
			.join("Charts"),
	};

	let mut output = parent.join(name);
	if bbzip {
		output.set_extension("bbzip");
	}

	Ok(output)
}

fn sanitise_filename(name: &str) -> String {
	let mut name: String = name
		.chars()
		.map(|character| {
			if character.is_control()
				|| matches!(
					character,
					'/' | '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*'
				) {
				'-'
			} else {
				character
			}
		})
		.collect();
	name = name.trim_end_matches([' ', '.']).to_owned();
	if name.is_empty() || matches!(name.as_str(), "." | "..") {
		"chart".to_owned()
	} else {
		name
	}
}
