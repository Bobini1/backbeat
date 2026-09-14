use backbeat_core::AssetPath;
use clap::Subcommand;

mod deps;
mod fracture;
mod inspect;
mod media_info;
mod meld;
mod missingdeps;
mod output;
mod pack;
mod sm_enrich;

use self::deps::DepsCommand;
use self::fracture::FractureCommand;
use self::inspect::InspectCommand;
use self::media_info::MediaInfoCommand;
use self::meld::MeldCommand;
use self::missingdeps::MissingDepsCommand;
use self::pack::PackCommand;
use self::sm_enrich::SmEnrichCommand;

/// The core, "guts" of backbeat. Mostly, the internal commands used as part of the
/// package pipeline.
///
/// These tools form a useful interface for debugging internal backbeat functionality.
///
/// In git terms, these are the "plumbing" commands.
#[derive(Debug, Subcommand)]
pub enum GutsCommand {
	/// Package a source chart file into a .bb file.
	Pack(PackCommand),

	/// Add Backbeat metadata to an unfractured StepMania `.sm` file.
	SmEnrich(SmEnrichCommand),

	/// Split a file that composes multiple playable charts, into one file per playable chart.
	Fracture(FractureCommand),

	/// Combine multiple individual-chart files back into one chart file.
	///
	/// The opposite of `bkb guts meld`.
	Meld(MeldCommand),

	/// List the asset dependencies of a source chart file.
	Deps(DepsCommand),

	/// List referenced assets that do not exist on disk.
	MissingDeps(MissingDepsCommand),

	/// Inspect a source chart or .bb bundle.
	Inspect(InspectCommand),

	/// Extract media metadata using ffprobe.
	MediaInfo(MediaInfoCommand),
}

impl GutsCommand {
	pub async fn run(self) -> anyhow::Result<()> {
		match self {
			Self::Pack(cmd) => cmd.run(),
			Self::SmEnrich(cmd) => cmd.run(),
			Self::Fracture(cmd) => cmd.run(),
			Self::Meld(cmd) => cmd.run(),
			Self::Deps(cmd) => cmd.run(),
			Self::MissingDeps(cmd) => cmd.run(),
			Self::Inspect(cmd) => cmd.run(),
			Self::MediaInfo(cmd) => cmd.run(),
		}
	}
}

pub(crate) fn require_no_missing_dependencies(mut missing: Vec<AssetPath>) -> anyhow::Result<()> {
	if missing.is_empty() {
		return Ok(());
	}

	missing.sort_unstable();
	missing.dedup();
	let missing = missing
		.iter()
		.map(|path| path.to_string())
		.collect::<Vec<_>>()
		.join(", ");

	anyhow::bail!("missing referenced dependencies: {missing}");
}
