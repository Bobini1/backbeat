import Lock from "lucide-solid/icons/lock";
import LockOpen from "lucide-solid/icons/lock-open";
import { createEffect, createMemo, createResource, createSignal, For, Show } from "solid-js";

import { HashField } from "../components/HashField";
import { PageHeader } from "../components/PageHeader";
import Banner from "../components/primitives/Banner";
import Button from "../components/primitives/Button";
import ExternalLink from "../components/primitives/ExternalLink";
import NiceLabel from "../components/primitives/NiceLabel";
import Section from "../components/primitives/Section";
import TextInput from "../components/primitives/TextInput";
import {
	assetPrune,
	corruptionCheck,
	type CorruptionCheckResponse,
	corruptionRepair,
	diskPrune,
	getSettings,
	openConfigFolder,
	openLogsFolder,
	saveSettings,
} from "../lib/tauri";
import { setThemePref, themePref, type ThemePref } from "../lib/theme";
import styles from "./Settings.module.css";

const THEME_OPTIONS: { label: string; value: ThemePref }[] = [
	{ value: "system", label: "System" },
	{ value: "light", label: "Light" },
	{ value: "dark", label: "Dark" },
];

const BYTE_SUFFIXES = new Set(["", "g", "gi", "k", "ki", "m", "mi", "t", "ti"]);
const BYTE_MULTIPLIERS: Record<string, bigint> = {
	"": 1n,
	ki: 1024n,
	mi: 1048576n,
	gi: 1073741824n,
	ti: 1099511627776n,
	k: 1000n,
	m: 1000000n,
	g: 1000000000n,
	t: 1000000000000n,
};
const U64_MAX = 18446744073709551615n;
const MAX_DOWNLOAD_CONCURRENCY = 1000;

type MaintenanceAction = "asset-prune" | "check" | "disk-prune" | "repair";

function corruptionDetails(report: CorruptionCheckResponse): string {
	const issues: Array<[number, string]> = [
		[report.missing_assets, "missing assets"],
		[report.corrupt_assets, "corrupt assets"],
		[report.corrupt_charts, "corrupt charts"],
		[report.wrong_chart_ids, "wrong chart IDs"],
		[report.uncomputable_chart_ids, "uncomputable chart IDs"],
		[report.dangling_asset_refs, "dangling asset references"],
	];
	return issues
		.filter(([count]) => count > 0)
		.map(([count, label]) => `${count} ${label}`)
		.join(", ");
}

/**
 * Validate a k8s-style byte quantity string the same way the Rust `ByteSize`
 * parser does. Returns a human message when invalid, or null when valid.
 */
function validateByteQuantity(input: string): null | string {
	const s = input.trim();
	if (s === "") {
		return "Enter a size, e.g. 16Ki or 8Mi.";
	}
	const split = s.search(/\D/);
	const idx = split === -1 ? s.length : split;
	const number = s.slice(0, idx);
	const suffix = s.slice(idx).toLowerCase();
	if (number === "" || !/^\d+$/.test(number)) {
		return "Start with a number, e.g. 16Ki.";
	}
	if (!BYTE_SUFFIXES.has(suffix)) {
		return `Unknown unit "${suffix}". Use Ki, Mi, Gi, Ti, k, M, G, or T.`;
	}
	if (BigInt(number) * BYTE_MULTIPLIERS[suffix] > U64_MAX) {
		return "That size is too large.";
	}
	return null;
}

function validateConcurrency(input: string): null | string {
	const s = input.trim();
	if (s === "") {
		return "Enter a number.";
	}
	if (!/^\d+$/.test(s)) {
		return "Use a whole number.";
	}
	if (Number(s) < 1) {
		return "Must be at least 1.";
	}
	if (Number(s) > MAX_DOWNLOAD_CONCURRENCY) {
		return `Must be at most ${MAX_DOWNLOAD_CONCURRENCY}.`;
	}
	return null;
}

/** Turn a raw backend save error into plain guidance for a non-developer. */
function describeSettingsError(err: unknown): string {
	const raw = String(err);
	if (/byte quantity|suffix|invalid/i.test(raw)) {
		return "One of the size fields isn't a valid byte quantity (e.g. 16Ki, 8Mi).";
	}
	if (/concurrency/i.test(raw)) {
		return `Download concurrency must be a whole number from 1 to ${MAX_DOWNLOAD_CONCURRENCY}.`;
	}
	return raw;
}

export default function Settings() {
	const [settings, { mutate }] = createResource(getSettings);
	const [unlocked, setUnlocked] = createSignal(false);
	const [storeInline, setStoreInline] = createSignal("");
	const [downloadConcurrency, setDownloadConcurrency] = createSignal("8");
	const [downloadStream, setDownloadStream] = createSignal("");
	const [saving, setSaving] = createSignal(false);
	const [saveError, setSaveError] = createSignal<null | string>(null);
	const [saveSuccess, setSaveSuccess] = createSignal(false);
	const [logsError, setLogsError] = createSignal<null | string>(null);
	const [configError, setConfigError] = createSignal<null | string>(null);
	const [maintenanceAction, setMaintenanceAction] = createSignal<MaintenanceAction | null>(null);
	const [maintenanceMessage, setMaintenanceMessage] = createSignal<null | string>(null);
	const [maintenanceError, setMaintenanceError] = createSignal<null | string>(null);
	const [corruptionResult, setCorruptionResult] = createSignal<CorruptionCheckResponse | null>(
		null,
	);

	/** Snapshot of the values last loaded from disk, for dirty-checking. */
	const [loaded, setLoaded] = createSignal({
		inline: "",
		concurrency: "",
		stream: "",
	});

	createEffect(() => {
		const data = settings();
		if (!data) {
			return;
		}
		setStoreInline(data.store_inline);
		setDownloadConcurrency(String(data.download_concurrency));
		setDownloadStream(data.download_stream);
		setLoaded({
			inline: data.store_inline,
			concurrency: String(data.download_concurrency),
			stream: data.download_stream,
		});
		setSaveSuccess(false);
	});

	const inlineError = createMemo(() => validateByteQuantity(storeInline()));
	const streamError = createMemo(() => validateByteQuantity(downloadStream()));
	const concurrencyError = createMemo(() => validateConcurrency(downloadConcurrency()));
	const valid = createMemo(() => !inlineError() && !streamError() && !concurrencyError());

	const dirty = createMemo(() => {
		const l = loaded();
		return (
			storeInline() !== l.inline ||
			downloadConcurrency() !== l.concurrency ||
			downloadStream() !== l.stream
		);
	});

	async function handleSave(event: SubmitEvent) {
		event.preventDefault();
		if (!valid()) {
			return;
		}
		setSaving(true);
		setSaveError(null);
		setSaveSuccess(false);
		try {
			const saved = await saveSettings({
				store_inline: storeInline(),
				download_concurrency: Number(downloadConcurrency()),
				download_stream: downloadStream(),
			});
			mutate(saved);
			setSaveSuccess(true);
		} catch (err) {
			setSaveError(describeSettingsError(err));
		} finally {
			setSaving(false);
		}
	}

	async function openConfig() {
		setConfigError(null);
		try {
			await openConfigFolder();
		} catch (err) {
			setConfigError(String(err));
		}
	}

	async function openLogs() {
		setLogsError(null);
		try {
			await openLogsFolder();
		} catch (err) {
			setLogsError(String(err));
		}
	}

	function beginMaintenance(action: MaintenanceAction) {
		setMaintenanceAction(action);
		setMaintenanceMessage(null);
		setMaintenanceError(null);
	}

	async function handleAssetPrune() {
		beginMaintenance("asset-prune");
		setCorruptionResult(null);
		try {
			const removed = await assetPrune();
			setMaintenanceMessage(
				`Removed ${removed} unused ${removed === 1 ? "asset" : "assets"}.`,
			);
		} catch (err) {
			setMaintenanceError(String(err));
		} finally {
			setMaintenanceAction(null);
		}
	}

	async function handleDiskPrune() {
		beginMaintenance("disk-prune");
		setCorruptionResult(null);
		try {
			const removed = await diskPrune();
			setMaintenanceMessage(`Removed ${removed} orphan ${removed === 1 ? "file" : "files"}.`);
		} catch (err) {
			setMaintenanceError(String(err));
		} finally {
			setMaintenanceAction(null);
		}
	}

	async function handleCorruptionCheck() {
		beginMaintenance("check");
		setCorruptionResult(null);
		try {
			setCorruptionResult(await corruptionCheck());
		} catch (err) {
			setMaintenanceError(String(err));
		} finally {
			setMaintenanceAction(null);
		}
	}

	async function handleCorruptionRepair() {
		if (!corruptionResult()) {
			return;
		}
		beginMaintenance("repair");
		try {
			await corruptionRepair();
			setCorruptionResult(null);
			setMaintenanceMessage(
				"Repair complete. Run the corruption check again to verify the store.",
			);
		} catch (err) {
			setMaintenanceError(String(err));
		} finally {
			setMaintenanceAction(null);
		}
	}

	return (
		<>
			<PageHeader
				subtitle="Where Backbeat keeps its config and how it tunes downloads."
				title="Settings"
			/>

			<Section heading="Theme">
				<div class={styles.themeSwitch} role="radiogroup">
					<For each={THEME_OPTIONS}>
						{(opt) => (
							<button
								aria-checked={themePref() === opt.value}
								class={styles.themeSwitchOption}
								onClick={() => setThemePref(opt.value)}
								role="radio"
								type="button"
							>
								{opt.label}
							</button>
						)}
					</For>
				</div>
			</Section>

			<Show when={settings.loading}>
				<Banner variant="base">Loading settings…</Banner>
			</Show>

			<Show when={settings.error}>
				<Banner variant="danger">Failed to load settings: {String(settings.error)}</Banner>
			</Show>

			<Show when={settings()}>
				{(snapshot) => (
					<>
						<Section heading="File Locations">
							<HashField
								label="Configuration File"
								onOpen={openConfig}
								value={snapshot().config_file}
							/>

							<Show when={configError()}>
								<Banner variant="danger">{configError()}</Banner>
							</Show>

							<HashField
								label="Log folder"
								onOpen={openLogs}
								value={snapshot().log_dir}
							/>

							<Show when={logsError()}>
								<Banner variant="danger">{logsError()}</Banner>
							</Show>
						</Section>

						<Section heading="Maintenance">
							<div class={styles.maintenanceActions}>
								<div class={styles.maintenanceAction}>
									<Button
										aria-describedby="asset-prune-description"
										disabled={maintenanceAction() !== null}
										onClick={handleAssetPrune}
										variant="base"
									>
										{maintenanceAction() === "asset-prune"
											? "Pruning…"
											: "Delete unused assets (quick)"}
									</Button>
									<p
										class={styles.maintenanceDescription}
										id="asset-prune-description"
									>
										Deletes assets that are no longer used.
									</p>
								</div>
								<div class={styles.maintenanceAction}>
									<Button
										aria-describedby="disk-prune-description"
										disabled={maintenanceAction() !== null}
										onClick={handleDiskPrune}
										variant="base"
									>
										{maintenanceAction() === "disk-prune"
											? "Pruning…"
											: "Force Disk Prune (expensive)"}
									</Button>
									<p
										class={styles.maintenanceDescription}
										id="disk-prune-description"
									>
										Delete any files on-disk that are untracked. This is a
										slower version of the above option, but can catch database
										corruption.
									</p>
								</div>
								<div class={styles.maintenanceAction}>
									<Button
										aria-describedby="corruption-check-description"
										disabled={maintenanceAction() !== null}
										onClick={handleCorruptionCheck}
										variant="base"
									>
										{maintenanceAction() === "check"
											? "Checking…"
											: "Check for corruption"}
									</Button>
									<p
										class={styles.maintenanceDescription}
										id="corruption-check-description"
									>
										Scans charts, assets, and chart IDs for corruption without
										changing anything.
									</p>
								</div>
								<div class={styles.maintenanceAction}>
									<Button
										aria-describedby="corruption-repair-description"
										disabled={
											maintenanceAction() !== null || !corruptionResult()
										}
										onClick={handleCorruptionRepair}
										variant="danger"
									>
										{maintenanceAction() === "repair"
											? "Repairing…"
											: "Repair corruption"}
									</Button>
									<p
										class={styles.maintenanceDescription}
										id="corruption-repair-description"
									>
										Run a check first. Rebuilds your database and removes
										corrupt content.
									</p>
								</div>
							</div>

							<Show when={corruptionResult()}>
								{(report) => (
									<Banner variant={report().is_ok ? "success" : "danger"}>
										<span>
											<strong>
												{report().is_ok
													? "No corruption found."
													: `${report().issue_count} corruption ${report().issue_count === 1 ? "issue" : "issues"} found.`}
											</strong>
											<br />
											Checked {report().chart_count} charts,{" "}
											{report().large_asset_count} on-disk assets, and{" "}
											{report().chart_id_count} chart IDs.
											<Show when={corruptionDetails(report())}>
												<>
													<br />
													{corruptionDetails(report())}
												</>
											</Show>
										</span>
									</Banner>
								)}
							</Show>

							<Show when={maintenanceMessage()}>
								<Banner variant="success">{maintenanceMessage()}</Banner>
							</Show>
							<Show when={maintenanceError()}>
								<Banner variant="danger">{maintenanceError()}</Banner>
							</Show>
						</Section>

						<Section
							detail={
								<button
									aria-controls="settings-form"
									aria-pressed={unlocked()}
									class={styles.unlockButton}
									onClick={() => setUnlocked((prev) => !prev)}
									role="switch"
									type="button"
								>
									{unlocked() ? "Lock" : "Unlock"}
									{unlocked() ? <LockOpen size={12} /> : <Lock size={12} />}
								</button>
							}
							heading="Advanced"
						>
							<Banner variant="danger">
								<span>
									Performance knobs for downloads and storage. Changes only apply
									after restarting Backbeat. <br />
									<strong style={{ "font-size": "1rem" }}>
										The defaults are good. Don't touch them unless you know what
										you're doing.
									</strong>
								</span>
							</Banner>

							<form
								aria-disabled={!unlocked()}
								class={styles.settingsForm}
								id="settings-form"
								onSubmit={handleSave}
							>
								<div class={styles.group}>
									<div>
										<NiceLabel as="label" for="store-inline-input">
											Inline Threshold
										</NiceLabel>
										<TextInput
											aria-describedby="store-inline-hint"
											aria-invalid={Boolean(inlineError())}
											id="store-inline-input"
											label="Inline threshold"
											onInput={(e) => setStoreInline(e.currentTarget.value)}
											required
											spellcheck={false}
											type="text"
											value={storeInline()}
										/>
									</div>
									<Banner id="store-inline-hint">
										<span>
											Assets bigger than this are stored as external blobs.
											<br />
											Assets smaller than this are stored in the sqlite
											database.
											<br />
											Read more here:{" "}
											<ExternalLink href="https://sqlite.org/intern-v-extern-blob.html">
												https://sqlite.org/intern-v-extern-blob.html
											</ExternalLink>
										</span>
									</Banner>
									<Show when={unlocked() && inlineError()}>
										<Banner variant="danger">{inlineError()}</Banner>
									</Show>
								</div>

								<div class={styles.group}>
									<div>
										<NiceLabel as="label" for="download-concurrency-input">
											Download Concurrency
										</NiceLabel>
										<TextInput
											aria-describedby="download-concurrency-hint"
											aria-invalid={!!concurrencyError()}
											id="download-concurrency-input"
											max={MAX_DOWNLOAD_CONCURRENCY}
											min={1}
											onInput={(e) =>
												setDownloadConcurrency(e.currentTarget.value)
											}
											required
											step={1}
											type="number"
											value={downloadConcurrency()}
										/>
									</div>
									<Banner>
										How many asset downloads Backbeat runs at the same time.
									</Banner>
									<Show when={unlocked() && concurrencyError()}>
										<Banner variant="danger">{concurrencyError()}</Banner>
									</Show>
								</div>

								<div class={styles.group}>
									<div>
										<NiceLabel as="label" for="download-stream-input">
											Stream Threshold
										</NiceLabel>
										<TextInput
											aria-describedby="download-stream-hint"
											aria-invalid={!!streamError()}
											id="download-stream-input"
											onInput={(e) =>
												setDownloadStream(e.currentTarget.value)
											}
											required
											spellcheck={false}
											type="text"
											value={downloadStream()}
										/>
									</div>
									<Banner id="download-stream-hint">
										<span>
											Assets bigger than this download straight to disk
											instead of buffering in memory. Examples:{" "}
											<code>8Mi</code>, <code>100Mi</code>.
										</span>
									</Banner>
									<Show when={unlocked() && streamError()}>
										<Banner variant="danger">{streamError()}</Banner>
									</Show>
								</div>

								<div class={styles.group}>
									<Show when={unlocked()}>
										<Button
											disabled={saving() || !dirty() || !valid()}
											type="submit"
										>
											{saving() ? "Saving…" : "Save settings"}
										</Button>
									</Show>

									<Show when={saveError()}>
										<Banner variant="danger">{saveError()}</Banner>
									</Show>

									<Show when={saveSuccess()}>
										<Banner variant="success">
											Settings saved. Restart Backbeat for changes to take
											effect.
										</Banner>
									</Show>
								</div>
							</form>
						</Section>
					</>
				)}
			</Show>
		</>
	);
}
