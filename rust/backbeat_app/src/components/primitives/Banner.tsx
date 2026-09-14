import { type ParentProps, Show } from "solid-js";

import { type Variant } from "../../lib/style";
import styles from "./Banner.module.css";
import Button from "./Button";

export interface BannerProps {
	button?: BannerButton;
	id?: string;
	progress?: BannerProgress | false | null;
	variant?: Variant;
}

export interface BannerButton {
	disabled?: boolean;
	label: string;
	onClick: () => void;
}

export interface BannerProgress {
	current: number;
	total: number;
}

/**
 * A banner component with an optional progress bar and button.
 */
export default function Banner(props: ParentProps<BannerProps>) {
	return (
		<div class={styles.banner} data-variant={props.variant ?? "base"} id={props.id}>
			{props.children}
			<Show when={props.button}>
				{(button) => (
					<Button
						disabled={button().disabled}
						onClick={button().onClick}
						variant={props.variant ?? "base"}
					>
						{button().label}
					</Button>
				)}
			</Show>
			<Show when={props.progress}>
				{(progress) => (
					<progress
						class={styles.progress}
						max={progress().total}
						value={progress().current}
					/>
				)}
			</Show>
		</div>
	);
}
