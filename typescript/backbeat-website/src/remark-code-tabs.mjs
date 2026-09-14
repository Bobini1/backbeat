const TAB_HEADING = /^===\s+(?:"([^"]+)"|'([^']+)'|“([^”]+)”|‘([^’]+)’)\s*$/;
const FENCE = /^(?<fence>`{3,}|~{3,})(?<info>.*)$/;

function tabLabel(node) {
	if (
		node?.type !== "paragraph" ||
		node.children?.length !== 1 ||
		node.children[0].type !== "text"
	) {
		return null;
	}

	const match = node.children[0].value.match(TAB_HEADING);
	return match?.[1] ?? match?.[2] ?? match?.[3] ?? match?.[4] ?? null;
}

function parseNestedCode(node) {
	if (node?.type !== "code" || node.lang !== null || node.meta !== null) {
		return null;
	}

	const lines = node.value.split("\n");
	const opening = lines[0].match(FENCE);
	const closing = lines.at(-1);
	if (
		!opening ||
		!closing ||
		!new RegExp(`^${opening.groups.fence[0]}{${opening.groups.fence.length},}\\s*$`).test(
			closing,
		)
	) {
		return null;
	}

	const info = opening.groups.info.trim();
	const [language = "", ...meta] = info.split(/\s+/);
	return {
		code: lines.slice(1, -1).join("\n"),
		language,
		meta: meta.join(" "),
	};
}

function element(tagName, properties, children) {
	return { children, properties, tagName, type: "element" };
}

export function codeTabsHandler(_state, node) {
	const { groupIndex, tabs } = node.data;
	const groupId = `code-tabs-${groupIndex}`;
	const tabButtons = tabs.map(({ label }, index) => {
		const tabId = `${groupId}-tab-${index}`;
		const panelId = `${groupId}-panel-${index}`;
		return element(
			"button",
			{
				ariaControls: panelId,
				ariaSelected: index === 0,
				className: ["code-tabs-tab"],
				id: tabId,
				role: "tab",
				tabIndex: index === 0 ? 0 : -1,
				type: "button",
			},
			[{ type: "text", value: label }],
		);
	});

	const panels = tabs.map(({ code, language, meta }, index) => {
		const tabId = `${groupId}-tab-${index}`;
		const panelId = `${groupId}-panel-${index}`;
		const codeProperties = language ? { className: [`language-${language}`] } : {};
		if (meta) codeProperties.metastring = meta;
		return element(
			"div",
			{
				ariaLabelledBy: tabId,
				className: ["code-tabs-panel"],
				hidden: index !== 0,
				id: panelId,
				role: "tabpanel",
				tabIndex: 0,
			},
			[
				element("pre", {}, [
					element("code", codeProperties, [{ type: "text", value: code }]),
				]),
			],
		);
	});

	return element("div", { className: ["code-tabs"], dataCodeTabs: "true" }, [
		element(
			"div",
			{ ariaLabel: "Code examples", className: ["code-tabs-list"], role: "tablist" },
			tabButtons,
		),
		...panels,
	]);
}

export default function remarkCodeTabs() {
	return (tree) => {
		let groupIndex = 0;
		const children = tree.children;

		for (let index = 0; index < children.length; ) {
			const firstLabel = tabLabel(children[index]);
			const firstCode = parseNestedCode(children[index + 1]);
			if (!firstLabel || !firstCode) {
				index += 1;
				continue;
			}

			const tabs = [{ label: firstLabel, ...firstCode }];
			let end = index + 2;
			while (end + 1 < children.length) {
				const label = tabLabel(children[end]);
				const code = parseNestedCode(children[end + 1]);
				if (!label || !code) break;
				tabs.push({ label, ...code });
				end += 2;
			}

			children.splice(index, end - index, {
				data: { groupIndex, tabs },
				type: "codeTabs",
			});
			groupIndex += 1;
			index += 1;
		}
	};
}
