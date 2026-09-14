use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChartExtract {
	pub song: SongExtract,
	pub chart: ChartMetaExtract,
	pub assets: AssetPathExtract,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SongExtract {
	pub title: Option<String>,
	pub subtitle: Option<String>,
	pub artist: Option<String>,
	pub genre: Option<String>,
	pub preview: Option<u64>,
	pub title_translit: Option<String>,
	pub artist_translit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ChartMetaExtract {
	pub credit: Option<String>,
	pub charter_note: Option<String>,
	pub difficulty_label: Option<String>,
	pub level: Option<String>,
	pub chart_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssetPathExtract {
	pub music: Option<String>,
	pub banner: Option<String>,
	pub background: Option<String>,
	pub jacket: Option<String>,
	pub signature: Option<String>,
}
