import { createSignal, Show } from "solid-js";

import styles from "./HashField.module.css";
import Button from "./primitives/Button";
import NiceLabel from "./primitives/NiceLabel";

const COPIED_RESET_MS = 1500;

export interface HashFieldProps {
	label: string;
	/**
	 * Override the copy button with a button that should reveal the file
	 * in the system file browser
	 */
	onOpen?: () => void;
	value: string;
}

export function HashField(props: HashFieldProps) {
	const [copied, setCopied] = createSignal(false);

	async function copy() {
		try {
			await navigator.clipboard.writeText(props.value);
			setCopied(true);
			setTimeout(() => setCopied(false), COPIED_RESET_MS);
		} catch {
			// Clipboard access can fail (e.g. no permission); nothing useful to
			// do beyond leaving the button in its normal state.
		}
	}

	return (
		<div class={styles.field}>
			<NiceLabel>{props.label}</NiceLabel>
			<div class={styles.row}>
				<span class={styles.value}>{props.value}</span>
				<Show
					fallback={
						<Button onClick={copy} type="button" variant="base">
							{copied() ? "Copied" : "Copy"}
						</Button>
					}
					when={props.onOpen}
				>
					<Button onClick={() => props.onOpen?.()} type="button" variant="base">
						Open in File Browser
					</Button>
				</Show>
			</div>
		</div>
	);
}
