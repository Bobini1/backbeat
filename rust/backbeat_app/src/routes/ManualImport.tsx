import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { FileArchive } from "lucide-solid";
import { createSignal, onCleanup, onMount, Show } from "solid-js";

import { PageHeader } from "../components/PageHeader";
import Banner from "../components/primitives/Banner";
import { importFiles } from "../lib/tauri";
import styles from "./ManualImport.module.css";

function canImport(paths: string[]) {
	return (
		paths.length > 0 &&
		paths.every((path) => {
			const lower = path.toLowerCase();
			return lower.endsWith(".bb") || lower.endsWith(".bbzip");
		})
	);
}

export default function ManualImport() {
	const [draggingImport, setDraggingImport] = createSignal(false);
	const [importing, setImporting] = createSignal(false);
	const [importedCount, setImportedCount] = createSignal<null | number>(null);
	const [importError, setImportError] = createSignal<null | string>(null);
	let unlisten: (() => void) | undefined;

	onMount(() => {
		getCurrentWebview()
			.onDragDropEvent(({ payload }) => {
				if (payload.type === "enter") {
					setDraggingImport(!importing() && canImport(payload.paths));
				} else if (payload.type === "drop") {
					setDraggingImport(false);
					void importPaths(payload.paths);
				} else if (payload.type === "leave") {
					setDraggingImport(false);
				}
			})
			.then((stop) => {
				unlisten = stop;
			});
	});

	onCleanup(() => unlisten?.());

	async function importPaths(paths: string[]) {
		if (importing() || !canImport(paths)) {
			return;
		}

		setImporting(true);
		setImportedCount(null);
		setImportError(null);
		try {
			setImportedCount(await importFiles(paths));
		} catch (err) {
			setImportError(String(err));
		} finally {
			setImporting(false);
		}
	}

	async function chooseFiles() {
		const selected = await open({
			multiple: true,
			filters: [{ name: "Backbeat files", extensions: ["bb", "bbzip"] }],
		});
		if (selected) {
			await importPaths(Array.isArray(selected) ? selected : [selected]);
		}
	}

	return (
		<>
			<PageHeader subtitle="Manually install .bbzip and .bb files." title="Manual Import" />
			<button
				class={styles.dropzone}
				data-active={draggingImport()}
				disabled={importing()}
				onClick={chooseFiles}
				type="button"
			>
				<FileArchive aria-hidden="true" class={styles.icon} />
				<strong>
					{draggingImport()
						? "Drop to import"
						: importing()
							? "Importing…"
							: "Choose or drop files"}
				</strong>
				<span>.bb and .bbzip files</span>
			</button>
			<Show when={importedCount()}>
				{(count) => (
					<Banner variant="success">
						Imported {count()} {count() === 1 ? "file" : "files"}.
					</Banner>
				)}
			</Show>
			<Show when={importError()}>
				{(error) => <Banner variant="danger">{error()}</Banner>}
			</Show>
		</>
	);
}
