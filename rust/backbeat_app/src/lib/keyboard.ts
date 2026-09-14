function isEditableElement(target: EventTarget | null): boolean {
	if (!(target instanceof HTMLElement)) {
		return false;
	}
	const tag = target.tagName;
	if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") {
		return true;
	}
	return target.isContentEditable;
}

export function installKeyboardGuards() {
	if (typeof window === "undefined") {
		return;
	}

	window.addEventListener(
		"keydown",
		(event) => {
			if (event.key !== "Backspace" && event.key !== " ") {
				return;
			}
			if (isEditableElement(event.target)) {
				return;
			}
			// Space on a focused button/link should still activate it.
			if (event.key === " " && event.target instanceof HTMLElement) {
				const tag = event.target.tagName;
				if (
					tag === "BUTTON" ||
					tag === "A" ||
					event.target.getAttribute("role") === "button"
				) {
					return;
				}
			}
			event.preventDefault();
		},
		true,
	);
}
