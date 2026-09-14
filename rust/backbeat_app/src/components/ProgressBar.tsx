import styles from "./ProgressBar.module.css";

export function progressPercentage(current: number, total: number): number {
	return total === 0 ? 100 : Math.round((current / total) * 100);
}

export function ProgressBar(props: { label: string; max: number; value: number }) {
	return (
		<div class={styles.container}>
			<progress
				aria-label={props.label}
				class={styles.progress}
				max={props.max}
				value={props.value}
			/>
		</div>
	);
}
