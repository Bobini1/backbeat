use backbeat_sdk::Backbeat;
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;

pub(crate) async fn open_readonly_database(store: &Backbeat) -> Result<SqlitePool, sqlx::Error> {
	SqlitePoolOptions::new()
		.max_connections(1)
		.connect(&store.sqlite_connection_url())
		.await
}
