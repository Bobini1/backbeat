import { open } from "@tauri-apps/plugin-dialog";
import { CircleAlert, ExternalLink, LoaderCircle, Settings2 } from "lucide-solid";
import { createEffect, createSignal, For, onCleanup, onMount, Show } from "solid-js";

import {
	type FuseInstaller,
	getFuseInstaller,
	getHomeDir,
	listVirtualFolders,
	openConfigFolder,
	openFuseInstaller,
	openVirtualFolder,
	setVirtualFolderEnabled,
	setVirtualFolderPath,
	type VirtualFolder,
	type VirtualFolderKind,
} from "./lib/tauri";

const FOLDERS: { kind: VirtualFolderKind; name: string; placeholder: string }[] = [
	{ kind: "bms", name: "BMS", placeholder: "bms" },
	{ kind: "kshoot", name: "K-Shoot", placeholder: "kshoot" },
	{ kind: "stepmania", name: "StepMania", placeholder: "stepmania" },
];

export function App() {
	const [folders, setFolders] = createSignal<VirtualFolder[]>([]);
	const [error, setError] = createSignal<string>();
	const [fuseInstaller, setFuseInstaller] = createSignal<FuseInstaller>();
	const [homeDir, setHomeDir] = createSignal<string>();
	const [loading, setLoading] = createSignal(true);

	const refresh = async () => {
		try {
			setFolders(await listVirtualFolders());
			setError(undefined);
		} catch (reason) {
			setError(String(reason));
		} finally {
			setLoading(false);
		}
	};

	onMount(() => {
		void refresh();
		void getHomeDir().then(setHomeDir);
		void getFuseInstaller().then(setFuseInstaller);
		const interval = window.setInterval(() => void refresh(), 5_000);
		onCleanup(() => window.clearInterval(interval));
	});

	const folderFor = (kind: VirtualFolderKind) =>
		folders().find((folder) => folder.folder === kind);
	const supported = () => folders().some((folder) => folder.status.mount_supported);

	return (
		<main>
			<header class="masthead">
				<div class="brand">
					<h1>Virtual Folders for Backbeat</h1>
				</div>
				<div class="header-actions">
					<Show when={loading() || error()}>
						<div class="daemon-state" classList={{ offline: Boolean(error()) }}>
							<span class="indicator" />
							{loading()
								? "Virtual Folder Service: Connecting"
								: "Virtual Folder Service: Unavailable"}
						</div>
					</Show>
					<button
						aria-label="Open configuration"
						class="configuration-button"
						onClick={() => void openConfigFolder()}
					>
						<Settings2 size={18} />
					</button>
				</div>
			</header>

			<section class="intro">
				<p>
					Use operating system trickery to make your Backbeat store look like a normal
					game folder.
					<br />
					This allows games without Backbeat compatibility to still work, although not all
					functionality will work.
				</p>
			</section>

			<section class="game-client-note">
				<p>
					After turning on a virtual folder, you should configure your game client to read
					from it.
					<br />
					How to do this depends on what game you're using, and you'll have to look it up.
					<br />
					You do not need to keep this app open. You can close it after enabling what you
					want.
				</p>
			</section>

			<Show when={error()}>
				{(message) => (
					<div class="notice">
						<CircleAlert size={19} />
						<div>
							<strong>Could not reach the mount daemon.</strong>
							<span>{message()}</span>
						</div>
						<button
							onClick={() => {
								setLoading(true);
								void refresh();
							}}
						>
							Retry
						</button>
					</div>
				)}
			</Show>
			<Show when={!loading() && !supported() && !error()}>
				<div class="availability-notice notice">
					<CircleAlert size={24} />
					<div>
						<strong>You're missing some software for virtual folders to work.</strong>
						<span>
							We can't do anything until this software is installed :P.
							<br />
							It's nerd stuff, but it's also the only way to create virtual folders.
							<br />
							There's a link on the right. Click it and follow the instructions for
							your platform.
							<br />
							You should then be able to restart the app.
						</span>
					</div>
					<Show when={fuseInstaller()}>
						{(installer) => (
							<a
								class="install-link"
								href={installer().url}
								onClick={(event) => {
									event.preventDefault();
									void openFuseInstaller();
								}}
								rel="noreferrer"
								target="_blank"
							>
								Install {installer().name}
							</a>
						)}
					</Show>
				</div>
			</Show>

			<Show when={loading() || supported()}>
				<section aria-label="Virtual folders" class="mount-list">
					<For each={FOLDERS}>
						{(definition) => (
							<FolderCard
								definition={definition}
								disabled={Boolean(error()) || !supported()}
								folder={folderFor(definition.kind)}
								homeDir={homeDir()}
								refresh={refresh}
							/>
						)}
					</For>
				</section>
			</Show>
		</main>
	);
}

function FolderCard(props: {
	definition: (typeof FOLDERS)[number];
	disabled: boolean;
	folder: undefined | VirtualFolder;
	homeDir: string | undefined;
	refresh: () => Promise<void>;
}) {
	const [path, setPath] = createSignal("");
	const [busy, setBusy] = createSignal(false);
	const [pathChanged, setPathChanged] = createSignal(false);
	const [message, setMessage] = createSignal<string>();
	createEffect(() => {
		if (!pathChanged()) {
			setPath(props.folder?.status.mount_path ?? "");
		}
	});

	const choosePath = async () => {
		const chosen = await open({
			directory: true,
			multiple: false,
			title: `Choose ${props.definition.name} folder`,
		});
		if (typeof chosen === "string") {
			setPath(chosen);
			setBusy(true);
			setMessage(undefined);
			try {
				await setVirtualFolderPath(props.definition.kind, chosen);
				setPathChanged(false);
				await props.refresh();
			} catch (reason) {
				setMessage(String(reason));
			} finally {
				setBusy(false);
			}
		}
	};
	const enable = async (enabled: boolean) => {
		setMessage(undefined);
		if (enabled && !path().trim()) {
			setMessage(
				"Choose or enter an absolute folder path before enabling this virtual folder.",
			);
			return;
		}
		setBusy(true);
		try {
			if (enabled) {
				await setVirtualFolderPath(props.definition.kind, path().trim());
			}
			await setVirtualFolderEnabled(props.definition.kind, enabled);
			setPathChanged(false);
			await props.refresh();
		} catch (reason) {
			setMessage(String(reason));
		} finally {
			setBusy(false);
		}
	};
	const savePath = async () => {
		if (!path().trim() || props.folder?.status.enabled) {
			return;
		}
		setBusy(true);
		setMessage(undefined);
		try {
			await setVirtualFolderPath(props.definition.kind, path().trim());
			setPathChanged(false);
			await props.refresh();
		} catch (reason) {
			setMessage(String(reason));
		} finally {
			setBusy(false);
		}
	};
	const status = () => props.folder?.status;

	return (
		<article class="mount-card" classList={{ active: Boolean(status()?.enabled) }}>
			<div class="card-heading">
				<div class="card-title">
					<h3>{props.definition.name}</h3>
					<Show when={status()?.enabled}>
						<span aria-label="Enabled" class="enabled-indicator" role="img" />
					</Show>
				</div>
				<Show when={busy()}>
					<LoaderCircle aria-label="Applying change" class="busy" />
				</Show>
				<div class="card-actions">
					<button
						aria-hidden={!status()?.enabled}
						class="open-button"
						classList={{ hidden: !status()?.enabled }}
						disabled={!status()?.enabled || !status()?.mount_path || busy()}
						onClick={() => void openVirtualFolder(props.definition.kind)}
						tabIndex={status()?.enabled ? 0 : -1}
					>
						Open folder <ExternalLink size={14} />
					</button>
					<button
						aria-checked={Boolean(status()?.enabled)}
						aria-label={`Enable ${props.definition.name}`}
						class="switch"
						disabled={props.disabled || busy()}
						onClick={() => void enable(!status()?.enabled)}
						role="switch"
					>
						<span />
					</button>
				</div>
			</div>
			<div class="path-row">
				<input
					aria-label={`${props.definition.name} mount location`}
					disabled={busy() || Boolean(status()?.enabled)}
					onBlur={() => void savePath()}
					onInput={(event) => {
						setPath(event.currentTarget.value);
						setPathChanged(true);
					}}
					placeholder={
						props.homeDir
							? `${props.homeDir}/backbeat-vf/${props.definition.placeholder}`
							: ""
					}
					value={path()}
				/>
				<button
					disabled={busy() || Boolean(status()?.enabled)}
					onClick={() => void choosePath()}
				>
					Set
				</button>
			</div>
			<Show when={message() || status()?.error}>
				{(detail) => <p class="card-error">{detail()}</p>}
			</Show>
		</article>
	);
}
