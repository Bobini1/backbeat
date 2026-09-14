import { type Component, type JSXElement, type ParentProps, Show } from "solid-js";

import NiceLabel from "./primitives/NiceLabel";
import styles from "./StatView.module.css";

export interface StatViewProps {
	children: (stat: Component<StatProps>) => JSXElement;
}

export interface StatProps extends ParentProps {
	detail?: string;
	label: string;
	variant?: "base" | "danger" | "success" | "warning";
}

function Stat(props: StatProps) {
	return (
		<div class={styles.stat} data-variant={props.variant}>
			{/* the direction of these components are visually reversed */}
			<div class={styles.statFooter}>
				<NiceLabel>{props.label}</NiceLabel>
				<Show when={props.detail}>
					<p class={styles.statDetail}>{props.detail}</p>
				</Show>
			</div>
			<Show
				fallback={props.children}
				when={["boolean", "number", "string"].includes(typeof props.children)}
			>
				<p class={styles.statValue}>{props.children}</p>
			</Show>
		</div>
	);
}

export default function StatView(props: StatViewProps) {
	return <div class={styles.statView}>{props.children(Stat)}</div>;
}
