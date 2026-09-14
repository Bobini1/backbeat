import { createSignal, onCleanup } from "solid-js";

export function useConfirmDestructive(resetMs?: null | number) {
	const [confirming, setConfirming] = createSignal(false);
	let confirmTimer: ReturnType<typeof setTimeout> | undefined;

	onCleanup(() => {
		if (confirmTimer) {
			clearTimeout(confirmTimer);
		}
	});

	function clearTimer() {
		if (confirmTimer) {
			clearTimeout(confirmTimer);
			confirmTimer = undefined;
		}
	}

	function arm() {
		setConfirming(true);
		clearTimer();
		if (resetMs && resetMs > 0) {
			confirmTimer = setTimeout(() => setConfirming(false), resetMs);
		}
	}

	function cancel() {
		clearTimer();
		setConfirming(false);
	}

	return { confirming, arm, cancel };
}
