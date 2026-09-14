import { createSignal, Show } from "solid-js";

import { truncateHash } from "../lib/format";
import "./CopyableId.css";
import Button from "./primitives/Button";

const COPIED_RESET_MS = 1500;
const DEFAULT_DISPLAY_LEN = 10;

/** Truncated mono id that copies the full value on click. */
export function CopyableId(props: { class?: string; displayLen?: number; value: string }) {
	const [copied, setCopied] = createSignal(false);

	async function copy(event: MouseEvent) {
		event.preventDefault();
		event.stopPropagation();
		try {
			await navigator.clipboard.writeText(props.value);
			setCopied(true);
			setTimeout(() => setCopied(false), COPIED_RESET_MS);
		} catch {
			/* clipboard may be unavailable */
		}
	}

	return (
		<Button
			class={`copyable-id${props.class ? ` ${props.class}` : ""}`}
			onClick={copy}
			title={props.value}
			variant="surface"
		>
			<Show
				fallback={truncateHash(props.value, props.displayLen ?? DEFAULT_DISPLAY_LEN)}
				when={copied()}
			>
				Copied
			</Show>
		</Button>
	);
}
