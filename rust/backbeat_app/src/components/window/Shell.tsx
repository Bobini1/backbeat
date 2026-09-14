import type { RouteSectionProps } from "@solidjs/router";

import { onMount, Show } from "solid-js";

import { HeaderProvider, useHeader } from "../../context/header-context";
import { startDownloadActivity } from "../../lib/downloadActivity";
import DownloadToast from "../DownloadToast";
import GlobalDownloadStatus from "../GlobalDownloadStatus";
import styles from "./Shell.module.css";
import Sidebar from "./Sidebar";

function ShellHeader() {
	const { header } = useHeader();
	return (
		<header class={styles.header}>
			<div>
				<h1 class={styles.headerTitle}>{header().title}</h1>
				<Show when={header().subtitle}>
					<p class={styles.headerSubtitle}>{header().subtitle}</p>
				</Show>
			</div>
			<Show when={header().actions}>
				<div class={styles.headerActions}>{header().actions}</div>
			</Show>
		</header>
	);
}

export default function Shell(props: RouteSectionProps) {
	onMount(() => {
		startDownloadActivity();
	});

	return (
		<HeaderProvider>
			<div class={styles.shell}>
				<Sidebar />
				<div class={styles.content}>
					<ShellHeader />
					<main class={styles.main}>{props.children}</main>
				</div>
				<div class={styles.toastOverlay}>
					<DownloadToast />
					<GlobalDownloadStatus />
				</div>
			</div>
		</HeaderProvider>
	);
}
