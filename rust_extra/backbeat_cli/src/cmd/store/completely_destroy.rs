use std::io::{IsTerminal, Write};

use anyhow::Context;
use clap::Args;

use crate::{store_util, style};

/// Permanently delete this store directory.
#[derive(Debug, Args)]
pub struct CompletelyDestroyCommand {
	/// Skip the interactive confirmation prompt.
	#[arg(long)]
	pub yes: bool,
}

impl CompletelyDestroyCommand {
	pub async fn run(self) -> anyhow::Result<()> {
		let store = store_util::open()?;
		let root = store.store_dir().to_path_buf();
		drop(store);

		println!("This will permanently delete: {}", root.display());

		if !self.yes && !confirm("Destroy the store?")? {
			println!("Aborted.");
			return Ok(());
		}

		std::fs::remove_dir_all(&root)
			.with_context(|| format!("failed to remove {}", root.display()))?;

		println!("{} Store completely destroyed.", style::check_mark());
		Ok(())
	}
}

/// Prompt for a yes/no confirmation on stdout/stdin.
///
/// Returns an error if stdin is not a terminal; use `--yes` to skip the
/// prompt in non-interactive environments.
pub(super) fn confirm(prompt: &str) -> anyhow::Result<bool> {
	if !std::io::stdin().is_terminal() {
		anyhow::bail!("stdin is not a terminal; pass --yes to skip the confirmation prompt");
	}

	print!("\n{prompt} [y/N] ");
	std::io::stdout()
		.flush()
		.context("failed to flush stdout")?;

	let mut input = String::new();
	std::io::stdin()
		.read_line(&mut input)
		.context("failed to read confirmation")?;

	Ok(input.trim().eq_ignore_ascii_case("y"))
}
