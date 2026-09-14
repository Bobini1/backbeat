import type { ParentProps } from "solid-js";

import styles from "./StatusDot.module.css";

export interface StatusDotProps {
	status?: "base" | "danger" | "success" | "warning";
}

export default function StatusDot(props: StatusDotProps) {
	return <span aria-hidden="true" class={styles.dot} data-status={props.status} />;
}

/** Sure it doesn't hurt to have a thing that also renders the some text */
export function StatusLabel(props: ParentProps<StatusDotProps>) {
	return (
		<div class={styles.label} data-status={props.status}>
			<StatusDot status={props.status} />
			{props.children}
		</div>
	);
}
