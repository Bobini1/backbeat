use std::cell::Cell;
use std::future::Future;
use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use serde::{Serialize, de::DeserializeOwned};

use crate::{Result, store::StoreError};

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

thread_local! {
	static STORE_THREAD: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn runtime() -> &'static tokio::runtime::Runtime {
	RUNTIME.get_or_init(|| {
		tokio::runtime::Builder::new_multi_thread()
			.worker_threads(1)
			.thread_name("backbeat-store")
			.on_thread_start(|| STORE_THREAD.set(true))
			.enable_all()
			.build()
			.expect("failed to build tokio runtime")
	})
}

#[allow(non_snake_case)]
#[track_caller]
pub(crate) fn BLOCK<F: Future>(fut: F) -> F::Output {
	let block = || {
		let _guard = runtime().enter();
		futures_lite::future::block_on(fut)
	};
	// Only our own workers need a replacement while blocked; callers may use any executor.
	if STORE_THREAD.get() {
		tokio::task::block_in_place(block)
	} else {
		block()
	}
}

pub(crate) fn encode_json<T: Serialize>(value: &T) -> Result<String> {
	Ok(serde_json::to_string(value)?)
}

pub(crate) fn decode_json<T: DeserializeOwned>(value: &str) -> Result<T> {
	serde_json::from_str(value)
		.map_err(|err| StoreError::Corrupt(format!("invalid JSON in DB: {err}")))
}

pub(crate) fn parse_iso8601(s: &str) -> Result<DateTime<Utc>> {
	DateTime::parse_from_rfc3339(s)
		.map(|dt| dt.with_timezone(&Utc))
		.map_err(|err| StoreError::Corrupt(format!("invalid timestamp: {err}")))
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use super::BLOCK;
	use crate::Backbeat;

	fn exercise_store(store: &Backbeat) {
		for _ in 0..64 {
			assert_eq!(store.should_refresh(0).unwrap(), (false, 0));
		}
		BLOCK(async {
			tokio::spawn(async {
				tokio::time::sleep(Duration::from_millis(1)).await;
			})
			.await
			.unwrap();
		});
	}

	#[test]
	fn store_from_plain_threads() {
		let (_temp, store) = crate::test_util::new_test_store("blocking_plain");
		exercise_store(&store);
		std::thread::scope(|scope| {
			for _ in 0..4 {
				scope.spawn(|| exercise_store(&store));
			}
		});
	}

	#[tokio::test]
	async fn store_from_current_thread_runtime() {
		let (_temp, store) = crate::test_util::new_test_store("blocking_current_thread");
		exercise_store(&store);
		tokio::spawn(async move { exercise_store(&store) })
			.await
			.unwrap();
	}

	#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
	async fn store_from_multi_thread_runtime() {
		let (_temp, store) = crate::test_util::new_test_store("blocking_multi_thread");
		exercise_store(&store);
		tokio::spawn(async move { exercise_store(&store) })
			.await
			.unwrap();
	}

	#[tokio::test]
	async fn blocking_query_after_async_connection_return() {
		let (_temp, mut store) = crate::test_util::new_test_store("blocking_async_return");
		store.pool = BLOCK(
			sqlx::sqlite::SqlitePoolOptions::new()
				.max_connections(1)
				.acquire_timeout(Duration::from_secs(2))
				.connect_with((*store.pool.connect_options()).clone()),
		)
		.unwrap();
		store
			.run(|store| async move {
				let connection = store.pool.acquire().await.unwrap();
				drop(connection);
			})
			.await;
		exercise_store(&store);
	}

	#[test]
	fn store_from_another_executor_and_its_own_worker() {
		futures::executor::block_on(async {
			let (_temp, store) = crate::test_util::new_test_store("blocking_other_executor");
			exercise_store(&store);
			store
				.run(|store| async move { exercise_store(&store) })
				.await;
			BLOCK(async { exercise_store(&store) });
		});
	}

	#[test]
	fn store_outlives_the_callers_runtime() {
		let caller = tokio::runtime::Builder::new_current_thread()
			.build()
			.unwrap();
		let (_temp, store) = caller
			.block_on(async { crate::test_util::new_test_store("blocking_runtime_lifetime") });
		drop(caller);
		exercise_store(&store);
	}

	#[tokio::test]
	async fn cancelling_an_operation_rolls_back_its_transaction() {
		struct Dropped(std::sync::mpsc::Sender<()>);
		impl Drop for Dropped {
			fn drop(&mut self) {
				let _ = self.0.send(());
			}
		}

		let (_temp, store) = crate::test_util::new_test_store("blocking_cancel");
		let (started, ready) = std::sync::mpsc::channel();
		let (dropped, finished) = std::sync::mpsc::channel();
		let mut operation = Box::pin(store.run(move |store| async move {
			let _dropped = Dropped(dropped);
			let mut tx = store.pool.begin().await.unwrap();
			sqlx::query!("UPDATE refresh SET revision = 123 WHERE id = 1")
				.execute(&mut *tx)
				.await
				.unwrap();
			started.send(()).unwrap();
			std::future::pending::<()>().await;
			tx.commit().await.unwrap();
		}));
		assert!(futures::poll!(operation.as_mut()).is_pending());
		ready.recv_timeout(Duration::from_secs(5)).unwrap();
		drop(operation);
		finished.recv_timeout(Duration::from_secs(5)).unwrap();
		exercise_store(&store);
	}

	#[test]
	fn asset_committer_does_not_keep_the_store_alive() {
		let (_temp, store) = crate::test_util::new_test_store("blocking_store_lifetime");
		let config = std::sync::Arc::downgrade(&store.config);
		let data = b"asset".to_vec();
		let asset_id = backbeat_core::AssetId(backbeat_core::Sha256::checksum_bytes(&data));
		BLOCK(
			store
				.download_mgr
				.commit_asset(&store, asset_id, data.len() as i64, Some(data)),
		)
		.unwrap();
		drop(store);
		assert!(config.upgrade().is_none());
	}
}
