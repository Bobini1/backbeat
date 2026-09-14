import { createMemo, Show } from "solid-js";

import StatusDot from "../StatusDot";
import styles from "./CoverageCount.module.css";

export default function CoverageCount(props: {
	installed: number;
	statusDot?: boolean;
	total: number;
	unit: string;
}) {
	const missing = () => Math.max(0, props.total - props.installed);
	const label = createMemo(() => {
		const nMissing = missing();
		const unit = nMissing === 1 ? props.unit : `${props.unit}s`;
		return nMissing ? `${nMissing} ${unit} not installed` : `All ${unit} installed`;
	});

	return (
		<div class={styles.count} data-status={missing() ? "missing" : "installed"}>
			<Show when={props.statusDot}>
				<StatusDot status={missing() ? "danger" : "success"} />
			</Show>
			{label()}
		</div>
	);
}

export function BundleStatus(props: { installed: boolean }) {
	return (
		<div class={styles.count} data-status={props.installed ? "installed" : "missing"}>
			<StatusDot status={props.installed ? "success" : "danger"} />
			{props.installed ? "Installed" : "Missing"}
		</div>
	);
}
