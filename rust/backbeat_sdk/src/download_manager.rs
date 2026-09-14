//! Download manager for remote fetches: de-duplication, parallel scheduling,
//! progress, and cancellation.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use backbeat_server_client::{RemoteAssetReturn, RemotePrecombinedAssetsReturn};
use parking_lot::Mutex;
use serde::Serialize;

use backbeat_core::BundleId;
use backbeat_core::{AssetId, Sha256, Sha256Digest};
use backbeat_core::{Assets, ChartId};
use dashmap::DashMap;
use futures::future::BoxFuture;
use futures::stream::{self, StreamExt as _};
use sqlx::types::Uuid;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::sync::{Notify, OnceCell, OwnedSemaphorePermit, Semaphore, mpsc, oneshot};
use tokio_util::{io::SyncIoBridge, sync::CancellationToken};

use crate::{Backbeat, Result, StoreError};

use crate::DataId;
use backbeat_server_client::RemoteError;

type DownloadResult<T> = std::result::Result<T, Arc<StoreError>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum DownloadState {
	Queued = 0,
	Downloading = 1,
	Verifying = 2,
	Committing = 3,
	Done = 4,
	Failed = 5,
	Cancelled = 6,
}

#[allow(clippy::fallible_impl_from)]
impl From<u8> for DownloadState {
	fn from(v: u8) -> Self {
		match v {
			0 => Self::Queued,
			1 => Self::Downloading,
			2 => Self::Verifying,
			3 => Self::Committing,
			4 => Self::Done,
			5 => Self::Failed,
			6 => Self::Cancelled,
			other => panic!("invalid download state {other}"),
		}
	}
}

impl DownloadState {
	/// Whether this state is terminal (`Done`, `Failed`, or `Cancelled`).
	fn is_terminal(self) -> bool {
		matches!(self, Self::Done | Self::Failed | Self::Cancelled)
	}
}

/// Soft cap on lingering failed download slots. Mass installs can produce
/// thousands of failures; keeping all of them would blow up memory and IPC.
const MAX_FAILED_SLOTS: usize = 250;
const DOWNLOAD_LIST_MAX_LIMIT: u32 = 200;
const ASSET_COMMIT_BATCH_SIZE: usize = 128;
const BUFFER_PERMIT_BYTES: u64 = 64 * 1024;
const PRECOMBINED_ASSET_THRESHOLD: usize = 30;
const PRECOMBINED_ASSET_PIPE_BYTES: usize = 1024 * 1024;

/// Aggregate counts for the download manager, without shipping every row.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadOverview {
	pub total: u64,
	pub queued: u64,
	pub running: u64,
	pub failed: u64,
	pub done: u64,
	pub cancelled: u64,
	/// First failure message found while scanning, if any.
	pub first_error: Option<String>,
}

/// One page of download rows plus the same aggregate counts as [`DownloadOverview`].
#[derive(Debug, Clone, Serialize)]
pub struct DownloadListResult {
	pub total: u64,
	pub queued: u64,
	pub running: u64,
	pub failed: u64,
	pub done: u64,
	pub cancelled: u64,
	pub first_error: Option<String>,
	pub downloads: Vec<DownloadSnapshot>,
	pub has_more: bool,
}

/// A point-in-time snapshot of a download's progress.
#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
	/// Bytes downloaded for an individual asset.
	pub bytes: u64,
	/// When this download will be done, if known. This is the Content-Length
	/// of an asset download.
	pub total: Option<u64>,
	/// Child assets that have finished downloading for a chart or bundle.
	pub items_done: u64,
	/// Total child assets to download for a chart or bundle.
	pub items_total: Option<u64>,

	pub state: DownloadState,
}

/// A point-in-time snapshot of one in-flight or just-finished download,
/// suitable for exposing to observers (e.g. the GUI).
#[derive(Debug, Clone, Serialize)]
pub struct DownloadSnapshot {
	/// What is being downloaded.
	pub key: DataId,
	/// Current progress and state.
	pub progress: DownloadProgress,
	/// Failure reason, if failed.
	pub error: Option<String>,
}

// ── Progress cell (lock-free, shared between future and observers) ────────────

#[derive(Default)]
struct ProgressCell {
	bytes: AtomicU64,
	total: OnceLock<Option<u64>>,
	items_done: AtomicU64,
	items_total: OnceLock<u64>,
	state: AtomicU8,
}

impl ProgressCell {
	fn new() -> Self {
		let cell = Self::default();
		cell.state
			.store(DownloadState::Queued as u8, Ordering::Relaxed);
		cell
	}

	fn snapshot(&self) -> DownloadProgress {
		DownloadProgress {
			bytes: self.bytes.load(Ordering::Relaxed),
			total: *self.total.get().unwrap_or(&None),
			items_done: self.items_done.load(Ordering::Relaxed),
			items_total: self.items_total.get().copied(),
			state: self.state.load(Ordering::Relaxed).into(),
		}
	}

	fn set_state(&self, state: DownloadState) {
		self.state.store(state as u8, Ordering::Relaxed);
	}

	fn set_total(&self, total: Option<u64>) {
		let _ = self.total.set(total);
	}

	fn set_items_total(&self, total: u64) {
		let _ = self.items_total.set(total);
	}

	fn add_bytes(&self, n: u64) {
		self.bytes.fetch_add(n, Ordering::Relaxed);
	}

	fn add_item_done(&self) {
		self.items_done.fetch_add(1, Ordering::Relaxed);
	}
}

// ── Completion (shared result cell + wakeup) ──────────────────────────────────

/// Shared completion state for one in-flight key. The active download writes
/// its result and notifies waiters; waiters read it once woken. The error is
/// `Arc`'d because `StoreError` is not `Clone`.
struct Completion {
	result: Mutex<Option<DownloadResult<()>>>,
	done: Notify,
}

impl Completion {
	fn new() -> Self {
		Self {
			result: Mutex::new(None),
			done: Notify::new(),
		}
	}

	/// Mark the download complete and wake all waiters.
	fn complete(&self, result: DownloadResult<()>) {
		*self.result.lock() = Some(result);
		self.done.notify_waiters();
	}

	/// Read the stored result. Panics if called before the download completes.
	fn result(&self) -> DownloadResult<()> {
		let guard = self.result.lock();
		let stored = guard.as_ref().expect("completion read before done").clone();
		drop(guard);
		stored
	}

	/// Human-readable failure message, if the download completed with an error.
	fn error_message(&self) -> Option<String> {
		let guard = self.result.lock();
		match guard.as_ref()? {
			Ok(()) => None,
			Err(err) => Some(err.to_string()),
		}
	}
}

/// Wait for `completion` to be marked done.
async fn await_completion(completion: &Completion) {
	let notified = completion.done.notified();
	let already_done = completion.result.lock().is_some();
	if !already_done {
		notified.await;
	}
}

struct Slot {
	progress: Arc<ProgressCell>,
	cancel: CancellationToken,
	completion: Arc<Completion>,
}

/// De-duplicating, parallel, cancellable download coordinator.
///
/// Stored on [`Backbeat`] behind an `Arc` so all clones of the store share
/// one registry. Cheap to clone.
#[derive(Clone)]
pub(crate) struct DownloadManager {
	inner: Arc<ManagerInner>,
}

struct ManagerInner {
	inflight: DashMap<DataId, Slot>,
	requests: Arc<RequestScheduler>,
	request_limit: usize,
	buffer_bytes: Arc<Semaphore>,
	asset_commits: OnceCell<mpsc::Sender<AssetCommit>>,
}

#[derive(Clone, Copy)]
enum RequestClass {
	Asset,
	Manifest,
}

struct RequestScheduler {
	state: Mutex<RequestSchedulerState>,
	notify: Notify,
}

struct RequestSchedulerState {
	available: usize,
	asset_waiters: usize,
	manifest_waiters: usize,
}

struct RequestWaiter {
	scheduler: Arc<RequestScheduler>,
	class: RequestClass,
	armed: bool,
}

struct RequestPermit {
	scheduler: Arc<RequestScheduler>,
}

impl RequestScheduler {
	fn new(limit: usize) -> Self {
		Self {
			state: Mutex::new(RequestSchedulerState {
				available: limit,
				asset_waiters: 0,
				manifest_waiters: 0,
			}),
			notify: Notify::new(),
		}
	}

	async fn acquire_asset(self: &Arc<Self>) -> RequestPermit {
		self.acquire(RequestClass::Asset).await
	}

	async fn acquire_manifest(self: &Arc<Self>) -> RequestPermit {
		self.acquire(RequestClass::Manifest).await
	}

	async fn acquire(self: &Arc<Self>, class: RequestClass) -> RequestPermit {
		let mut waiter = RequestWaiter::new(Arc::clone(self), class);
		loop {
			let notified = self.notify.notified();
			let acquired = {
				let mut state = self.state.lock();
				if Self::can_acquire(&state, class) {
					state.available -= 1;
					*Self::waiters_mut(&mut state, class) -= 1;
					true
				} else {
					false
				}
			};
			if acquired {
				waiter.disarm();
				return RequestPermit {
					scheduler: Arc::clone(self),
				};
			}
			notified.await;
		}
	}

	fn can_acquire(state: &RequestSchedulerState, class: RequestClass) -> bool {
		state.available > 0 && (matches!(class, RequestClass::Asset) || state.asset_waiters == 0)
	}

	fn waiters_mut(state: &mut RequestSchedulerState, class: RequestClass) -> &mut usize {
		match class {
			RequestClass::Asset => &mut state.asset_waiters,
			RequestClass::Manifest => &mut state.manifest_waiters,
		}
	}

	#[cfg(test)]
	fn available_slots(&self) -> usize {
		self.state.lock().available
	}
}

impl RequestWaiter {
	fn new(scheduler: Arc<RequestScheduler>, class: RequestClass) -> Self {
		{
			let mut state = scheduler.state.lock();
			*RequestScheduler::waiters_mut(&mut state, class) += 1;
		}
		Self {
			scheduler,
			class,
			armed: true,
		}
	}

	fn disarm(&mut self) {
		self.armed = false;
	}
}

impl Drop for RequestWaiter {
	fn drop(&mut self) {
		if !self.armed {
			return;
		}
		{
			let mut state = self.scheduler.state.lock();
			*RequestScheduler::waiters_mut(&mut state, self.class) -= 1;
		}
		self.scheduler.notify.notify_waiters();
	}
}

impl Drop for RequestPermit {
	fn drop(&mut self) {
		{
			let mut state = self.scheduler.state.lock();
			state.available += 1;
		}
		self.scheduler.notify.notify_waiters();
	}
}

impl DownloadManager {
	/// Manage downloads. What do you think.
	pub(crate) fn new(request_concurrency: usize, buffer_budget_bytes: u64) -> Self {
		let request_limit = request_concurrency.max(1);
		let buffer_permits = buffer_budget_bytes
			.div_ceil(BUFFER_PERMIT_BYTES)
			.try_into()
			.unwrap_or(Semaphore::MAX_PERMITS)
			.min(Semaphore::MAX_PERMITS);
		Self {
			inner: Arc::new(ManagerInner {
				inflight: DashMap::new(),
				requests: Arc::new(RequestScheduler::new(request_limit)),
				request_limit,
				buffer_bytes: Arc::new(Semaphore::new(buffer_permits)),
				asset_commits: OnceCell::new(),
			}),
		}
	}

	pub(crate) async fn acquire_buffer_budget(&self, bytes: usize) -> Result<OwnedSemaphorePermit> {
		let permits = (bytes as u64)
			.div_ceil(BUFFER_PERMIT_BYTES)
			.try_into()
			.map_err(|_| StoreError::Corrupt("download buffer request exceeds limit".into()))?;

		Arc::clone(&self.inner.buffer_bytes)
			.acquire_many_owned(permits)
			.await
			.map_err(|_| StoreError::Corrupt("download buffer semaphore closed".into()))
	}

	// batch up asset writes because of concurrency issues with
	// sqlite.
	//
	// fuck you - zk
	pub(crate) async fn commit_asset(
		&self,
		store: &Backbeat,
		asset_id: AssetId,
		size: i64,
		inline_data: Option<Vec<u8>>,
	) -> DownloadResult<()> {
		let capacity = self.inner.request_limit * 2;
		let sender = self
			.inner
			.asset_commits
			.get_or_init(|| async {
				let (sender, receiver) = mpsc::channel(capacity);
				crate::util::runtime().spawn(run_asset_committer(store.pool.clone(), receiver));
				sender
			})
			.await;
		let (done, completed) = oneshot::channel();

		sender
			.send(AssetCommit {
				asset_id,
				size,
				inline_data,
				done,
			})
			.await
			.map_err(|_| {
				StoreError::shared(StoreError::Corrupt("asset commit worker stopped".into()))
			})?;

		match completed.await {
			Ok(Ok(())) => Ok(()),
			Ok(Err(err)) => Err(err),
			Err(_) => Err(StoreError::Corrupt("asset commit worker stopped".into()).into()),
		}
	}

	/// Snapshot the progress of an in-flight download, if any.
	pub(crate) fn progress(&self, key: &DataId) -> Option<DownloadProgress> {
		self.inner
			.inflight
			.get(key)
			.map(|slot| slot.progress.snapshot())
	}

	/// Snapshot every download currently registered in the manager, including
	/// finished entries that have not yet been pruned.
	pub(crate) fn all_progress(&self) -> Vec<DownloadSnapshot> {
		self.inner
			.inflight
			.iter()
			.map(|entry| DownloadSnapshot {
				key: entry.key().clone(),
				progress: entry.progress.snapshot(),
				error: entry.completion.error_message(),
			})
			.collect()
	}

	/// Aggregate counts (and first failure message) without returning rows.
	pub(crate) fn overview(&self) -> DownloadOverview {
		overview_from_rows(&self.all_progress())
	}

	pub(crate) fn list_progress(&self, offset: u64, limit: u32) -> DownloadListResult {
		let limit = limit.clamp(1, DOWNLOAD_LIST_MAX_LIMIT) as usize;
		list_rows(self.all_progress(), offset, limit)
	}

	/// Paginated chart and bundle download list, excluding child asset transfers.
	pub(crate) fn list_collection_progress(&self, offset: u64, limit: u32) -> DownloadListResult {
		let limit = limit.clamp(1, DOWNLOAD_LIST_MAX_LIMIT) as usize;
		let rows = self
			.all_progress()
			.into_iter()
			.filter(|snapshot| !matches!(snapshot.key, DataId::Asset(_)))
			.collect();
		list_rows(rows, offset, limit)
	}

	/// Drop successful/cancelled asset slots so mass installs don't retain
	/// millions of finished rows. Chart/bundle parents and failures stay.
	fn evict_if_ephemeral(&self, key: &DataId, state: DownloadState) {
		let drop = matches!(key, DataId::Asset(_))
			&& matches!(state, DownloadState::Done | DownloadState::Cancelled);
		if drop {
			self.inner.inflight.remove(key);
		}
	}

	/// Bound how many failed slots linger after mass-download storms.
	fn trim_failed_slots(&self) {
		let failed: Vec<DataId> = self
			.inner
			.inflight
			.iter()
			.filter(|entry| entry.progress.snapshot().state == DownloadState::Failed)
			.map(|entry| entry.key().clone())
			.collect();
		if failed.len() <= MAX_FAILED_SLOTS {
			return;
		}
		let excess = failed.len() - MAX_FAILED_SLOTS;
		for key in failed.into_iter().take(excess) {
			self.inner.inflight.remove(&key);
		}
	}

	pub(crate) fn cancel_download(&self, key: impl Into<DataId>) -> bool {
		let key = key.into();

		if let Some(slot) = self.inner.inflight.get(&key) {
			if slot.progress.snapshot().state.is_terminal() {
				return false;
			}
			slot.cancel.cancel();
			true
		} else {
			false
		}
	}

	pub(crate) async fn install_asset(
		&self,
		store: &Backbeat,
		asset_id: AssetId,
	) -> DownloadResult<()> {
		self.run_shared(store, asset_id, move |store, cancel| {
			Box::pin(download_asset(store, asset_id, cancel))
		})
		.await
	}

	pub(crate) fn queue_asset(&self, store: &Backbeat, asset_id: AssetId) {
		let key = DataId::Asset(asset_id);
		let store = store.clone();
		let make_future = move |store, cancel| -> BoxFuture<'static, DownloadResult<()>> {
			Box::pin(download_asset(store, asset_id, cancel))
		};

		match self.inner.inflight.entry(key) {
			dashmap::mapref::entry::Entry::Occupied(occupied) => {
				if occupied.get().progress.snapshot().state.is_terminal() {
					replace_and_spawn(occupied, store, make_future);
				}
			}
			dashmap::mapref::entry::Entry::Vacant(vacant) => {
				install_and_spawn(vacant, store, make_future);
			}
		}
	}

	pub(crate) async fn install_chart(
		&self,
		store: &Backbeat,
		chart_id: &ChartId,
	) -> DownloadResult<()> {
		let chart_id = chart_id.clone();
		self.run_shared(store, chart_id.clone(), move |store, cancel| {
			Box::pin(download_chart(store, chart_id, cancel))
		})
		.await
	}

	pub(crate) async fn install_bundle(
		&self,
		store: &Backbeat,
		bundle_id: BundleId,
	) -> DownloadResult<()> {
		self.run_shared(store, bundle_id, move |store, cancel| {
			Box::pin(download_bundle(store, bundle_id, cancel))
		})
		.await
	}

	async fn run_shared<F>(
		&self,
		store: &Backbeat,
		key: impl Into<DataId>,
		make_future: F,
	) -> DownloadResult<()>
	where
		F: FnOnce(Backbeat, CancellationToken) -> BoxFuture<'static, DownloadResult<()>>
			+ Send
			+ 'static,
	{
		let key = key.into();

		let store_clone = store.clone();
		let entry = self.inner.inflight.entry(key.clone());

		let completion = match entry {
			dashmap::mapref::entry::Entry::Occupied(occupied) => {
				let slot_state = occupied.get().progress.snapshot().state;
				if slot_state.is_terminal() {
					// Finished slot lingering for observability: replace it and
					// start a fresh download, then await that.
					replace_and_spawn(occupied, store_clone, make_future)
				} else {
					// Wait queue: join the active download's completion. We
					// release the DashMap guard before awaiting to avoid
					// holding the shard lock across the wait.
					let completion = Arc::clone(&occupied.get().completion);
					drop(occupied);
					await_completion(&completion).await;
					return completion.result();
				}
			}
			dashmap::mapref::entry::Entry::Vacant(vacant) => {
				install_and_spawn(vacant, store_clone, make_future)
			}
		};

		// Primary caller (or re-requester) waits on the freshly installed slot.
		await_completion(&completion).await;
		completion.result()
	}

	/// Remove every finished download from the in-flight map. "Finished" means either failed or completed.
	pub(crate) fn clear_finished(&self) -> usize {
		let to_remove: Vec<DataId> = self
			.inner
			.inflight
			.iter()
			.filter(|entry| entry.progress.snapshot().state.is_terminal())
			.map(|entry| entry.key().clone())
			.collect();

		let n = to_remove.len();
		for key in to_remove {
			self.inner.inflight.remove(&key);
		}

		n
	}

	/// What's the progress for this bit of data?
	fn progress_cell(&self, key: &DataId) -> Option<Arc<ProgressCell>> {
		self.inner
			.inflight
			.get(key)
			.map(|s| Arc::clone(&s.progress))
	}
}

fn list_rows(mut rows: Vec<DownloadSnapshot>, offset: u64, limit: usize) -> DownloadListResult {
	rows.sort_by_cached_key(|row| (list_rank(row.progress.state), row.key.to_string()));
	let overview = overview_from_rows(&rows);
	let offset = usize::try_from(offset).unwrap_or(usize::MAX);
	let has_more = rows.len().saturating_sub(offset) > limit;
	let downloads = rows.into_iter().skip(offset).take(limit).collect();

	DownloadListResult {
		total: overview.total,
		queued: overview.queued,
		running: overview.running,
		failed: overview.failed,
		done: overview.done,
		cancelled: overview.cancelled,
		first_error: overview.first_error,
		downloads,
		has_more,
	}
}

fn overview_from_rows(rows: &[DownloadSnapshot]) -> DownloadOverview {
	let mut overview = DownloadOverview {
		total: 0,
		queued: 0,
		running: 0,
		failed: 0,
		done: 0,
		cancelled: 0,
		first_error: None,
	};
	for snapshot in rows {
		overview.total += 1;
		match snapshot.progress.state {
			DownloadState::Queued => overview.queued += 1,
			DownloadState::Downloading | DownloadState::Verifying | DownloadState::Committing => {
				overview.running += 1;
			}
			DownloadState::Failed => {
				overview.failed += 1;
				if overview.first_error.is_none() {
					overview.first_error.clone_from(&snapshot.error);
				}
			}
			DownloadState::Done => overview.done += 1,
			DownloadState::Cancelled => overview.cancelled += 1,
		}
	}
	overview
}

/// Replace a lingering finished slot with a fresh one and spawn its driver.
/// Returns the new slot's completion, for the caller to await.
fn replace_and_spawn<F>(
	occupied: dashmap::mapref::entry::OccupiedEntry<'_, DataId, Slot>,
	store: Backbeat,
	make_future: F,
) -> Arc<Completion>
where
	F: FnOnce(Backbeat, CancellationToken) -> BoxFuture<'static, DownloadResult<()>>
		+ Send
		+ 'static,
{
	install_and_spawn_into_occupied(occupied, store, make_future)
}

/// Install a fresh slot into a vacant entry and spawn its download driver.
/// Returns the slot's completion, for the caller to await.
fn install_and_spawn<F>(
	vacant: dashmap::mapref::entry::VacantEntry<'_, DataId, Slot>,
	store: Backbeat,
	make_future: F,
) -> Arc<Completion>
where
	F: FnOnce(Backbeat, CancellationToken) -> BoxFuture<'static, DownloadResult<()>>
		+ Send
		+ 'static,
{
	let key = vacant.key().clone();
	let cancel = CancellationToken::new();
	let progress = Arc::new(ProgressCell::new());
	let completion = Arc::new(Completion::new());
	let cancel_for_future = cancel.clone();

	let slot = Slot {
		progress: Arc::clone(&progress),
		cancel,
		completion: Arc::clone(&completion),
	};
	vacant.insert(slot);
	spawn_driver(
		store.download_mgr.clone(),
		key,
		store,
		progress,
		Arc::clone(&completion),
		cancel_for_future,
		make_future,
	);
	completion
}

/// Overwrite an occupied (finished) entry with a fresh slot and spawn its
/// driver. Returns the new slot's completion.
fn install_and_spawn_into_occupied<F>(
	mut occupied: dashmap::mapref::entry::OccupiedEntry<'_, DataId, Slot>,
	store: Backbeat,
	make_future: F,
) -> Arc<Completion>
where
	F: FnOnce(Backbeat, CancellationToken) -> BoxFuture<'static, DownloadResult<()>>
		+ Send
		+ 'static,
{
	let key = occupied.key().clone();
	let cancel = CancellationToken::new();
	let progress = Arc::new(ProgressCell::new());
	let completion = Arc::new(Completion::new());
	let cancel_for_future = cancel.clone();

	let slot = Slot {
		progress: Arc::clone(&progress),
		cancel,
		completion: Arc::clone(&completion),
	};
	occupied.insert(slot);
	spawn_driver(
		store.download_mgr.clone(),
		key,
		store,
		progress,
		Arc::clone(&completion),
		cancel_for_future,
		make_future,
	);
	completion
}

/// Drive a download future to completion on a detached task, writing terminal
/// state and signalling waiters.
///
/// Successful (and cancelled) **asset** slots are evicted immediately after
/// completion — they dominate the map during mass installs and are not useful
/// once finished. Chart/bundle parents and failures linger until
/// [`DownloadManager::prune_finished`] (or the failed-slot cap) clears them.
///
/// The download future runs on its *own* inner [`tokio::spawn`], and this
/// outer task awaits that inner `JoinHandle` rather than the future directly.
/// This matters: if the download future panics, awaiting it directly (with
/// no intervening task boundary) would unwind straight through this task,
/// skipping `completion.complete(...)` below and leaving every current and
/// future waiter on this key hung forever (`await_completion` never wakes,
/// and the slot never reaches a terminal state). Routing through an inner
/// `JoinHandle` converts a panic into an ordinary `Err` we can still report.
fn spawn_driver<F>(
	downloads: DownloadManager,
	key: DataId,
	store: Backbeat,
	progress: Arc<ProgressCell>,
	completion: Arc<Completion>,
	cancel: CancellationToken,
	make_future: F,
) where
	F: FnOnce(Backbeat, CancellationToken) -> BoxFuture<'static, DownloadResult<()>>
		+ Send
		+ 'static,
{
	crate::util::runtime().spawn(async move {
		let future_cancel = cancel.clone();
		let cancel_progress = Arc::clone(&progress);
		let handle = tokio::spawn(async move {
			tokio::select! {
				biased;
				() = cancel.cancelled() => {
					cancel_progress.set_state(DownloadState::Cancelled);
					Err(Arc::new(StoreError::from(RemoteError::Cancelled)))
				}
				result = make_future(store, future_cancel) => result,
			}
		});
		let result = match handle.await {
			Ok(result) => result,
			Err(join_err) => Err(Arc::new(StoreError::DownloadTaskFailed(
				join_err.to_string(),
			))),
		};
		let terminal = match &result {
			Ok(()) => {
				progress.set_state(DownloadState::Done);
				DownloadState::Done
			}
			Err(_) => {
				if matches!(progress.snapshot().state, DownloadState::Cancelled) {
					DownloadState::Cancelled
				} else {
					progress.set_state(DownloadState::Failed);
					DownloadState::Failed
				}
			}
		};
		completion.complete(result);
		downloads.evict_if_ephemeral(&key, terminal);
		if terminal == DownloadState::Failed {
			downloads.trim_failed_slots();
		}
	});
}

fn list_rank(state: DownloadState) -> u8 {
	match state {
		DownloadState::Failed => 0,
		DownloadState::Queued
		| DownloadState::Downloading
		| DownloadState::Verifying
		| DownloadState::Committing => 1,
		DownloadState::Done | DownloadState::Cancelled => 2,
	}
}

// ── Download futures ──────────────────────────────────────────────────────────

/// Download and ingest one asset, reporting byte progress into the slot's
/// progress cell and honouring `cancel`.
async fn download_asset(
	store: Backbeat,
	asset_id: AssetId,
	cancel: CancellationToken,
) -> DownloadResult<()> {
	cancel.check_store().map_err(StoreError::shared)?;

	// Bound the complete operation, including database access, rather than
	// allowing every dependency to contend for the pool before this gate.
	let _permit = store.download_mgr.inner.requests.acquire_asset().await;

	cancel.check_store().map_err(StoreError::shared)?;

	// Re-check presence: a concurrent fetch or import may have stored it.
	if store
		.has_asset_async(asset_id)
		.await
		.map_err(StoreError::shared)?
	{
		return Ok(());
	}

	// Fetch reader + best-known total.
	let RemoteAssetReturn {
		content_length,
		reader,
	} = store
		.server_get_asset(asset_id)
		.await
		.map_err(StoreError::shared)?;

	// Register the total on the progress cell now that we know it.
	let key: DataId = asset_id.into();
	if let Some(cell) = store.download_mgr.progress_cell(&key) {
		cell.set_total(content_length);
		cell.set_state(DownloadState::Downloading);
	}

	cancel.check_store().map_err(StoreError::shared)?;

	let bytes_cell = store.download_mgr.progress_cell(&key);
	let counting = CountingReader::new(reader, bytes_cell, cancel.clone());

	if let Some(cell) = store.download_mgr.progress_cell(&key) {
		cell.set_state(DownloadState::Verifying);
	}

	internal_add_asset(&store, counting, Some(asset_id), content_length).await?;

	Ok(())
}

/// Fetch a chart's `.bb` manifest and download all missing assets in parallel
/// before importing its metadata.
async fn download_chart(
	store: Backbeat,
	chart_id: ChartId,
	cancel: CancellationToken,
) -> DownloadResult<()> {
	let key: DataId = chart_id.clone().into();
	cancel.check_store().map_err(StoreError::shared)?;

	let permit = store.download_mgr.inner.requests.acquire_manifest().await;

	if store
		.has_chart_async(&chart_id)
		.await
		.map_err(StoreError::shared)?
	{
		return Ok(());
	}

	if let Some(cell) = store.download_mgr.progress_cell(&key) {
		cell.set_state(DownloadState::Downloading);
	}

	let bb = store
		.server_get_chart(&chart_id)
		.await
		.map_err(StoreError::shared)?;
	let actual_chart_id = if chart_id.alg == backbeat_core::IdAlgorithm::Sha256 {
		ChartId {
			alg: backbeat_core::IdAlgorithm::Sha256,
			val: bb.chart_sha256().to_string(),
		}
	} else {
		bb.additional_chart_ids()
			.into_iter()
			.find(|id| id.alg == chart_id.alg)
			.ok_or_else(|| StoreError::NotFound(format!("could not calculate {}", chart_id.alg)))
			.map_err(StoreError::shared)?
	};
	if actual_chart_id != chart_id {
		return Err(StoreError::ChartIdMismatch {
			expected: chart_id,
			actual: actual_chart_id,
		}
		.into());
	}

	drop(permit);

	download_all_assets(&store, &key, &bb.assets, cancel).await?;
	store
		.import_bb_async(&bb)
		.await
		.map_err(StoreError::shared)?;

	Ok(())
}

/// Fetch a bundle's `.bb` manifest by bundle id and download all missing assets
/// before importing its metadata. Mirrors [`download_chart`], just looked up
/// by [`BundleId`] instead of by chart algorithm + id.
async fn download_bundle(
	store: Backbeat,
	bundle_id: BundleId,
	cancel: CancellationToken,
) -> DownloadResult<()> {
	let key: DataId = bundle_id.into();

	cancel.check_store().map_err(StoreError::shared)?;

	let permit = store.download_mgr.inner.requests.acquire_manifest().await;

	if store
		.has_bundle_async(bundle_id)
		.await
		.map_err(StoreError::shared)?
	{
		return Ok(());
	}

	if let Some(cell) = store.download_mgr.progress_cell(&key) {
		cell.set_state(DownloadState::Downloading);
	}

	let bb = store
		.server_get_bundle(bundle_id)
		.await
		.map_err(StoreError::shared)?;
	let actual_bundle_id = bb.bundle_id();
	if actual_bundle_id != bundle_id {
		return Err(StoreError::BundleIdMismatch {
			expected: bundle_id,
			actual: actual_bundle_id,
		}
		.into());
	}

	drop(permit);

	download_all_assets(&store, &key, &bb.assets, cancel).await?;
	store
		.import_bb_async(&bb)
		.await
		.map_err(StoreError::shared)?;

	Ok(())
}

/// Queue up downloads for all the assets that need to be downloaded
/// from this bb file.
async fn download_all_assets(
	store: &Backbeat,
	parent_key: &DataId,
	assets: &Assets,
	cancel: CancellationToken,
) -> DownloadResult<()> {
	if assets.len() >= PRECOMBINED_ASSET_THRESHOLD {
		cancel.check_store().map_err(StoreError::shared)?;
		{
			let _permit = store.download_mgr.inner.requests.acquire_asset().await;
			cancel.check_store().map_err(StoreError::shared)?;
			let combined_assets_id = backbeat_core::CombinedAssetsId::compute(assets);

			match store
				.server_get_precombined_assets(combined_assets_id)
				.await
			{
				Ok(response) => {
					tracing::debug!(
						%combined_assets_id,
						asset_count = assets.len(),
						archive_bytes = ?response.content_length,
						"using precombined asset archive"
					);
					match download_precombined_assets(
						store,
						parent_key,
						assets,
						response,
						cancel.clone(),
					)
					.await
					{
						Ok(()) => return Ok(()),
						Err(err) if matches!(&*err, StoreError::PrecombinedAssets(_)) => {
							cancel.check_store().map_err(StoreError::shared)?;
						}
						Err(err) => return Err(err),
					}
				}
				Err(StoreError::Remote(RemoteError::NotFound)) => {
					tracing::debug!(
						%combined_assets_id,
						asset_count = assets.len(),
						"no precombined available"
					);
				}
				Err(err) => return Err(err.into()),
			}
		}
	}

	queue_downloads_for_assets(store, parent_key, assets).await
}

async fn queue_downloads_for_assets(
	store: &Backbeat,
	parent_key: &DataId,
	assets: &Assets,
) -> DownloadResult<()> {
	let assets: Vec<AssetId> = assets.values().copied().collect();

	if let Some(cell) = store.download_mgr.progress_cell(parent_key) {
		cell.set_state(DownloadState::Committing);
		cell.set_items_total(assets.len() as u64);
	}

	let concurrency = store.download_mgr.inner.request_limit;
	let mut downloads = stream::iter(assets)
		.map(|asset_id| {
			let store = store.clone();
			let parent_key = parent_key.clone();

			async move {
				store.download_mgr.install_asset(&store, asset_id).await?;
				if let Some(cell) = store.download_mgr.progress_cell(&parent_key) {
					cell.add_item_done();
				}
				Ok::<(), Arc<StoreError>>(())
			}
		})
		.buffer_unordered(concurrency);

	while let Some(result) = downloads.next().await {
		result?;
	}

	Ok(())
}

async fn download_precombined_assets(
	store: &Backbeat,
	parent_key: &DataId,
	assets: &Assets,
	RemotePrecombinedAssetsReturn {
		reader,
		content_length,
	}: RemotePrecombinedAssetsReturn,
	cancel: CancellationToken,
) -> DownloadResult<()> {
	if let Some(cell) = store.download_mgr.progress_cell(parent_key) {
		cell.set_state(DownloadState::Downloading);
		cell.set_total(content_length);
	}

	let bytes_cell = store.download_mgr.progress_cell(parent_key);
	let reader = CountingReader::new(reader, bytes_cell, cancel.clone());
	let expected_assets = assets.clone();
	let staging_store = store.clone();
	let runtime = tokio::runtime::Handle::current();
	if let Some(cell) = store.download_mgr.progress_cell(parent_key) {
		cell.set_state(DownloadState::Verifying);
	}

	tokio::task::spawn_blocking(move || {
		let mut ingested_ids = HashSet::new();
		backbeat_core::precombined_assets::read_archive(
			&expected_assets,
			SyncIoBridge::new(reader),
			|_, asset_id, reader| {
				if !ingested_ids.insert(asset_id) {
					std::io::copy(reader, &mut std::io::sink())?;
					return Ok(());
				}

				let (asset_reader, asset_writer) = tokio::io::duplex(PRECOMBINED_ASSET_PIPE_BYTES);
				let (result_tx, result_rx) = std::sync::mpsc::sync_channel(1);
				let store = staging_store.clone();
				runtime.spawn(async move {
					let result =
						internal_add_asset(&store, asset_reader, Some(asset_id), None).await;
					let _ = result_tx.send(result);
				});

				{
					let mut writer = SyncIoBridge::new(asset_writer);
					std::io::copy(reader, &mut writer)?;
				}
				result_rx
					.recv()
					.map_err(|err| std::io::Error::other(err.to_string()))?
					.map_err(std::io::Error::other)?;
				Ok(())
			},
		)?;
		Ok::<_, StoreError>(())
	})
	.await
	.map_err(|err| StoreError::DownloadTaskFailed(err.to_string()))
	.map_err(StoreError::shared)?
	.map_err(StoreError::shared)?;

	cancel.check_store().map_err(StoreError::shared)?;
	if let Some(cell) = store.download_mgr.progress_cell(parent_key) {
		cell.set_state(DownloadState::Committing);
		cell.set_items_total(assets.len() as u64);
	}

	if let Some(cell) = store.download_mgr.progress_cell(parent_key) {
		for _ in assets {
			cell.add_item_done();
		}
	}

	Ok(())
}

// -- complex ingestion of assets logic --

struct AssetCommit {
	asset_id: AssetId,
	size: i64,
	inline_data: Option<Vec<u8>>,
	done: oneshot::Sender<std::result::Result<(), Arc<StoreError>>>,
}

async fn run_asset_committer(pool: sqlx::SqlitePool, mut receiver: mpsc::Receiver<AssetCommit>) {
	while let Some(first) = receiver.recv().await {
		let mut batch = Vec::with_capacity(ASSET_COMMIT_BATCH_SIZE);
		batch.push(first);
		tokio::task::yield_now().await;
		while batch.len() < ASSET_COMMIT_BATCH_SIZE {
			match receiver.try_recv() {
				Ok(commit) => batch.push(commit),
				Err(_) => break,
			}
		}

		let result = commit_asset_batch(&pool, &batch).await;
		match result {
			Ok(()) => {
				for commit in batch {
					let _ = commit.done.send(Ok(()));
				}
			}
			Err(err) => {
				let err = Arc::new(err);
				for commit in batch {
					let _ = commit.done.send(Err(Arc::clone(&err)));
				}
			}
		}
	}
}

async fn commit_asset_batch(pool: &sqlx::SqlitePool, batch: &[AssetCommit]) -> Result<()> {
	let mut tx = pool.begin().await?;
	let mut inserted = false;
	for commit in batch {
		let result = sqlx::query!(
			"INSERT OR IGNORE INTO downloaded_asset (sha256, size, inline_data) VALUES (?1, ?2, ?3)",
			commit.asset_id,
			commit.size,
			commit.inline_data,
		)
		.execute(&mut *tx)
		.await?;
		inserted |= result.rows_affected() > 0;
	}
	if inserted {
		Backbeat::increment_refresh(&mut tx).await?;
	}
	tx.commit().await?;
	Ok(())
}

/// Wraps an [`tokio::io::AsyncRead`], tallying bytes into the shared progress
/// cell and checking cancellation between reads.
struct CountingReader<R> {
	inner: R,
	progress: Option<Arc<ProgressCell>>,
	cancel: CancellationToken,
}

impl<R> CountingReader<R> {
	fn new(inner: R, progress: Option<Arc<ProgressCell>>, cancel: CancellationToken) -> Self {
		Self {
			inner,
			progress,
			cancel,
		}
	}
}

impl<R> tokio::io::AsyncRead for CountingReader<R>
where
	R: tokio::io::AsyncRead + Unpin,
{
	fn poll_read(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
		buf: &mut tokio::io::ReadBuf<'_>,
	) -> std::task::Poll<std::io::Result<()>> {
		let this = self.get_mut();
		if this.cancel.is_cancelled() {
			return std::task::Poll::Ready(Err(std::io::ErrorKind::Interrupted.into()));
		}
		let before = buf.filled().len();
		let poll = std::pin::Pin::new(&mut this.inner).poll_read(cx, buf);
		if matches!(poll, std::task::Poll::Ready(Ok(()))) {
			if this.cancel.is_cancelled() {
				return std::task::Poll::Ready(Err(std::io::ErrorKind::Interrupted.into()));
			}
			let after = buf.filled().len();
			let n = after.saturating_sub(before);
			if n > 0
				&& let Some(progress) = &this.progress
			{
				progress.add_bytes(n as u64);
				progress.set_state(DownloadState::Downloading);
			}
		}
		poll
	}
}

// ── Cancellation helper ───────────────────────────────────────────────────────

trait CheckCancelled {
	fn check_store(&self) -> Result<()>;
}

impl CheckCancelled for CancellationToken {
	fn check_store(&self) -> Result<()> {
		if self.is_cancelled() {
			return Err(StoreError::from(RemoteError::Cancelled));
		}
		Ok(())
	}
}

// this function fucking sucks and is too complex
// there's lots of cross-pollution with this API and
// the download manager and as such it needlessly
// touches a bunch of "batching" shit itd.
pub(crate) async fn internal_add_asset(
	store: &Backbeat,
	mut reader: impl AsyncRead + Unpin + Send,
	expected_id: Option<AssetId>,
	known_size: Option<u64>,
) -> std::result::Result<(), Arc<StoreError>> {
	/// Build up the bytes from the reader in memory and just write it
	/// to the store.
	async fn ingest_inline_from_reader(
		store: &Backbeat,
		reader: &mut (impl AsyncRead + Unpin + Send),
		expected_id: Option<AssetId>,
		size: usize,
	) -> std::result::Result<(), Arc<StoreError>> {
		let mut data = vec![0u8; size];
		reader
			.read_exact(&mut data)
			.await
			.map_err(StoreError::shared)?;

		let got_id = AssetId(Sha256::checksum_bytes(&data));
		if expected_id.is_some_and(|e| got_id != e) {
			return Err(StoreError::HashMismatch.into());
		}

		if size as u64 > store.config.read().store.inline.0 {
			let assets = store.assets.clone();
			tokio::task::spawn_blocking(move || assets.store(got_id, &data))
				.await
				.map_err(|err| StoreError::DownloadTaskFailed(err.to_string()))
				.map_err(StoreError::shared)?
				.map_err(StoreError::shared)?;
			return store
				.download_mgr
				.commit_asset(store, got_id, size as i64, None)
				.await;
		}

		store
			.download_mgr
			.commit_asset(store, got_id, size as i64, Some(data))
			.await
	}

	/// Stream the results of the reader to `.downloading` and then atomically rename the file into the right place.
	async fn ingest_staged_from_reader(
		store: &Backbeat,
		mut reader: impl AsyncRead + Unpin + Send,
		expected_id: Option<AssetId>,
	) -> std::result::Result<(), Arc<StoreError>> {
		let filename = match expected_id {
			Some(v) => v.to_string(),
			None => Uuid::new_v4().to_string(),
		};

		// Write to temp file while teeing every chunk to the hasher.
		let temp_path = store.store_dir().join(".downloading").join(&filename);
		let mut file = tokio::fs::File::create(&temp_path)
			.await
			.map_err(StoreError::shared)?;
		let mut digest = Sha256Digest::new();

		/// Buffer size for the tee loop (64 KiB).
		#[allow(clippy::items_after_statements)]
		const BUF_SIZE: usize = 64 * 1024;
		let mut buf = vec![0u8; BUF_SIZE];

		loop {
			let n = reader.read(&mut buf).await.map_err(StoreError::shared)?;
			if n == 0 {
				break;
			}
			file.write_all(&buf[..n])
				.await
				.map_err(StoreError::shared)?;
			digest.update(&buf[..n]);
		}

		file.flush().await.map_err(StoreError::shared)?;

		let got_id = AssetId(digest.finalize());
		// Verify hash before touching the permanent store.
		if expected_id.is_some_and(|e| e != got_id) {
			drop(file);
			let _ = tokio::fs::remove_file(&temp_path).await;
			return Err(StoreError::HashMismatch.into());
		}

		let size = file.metadata().await.map_err(StoreError::shared)?.len();
		drop(file);

		let threshold = store.config.read().store.inline.0;

		if size <= threshold {
			// Obscure lol: if streaming < inline threshold...
			// don't even atomically rename the file, just... read it into memory and put it on disk.
			//
			// users shouldn't do this.
			let data = tokio::fs::read(&temp_path)
				.await
				.map_err(StoreError::shared)?;

			store
				.download_mgr
				.commit_asset(store, got_id, size.cast_signed(), Some(data))
				.await?;

			let _ = tokio::fs::remove_file(&temp_path).await;
		} else {
			let asset = got_id;
			let dest_path = store.assets.asset_path(asset);
			if let Some(parent) = dest_path.parent() {
				tokio::fs::create_dir_all(parent)
					.await
					.map_err(StoreError::shared)?;
			}
			tokio::fs::rename(&temp_path, &dest_path)
				.await
				.map_err(StoreError::shared)?;
			let size_i64 = size.cast_signed();
			store
				.download_mgr
				.commit_asset(store, got_id, size_i64, None)
				.await?;
		}

		Ok(())
	}

	let stream = store
		.config
		.read()
		.downloads
		.stream
		.as_usize()
		.unwrap_or(usize::MAX);

	if let Some(size) = known_size
		&& let Ok(size) = usize::try_from(size)
		&& size <= stream
	{
		let _buffer_permit = store
			.download_mgr
			.acquire_buffer_budget(size)
			.await
			.map_err(StoreError::shared)?;

		return ingest_inline_from_reader(store, &mut reader, expected_id, size).await;
	}

	ingest_staged_from_reader(store, reader, expected_id).await
}

#[cfg(test)]
mod tests {
	use backbeat_core::Sha256;
	use tokio::sync::oneshot;

	use super::*;

	#[test]
	fn progress_snapshot_includes_asset_and_parent_progress() {
		let progress = ProgressCell::new();
		progress.add_bytes(128);
		progress.set_total(Some(512));
		progress.set_items_total(3);
		progress.add_item_done();

		let snapshot = progress.snapshot();
		assert_eq!(snapshot.bytes, 128);
		assert_eq!(snapshot.total, Some(512));
		assert_eq!(snapshot.items_done, 1);
		assert_eq!(snapshot.items_total, Some(3));
	}

	#[test]
	fn list_rows_has_stable_page_boundaries() {
		let rows: Vec<_> = ["one", "two", "three", "four"]
			.into_iter()
			.map(|value| DownloadSnapshot {
				key: DataId::Asset(Sha256::checksum_bytes(value.as_bytes()).into()),
				progress: DownloadProgress {
					bytes: 0,
					total: None,
					items_done: 0,
					items_total: None,
					state: DownloadState::Queued,
				},
				error: None,
			})
			.collect();

		let first = list_rows(rows.clone(), 1, 2);
		let second = list_rows(rows.into_iter().rev().collect(), 1, 2);
		let first_keys: Vec<_> = first.downloads.into_iter().map(|row| row.key).collect();
		let second_keys: Vec<_> = second.downloads.into_iter().map(|row| row.key).collect();

		assert_eq!(first_keys, second_keys);
		assert!(first.has_more);
		assert!(second.has_more);
	}

	#[test]
	fn list_rows_handles_an_extreme_offset() {
		let rows = vec![DownloadSnapshot {
			key: DataId::Asset(Sha256::checksum_bytes(b"one").into()),
			progress: DownloadProgress {
				bytes: 0,
				total: None,
				items_done: 0,
				items_total: None,
				state: DownloadState::Queued,
			},
			error: None,
		}];

		let result = list_rows(rows, u64::MAX, 10);

		assert!(result.downloads.is_empty());
		assert!(!result.has_more);
		assert_eq!(result.total, 1);
	}

	#[tokio::test(flavor = "multi_thread")]
	async fn cancellation_stops_the_download_driver() {
		let (_tmp, store) = crate::test_util::new_test_store("download_manager_cancel_test");
		let manager = DownloadManager::new(1, BUFFER_PERMIT_BYTES);
		let key = DataId::Bundle(Sha256::checksum_bytes(b"cancel-test").into());
		let task_manager = manager.clone();
		let task_key = key.clone();
		let task = tokio::spawn(async move {
			task_manager
				.run_shared(&store, task_key, |_store, _cancel| {
					Box::pin(std::future::pending())
				})
				.await
		});
		while manager.progress(&key).is_none() {
			tokio::task::yield_now().await;
		}

		assert!(manager.cancel_download(key.clone()));
		assert!(
			tokio::time::timeout(std::time::Duration::from_secs(1), task)
				.await
				.expect("cancelled driver should stop")
				.unwrap()
				.is_err()
		);
		let snapshot = manager.all_progress().pop().unwrap();
		assert_eq!(snapshot.progress.state, DownloadState::Cancelled);

		let progress = Arc::new(ProgressCell::new());
		progress.set_state(DownloadState::Cancelled);
		let cancel = CancellationToken::new();
		cancel.cancel();
		let mut reader = CountingReader::new(&b"data"[..], Some(Arc::clone(&progress)), cancel);
		assert!(reader.read_to_end(&mut Vec::new()).await.is_err());
		assert_eq!(progress.snapshot().state, DownloadState::Cancelled);
	}

	#[tokio::test(flavor = "multi_thread")]
	async fn request_scheduler_prioritises_assets_over_waiting_manifests() {
		let scheduler = Arc::new(RequestScheduler::new(1));
		let first_manifest = scheduler.acquire_manifest().await;

		let (manifest_tx, mut manifest_rx) = oneshot::channel();
		let manifest_scheduler = Arc::clone(&scheduler);
		tokio::spawn(async move {
			let permit = manifest_scheduler.acquire_manifest().await;
			let _ = manifest_tx.send(permit);
		});
		while scheduler.state.lock().manifest_waiters == 0 {
			tokio::task::yield_now().await;
		}

		let (asset_tx, asset_rx) = oneshot::channel();
		let asset_scheduler = Arc::clone(&scheduler);
		tokio::spawn(async move {
			let permit = asset_scheduler.acquire_asset().await;
			let _ = asset_tx.send(permit);
		});
		while scheduler.state.lock().asset_waiters == 0 {
			tokio::task::yield_now().await;
		}

		drop(first_manifest);
		let asset_permit = tokio::time::timeout(std::time::Duration::from_secs(1), asset_rx)
			.await
			.expect("asset request should acquire the freed slot")
			.expect("asset task should not be cancelled");
		assert!(manifest_rx.try_recv().is_err());

		drop(asset_permit);
		let _manifest_permit = tokio::time::timeout(std::time::Duration::from_secs(1), manifest_rx)
			.await
			.expect("manifest request should acquire after the asset completes")
			.expect("manifest task should not be cancelled");
	}

	/// A download future that panics must not hang the caller (or any
	/// concurrent waiter) forever: [`spawn_driver`] routes the future
	/// through an inner `JoinHandle` specifically so a panic surfaces as an
	/// ordinary `Err` via `completion.complete(...)` instead of unwinding
	/// past it and leaving `Completion::result` empty forever.
	#[tokio::test(flavor = "multi_thread")]
	async fn panicking_download_future_does_not_hang_waiters() {
		let (_tmp, store) = crate::test_util::new_test_store("download_manager_panic_test");

		let manager = DownloadManager::new(1, 32 * 1024 * 1024);

		let key = DataId::Asset(Sha256::checksum_bytes(b"panic-test").into());

		let primary = tokio::time::timeout(
			std::time::Duration::from_secs(5),
			manager.run_shared(&store, key.clone(), |_store, _cancel| {
				Box::pin(async { panic!("simulated download panic") })
			}),
		);

		let waiter_manager = manager.clone();
		let waiter_store = store.clone();
		let waiter_key = key.clone();
		let waiter = tokio::spawn(async move {
			tokio::time::timeout(
				std::time::Duration::from_secs(5),
				waiter_manager.run_shared(&waiter_store, waiter_key, |_store, _cancel| {
					Box::pin(async { panic!("second caller should never run this") })
				}),
			)
			.await
		});

		let primary_result = primary
			.await
			.expect("primary caller must not hang when the download future panics");
		assert!(primary_result.is_err());

		let waiter_result = waiter
			.await
			.expect("waiter task itself should not panic")
			.expect("waiter must not hang when the download future panics");
		assert!(waiter_result.is_err());
	}

	#[tokio::test]
	async fn buffer_budget_is_independent_from_request_limit() {
		let manager = DownloadManager::new(32, 2 * BUFFER_PERMIT_BYTES);

		let first = manager.acquire_buffer_budget(1).await.unwrap();
		let second = manager
			.acquire_buffer_budget(BUFFER_PERMIT_BYTES as usize)
			.await
			.unwrap();

		assert_eq!(manager.inner.buffer_bytes.available_permits(), 0);
		assert_eq!(manager.inner.requests.available_slots(), 32);

		drop(first);
		assert_eq!(manager.inner.buffer_bytes.available_permits(), 1);
		drop(second);
	}
}
