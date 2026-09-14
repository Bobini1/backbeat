import {
	Download,
	House,
	Import,
	Music,
	Package,
	Route,
	Scale,
	Server,
	Settings,
} from "lucide-solid";
import { type Component, createResource, type JSX, Show } from "solid-js";

import { downloadSummary } from "../../lib/downloadActivity";
import { buildInfo } from "../../lib/tauri";
import NavMenu from "../primitives/NavMenu";
import NotificationBadge from "../primitives/NotificationBadge";
import styles from "./Sidebar.module.css";

export default function Sidebar() {
	const downloadBadgeCount = () => downloadSummary()?.badgeCount ?? null;
	const downloadFailed = () => downloadSummary()?.hasFailureAttention ?? false;

	const DownloadsIcon: Component<JSX.SvgSVGAttributes<SVGSVGElement>> = (props) => (
		<NotificationBadge
			badgeCount={downloadBadgeCount() ?? 0}
			badgeShow={downloadBadgeCount() !== null || downloadFailed()}
			badgeVariant={downloadFailed() ? "danger" : undefined}
		>
			<Download {...props} />
		</NotificationBadge>
	);

	return (
		<div class={styles.sidebar}>
			<div class={styles.brand}>
				<span class={styles.brandName}>Backbeat</span>
			</div>
			<NavMenu name="Main Menu">
				{(Section, Item) => (
					<>
						<Section>
							<Item icon={House} label="Home" path="/" />
						</Section>
						<Section title="Your Store">
							<Item icon={Music} label="Charts" path="/bundles" />
							<Item icon={Package} label="Packs" path="/packs" />
							<Item icon={Route} label="Courses" path="/courses" />
							<Item icon={Scale} label="Difficulty Tables" path="/tables" />
						</Section>
						<Section title="Servers">
							<Item icon={Server} label="Data Servers" path="/servers" />
						</Section>
						<div class={styles.footer}>
							<Section>
								<Item icon={Import} label="Manual Import" path="/manual-import" />
								<Item icon={DownloadsIcon} label="Downloads" path="/downloads" />
								<Item icon={Settings} label="Settings" path="/settings" />
							</Section>
							<SidebarBuildInfo />
						</div>
					</>
				)}
			</NavMenu>
		</div>
	);
}

function SidebarBuildInfo() {
	const [info] = createResource(buildInfo);

	return (
		<Show when={info()}>
			{(data) => (
				<p class={styles.buildInfo}>
					v{data().version}
					<Show when={data().git_sha_short}>
						<span>{data().git_sha_short}</span>
					</Show>
				</p>
			)}
		</Show>
	);
}
