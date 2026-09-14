use std::path::Component;

use backbeat_core::Assets;

pub(crate) fn depth(assets: &Assets) -> usize {
	assets
		.keys()
		.map(|path| {
			let mut relative_depth = 0isize;
			let mut max_upward_depth = 0usize;
			for component in path.as_path().components() {
				match component {
					Component::CurDir => {}
					Component::ParentDir => {
						relative_depth -= 1;
						max_upward_depth = max_upward_depth.max((-relative_depth).max(0) as usize);
					}
					Component::Normal(_) => relative_depth += 1,
					Component::RootDir | Component::Prefix(_) => {}
				}
			}
			max_upward_depth
		})
		.max()
		.unwrap_or(0)
}

pub(crate) fn path(depth: usize) -> String {
	"_x/".repeat(depth)
}

#[cfg(test)]
mod tests {
	use std::collections::HashMap;

	use backbeat_core::{AssetId, AssetPath, Sha256};

	use super::*;

	#[test]
	fn depth_covers_the_deepest_upward_traversal() {
		let asset = AssetId::from(Sha256::checksum_bytes(b"asset"));
		assert_eq!(
			depth(&HashMap::from([(
				AssetPath::from_path("local.wav").unwrap(),
				asset
			)])),
			0
		);
		assert_eq!(
			depth(&HashMap::from([(
				AssetPath::from_path("../shared.wav").unwrap(),
				asset
			)])),
			1
		);
		assert_eq!(
			depth(&HashMap::from([(
				AssetPath::from_path("../../../../shared.wav").unwrap(),
				asset
			)])),
			4
		);
	}
}
