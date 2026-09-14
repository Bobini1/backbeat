#![cfg_attr(test, allow(unreachable_pub, clippy::all, clippy::restriction))]
#![cfg_attr(test, allow(clippy::pedantic, clippy::nursery, clippy::cargo))]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

pub(crate) use fs_err as fs;

mod asset_store;
mod data;
mod util;

#[cfg(test)]
mod test_util;

#[cfg(any(test, feature = "test-util"))]
pub mod test_support;

pub mod assets;
pub mod bundle;
pub mod collections;
pub mod download_manager;
pub mod maintenance;
pub mod remote;
pub mod store;

pub use self::assets::AssetData;
pub use self::collections::CollectionClientError;
pub use self::data::DataId;
pub use self::store::{Backbeat, StoreError};

pub use self::store::Result;
