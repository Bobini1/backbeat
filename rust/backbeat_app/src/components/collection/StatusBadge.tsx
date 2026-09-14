import { createMemo } from "solid-js";

import type { CollectionDocumentStatus } from "../../lib/tauri";

import styles from "./StatusBadge.module.css";

export default function StatusBadge(props: {
	checkingLabel?: string;
	status: CollectionDocumentStatus | undefined;
}) {
	const label = createMemo(() => {
		if (!props.status && props.checkingLabel) {
			return props.checkingLabel;
		}

		if (props.status?.error) {
			return "Unavailable";
		}

		switch (props.status?.needs_update) {
			case false:
				return "Up to date";
			case true:
				return "Update available";
			default:
				return "Checking…";
		}
	});

	return (
		<span
			class={styles.badge}
			data-available={props.status ? props.status.error === null : undefined}
			data-status={
				props.status?.needs_update
					? "update"
					: props.status?.error
						? "unavailable"
						: props.status
							? "current"
							: "checking"
			}
		>
			{label()}
		</span>
	);
}
