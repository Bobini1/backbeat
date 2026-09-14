pub mod collection;
pub mod config;
pub mod convert_and_import;
pub mod data_server;
pub mod download;
pub mod export;
pub mod guts;
pub mod import;
pub mod info;
pub mod init;
pub mod shell_completions;
pub mod store;

use clap::{CommandFactory, Subcommand};

use self::collection::CollectionCommand;
use self::config::ConfigCommand;
use self::convert_and_import::ConvertAndImportCommand;
use self::data_server::StartDataServerCommand;
use self::download::DownloadCommand;
use self::export::ExportCommand;
use self::guts::GutsCommand;
use self::import::ImportCommand;
use self::info::InfoCommand;
use self::init::InitCommand;
use self::shell_completions::ShellCompletionsCommand;
use self::store::StoreCommand;
use crate::Cli;

#[derive(Debug, Subcommand)]
pub enum Command {
	/// Ensure your global Backbeat store is set up.
	///
	/// It's optional, really. Every command will auto-init on first run. There are "open door" buttons on tube carriages
	/// even though the doors open themselves always, and the open door buttons aren't wired into anything.
	Init(InitCommand),

	/// Show config, data paths, and store statistics.
	Info(InfoCommand),

	/// Add an existing Backbeat bundle to the store.
	Import(ImportCommand),

	/// Convert a source chart and import it and its assets into the store.
	ConvertAndImport(ConvertAndImportCommand),

	/// Export an installed chart and its assets.
	Export(ExportCommand),

	/// Download content from your configured data servers.
	#[command(subcommand)]
	Download(DownloadCommand),

	/// Read or update `backbeat.toml` configuration values.
	#[command(subcommand)]
	Config(ConfigCommand),

	/// Manage installed collections (packs, tables, courses).
	#[command(subcommand)]
	Collection(CollectionCommand),

	/// Manage your backbeat store.
	#[command(subcommand)]
	Store(StoreCommand),

	/// Low-level utilities primarily for debugging backbeat internals.
	#[command(subcommand)]
	Guts(GutsCommand),

	/// Start a Backbeat Server, that serves the contents of your store.
	///
	/// You can use this as a production-ready server, if you put it behind a reverse proxy. Seriously!
	StartDataServer(StartDataServerCommand),

	/// Generate shell completions and print them to stdout.
	///
	/// Pipe the output to the appropriate file for your shell:
	///
	///   bkb shell-completions zsh  > ~/.zfunc/_bkb
	///   bkb shell-completions bash > ~/.local/share/bash-completion/completions/bkb
	///   bkb shell-completions fish > ~/.config/fish/completions/bkb.fish
	ShellCompletions(ShellCompletionsCommand),
}

impl Command {
	pub async fn run(self) -> anyhow::Result<()> {
		match self {
			Self::Init(cmd) => cmd.run(),
			Self::Info(cmd) => cmd.run(),
			Self::Import(cmd) => cmd.run(),
			Self::ConvertAndImport(cmd) => cmd.run().await,
			Self::Export(cmd) => cmd.run(),
			Self::Download(cmd) => cmd.run().await,
			Self::Config(cmd) => cmd.run(),
			Self::Collection(cmd) => cmd.run().await,
			Self::Store(cmd) => cmd.run().await,
			Self::Guts(cmd) => cmd.run().await,
			Self::StartDataServer(cmd) => cmd.run().await,
			Self::ShellCompletions(cmd) => {
				cmd.run(&mut Cli::command());
				Ok(())
			}
		}
	}
}
