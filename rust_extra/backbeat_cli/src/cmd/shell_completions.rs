use clap::Args;
use clap_complete::Shell;

/// Generate shell completions for `bkb` and print them to stdout.
///
/// Pipe the output to the appropriate file for your shell:
///
///   bkb shell-completions zsh  > ~/.zfunc/_bkb
///   bkb shell-completions bash > ~/.local/share/bash-completion/completions/bkb
///   bkb shell-completions fish > ~/.config/fish/completions/bkb.fish
#[derive(Debug, Args)]
pub struct ShellCompletionsCommand {
	/// Shell to generate completions for.
	pub shell: Shell,
}

impl ShellCompletionsCommand {
	pub fn run(self, cmd: &mut clap::Command) {
		clap_complete::generate(self.shell, cmd, "bkb", &mut std::io::stdout());
	}
}
