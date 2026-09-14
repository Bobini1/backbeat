#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![doc = include_str!("../README.md")]
mod client;
mod paths;

pub use self::client::{
	BackbeatServerClient, RemoteAssetReturn, RemoteError, RemotePrecombinedAssetsReturn,
};
pub use self::paths::ServerPaths;
