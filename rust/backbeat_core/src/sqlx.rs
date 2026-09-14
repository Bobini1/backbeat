use crate::asset_id::AssetId;
use crate::bb::{BundleId, CombinedAssetsId};
use crate::sha256::Sha256;
use crate::{AssetPath, ChartDesc, ChartFilename, ChartId, ValidGamemodeIdentifier};
use sqlx::decode::Decode;
use sqlx::encode::{Encode, IsNull};
use sqlx::error::BoxDynError;
use sqlx::sqlite::{SqliteTypeInfo, SqliteValueRef};
use sqlx::{Database, Sqlite, Type};

macro_rules! impl_sqlx_id {
	($ty:ty) => {
		impl Type<Sqlite> for $ty {
			fn type_info() -> SqliteTypeInfo {
				<&str as Type<Sqlite>>::type_info()
			}

			fn compatible(ty: &SqliteTypeInfo) -> bool {
				<&str as Type<Sqlite>>::compatible(ty)
			}
		}

		impl<'q> Encode<'q, Sqlite> for $ty {
			fn encode_by_ref(
				&self,
				buf: &mut <Sqlite as Database>::ArgumentBuffer,
			) -> Result<IsNull, BoxDynError> {
				let text = self.to_string();
				<String as Encode<'q, Sqlite>>::encode_by_ref(&text, buf)
			}
		}

		impl<'r> Decode<'r, Sqlite> for $ty {
			fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
				let value: String = Decode::<Sqlite>::decode(value)?;
				value.parse::<$ty>().map_err(|err| err.into())
			}
		}
	};
}

impl_sqlx_id!(CombinedAssetsId);
impl_sqlx_id!(BundleId);
impl_sqlx_id!(ChartId);
impl_sqlx_id!(ValidGamemodeIdentifier);

fn decode_sha256(value: SqliteValueRef<'_>) -> Result<Sha256, BoxDynError> {
	let value: String = Decode::<Sqlite>::decode(value)?;
	value.parse::<Sha256>().map_err(Into::into)
}

macro_rules! impl_sqlx_sha256_text {
	($ty:ty, $sha256:expr, $from_sha256:expr) => {
		impl Type<Sqlite> for $ty {
			fn type_info() -> SqliteTypeInfo {
				<&str as Type<Sqlite>>::type_info()
			}

			fn compatible(ty: &SqliteTypeInfo) -> bool {
				<&str as Type<Sqlite>>::compatible(ty)
			}
		}

		impl<'q> Encode<'q, Sqlite> for $ty {
			fn encode_by_ref(
				&self,
				buf: &mut <Sqlite as Database>::ArgumentBuffer,
			) -> Result<IsNull, BoxDynError> {
				let text = $sha256(self).to_string();
				<String as Encode<'q, Sqlite>>::encode_by_ref(&text, buf)
			}
		}

		impl<'r> Decode<'r, Sqlite> for $ty {
			fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
				Ok($from_sha256(decode_sha256(value)?))
			}
		}
	};
}

impl_sqlx_sha256_text!(Sha256, |sha256: &Sha256| *sha256, |sha256| sha256);
impl_sqlx_sha256_text!(AssetId, |asset_id: &AssetId| asset_id.0, AssetId);

macro_rules! impl_sqlx_str {
	($ty:ty) => {
		impl Type<Sqlite> for $ty {
			fn type_info() -> SqliteTypeInfo {
				<&str as Type<Sqlite>>::type_info()
			}

			fn compatible(ty: &SqliteTypeInfo) -> bool {
				<&str as Type<Sqlite>>::compatible(ty)
			}
		}

		impl<'q> Encode<'q, Sqlite> for $ty {
			fn encode_by_ref(
				&self,
				buf: &mut <Sqlite as Database>::ArgumentBuffer,
			) -> Result<IsNull, BoxDynError> {
				let text = self.as_str().to_owned();
				<String as Encode<'q, Sqlite>>::encode_by_ref(&text, buf)
			}
		}

		impl<'r> Decode<'r, Sqlite> for $ty {
			fn decode(value: SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
				let value: String = Decode::<Sqlite>::decode(value)?;
				value.parse::<$ty>().map_err(|err| err.into())
			}
		}
	};
}

impl_sqlx_str!(ChartFilename);
impl_sqlx_str!(AssetPath);
impl_sqlx_str!(ChartDesc);
