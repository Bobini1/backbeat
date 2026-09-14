import { type Component, type JSXElement, type ParentProps } from "solid-js";

import styles from "./DetailsGrid.module.css";

export interface DetailsGridProps {
	children: (field: Component<DetailFieldProps>) => JSXElement;
}

export interface DetailFieldProps extends ParentProps {
	label: string;
}

function DetailField(props: DetailFieldProps) {
	return (
		<div class={styles.detailField}>
			<span class={styles.detailLabel}>{props.label}</span>
			<span class={styles.detailValue}>{props.children}</span>
		</div>
	);
}

export default function DetailsGrid(props: DetailsGridProps) {
	return <div class={styles.detailsGrid}>{props.children(DetailField)}</div>;
}
