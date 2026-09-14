use vergen_git2::{Emitter, Git2Builder};

fn main() {
	let git2 = Git2Builder::default()
		.sha(true)
		.build()
		.expect("vergen git2");
	Emitter::default()
		.add_instructions(&git2)
		.expect("vergen emitter")
		.emit()
		.expect("vergen emit");
	tauri_build::build();
}
