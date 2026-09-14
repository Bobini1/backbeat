const codeTabGroups = document.querySelectorAll<HTMLElement>("[data-code-tabs]");

for (const group of codeTabGroups) {
	const tabs = Array.from(group.querySelectorAll<HTMLButtonElement>('[role="tab"]'));
	const panels = Array.from(group.querySelectorAll<HTMLElement>('[role="tabpanel"]'));

	function activate(index: number, moveFocus: boolean) {
		for (const [tabIndex, tab] of tabs.entries()) {
			const selected = tabIndex === index;
			tab.setAttribute("aria-selected", String(selected));
			tab.tabIndex = selected ? 0 : -1;
			panels[tabIndex]?.toggleAttribute("hidden", !selected);
		}

		if (moveFocus) {
			tabs[index]?.focus();
		}
	}

	const selectedIndex = tabs.findIndex((tab) => tab.getAttribute("aria-selected") === "true");
	activate(selectedIndex >= 0 ? selectedIndex : 0, false);

	for (const [index, tab] of tabs.entries()) {
		tab.addEventListener("click", () => activate(index, false));
		tab.addEventListener("keydown", (event) => {
			let nextIndex: number | undefined;
			switch (event.key) {
				case "ArrowLeft":
					nextIndex = (index - 1 + tabs.length) % tabs.length;
					break;
				case "ArrowRight":
					nextIndex = (index + 1) % tabs.length;
					break;
				case "End":
					nextIndex = tabs.length - 1;
					break;
				case "Home":
					nextIndex = 0;
					break;
			}

			if (nextIndex !== undefined) {
				event.preventDefault();
				activate(nextIndex, true);
			}
		});
	}
}
