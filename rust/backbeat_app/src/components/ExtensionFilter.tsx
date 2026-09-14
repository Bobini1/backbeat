import { createEffect, createSignal, createUniqueId, For, onCleanup, Show } from "solid-js";

import "./GamemodeFilter.css";
import RandomColourBadge from "./RandomColourBadge";

export function ExtensionFilter(props: {
	onChange: (value: string) => void;
	options: string[];
	value: string;
}) {
	const [open, setOpen] = createSignal(false);
	const [activeIndex, setActiveIndex] = createSignal(0);
	const listboxId = createUniqueId();
	let root: HTMLDivElement | undefined;
	let menu: HTMLDivElement | undefined;
	let trigger: HTMLButtonElement | undefined;

	const items = () => ["", ...props.options];
	const optionId = (index: number) => `${listboxId}-option-${index}`;

	function close(restoreFocus = false) {
		setOpen(false);
		if (restoreFocus) {
			queueMicrotask(() => trigger?.focus());
		}
	}

	function select(extension: string) {
		props.onChange(extension);
		close(true);
	}

	function openMenu() {
		const idx = items().indexOf(props.value);
		setActiveIndex(idx >= 0 ? idx : 0);
		setOpen(true);
	}

	createEffect(() => {
		if (!open()) {
			return;
		}

		const onPointerDown = (event: PointerEvent) => {
			if (root && !root.contains(event.target as Node)) {
				close();
			}
		};
		const onKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				event.preventDefault();
				close(true);
			}
		};

		document.addEventListener("pointerdown", onPointerDown);
		document.addEventListener("keydown", onKeyDown);
		queueMicrotask(() => menu?.focus());

		onCleanup(() => {
			document.removeEventListener("pointerdown", onPointerDown);
			document.removeEventListener("keydown", onKeyDown);
		});
	});

	createEffect(() => {
		if (!open()) {
			return;
		}
		const option = menu?.querySelector<HTMLElement>(`[data-index="${activeIndex()}"]`);
		option?.scrollIntoView({ block: "nearest" });
	});

	function onMenuKeyDown(event: KeyboardEvent) {
		const list = items();
		switch (event.key) {
			case " ":
			case "Enter":
				event.preventDefault();
				select(list[activeIndex()] ?? "");
				break;
			case "ArrowDown":
				event.preventDefault();
				setActiveIndex((i) => Math.min(i + 1, list.length - 1));
				break;
			case "ArrowUp":
				event.preventDefault();
				setActiveIndex((i) => Math.max(i - 1, 0));
				break;
			case "End":
				event.preventDefault();
				setActiveIndex(list.length - 1);
				break;
			case "Home":
				event.preventDefault();
				setActiveIndex(0);
				break;
			case "Tab":
				close();
				break;
		}
	}

	return (
		<div class="gamemode-filter" ref={root}>
			<button
				aria-controls={open() ? listboxId : undefined}
				aria-expanded={open()}
				aria-haspopup="listbox"
				aria-label="Filter by file extension"
				class="gamemode-filter-trigger"
				onClick={() => {
					if (open()) {
						close();
					} else {
						openMenu();
					}
				}}
				onKeyDown={(event) => {
					if (event.key === "ArrowDown") {
						event.preventDefault();
						openMenu();
					}
				}}
				ref={trigger}
				type="button"
			>
				<Show
					fallback={
						<span class="gamemode-filter-placeholder">
							<RandomColourBadge content="any" />
						</span>
					}
					when={props.value}
				>
					<RandomColourBadge content={`.${props.value}`} />
				</Show>
				<span aria-hidden="true" class="gamemode-filter-chevron">
					▾
				</span>
			</button>

			<Show when={open()}>
				<div
					aria-activedescendant={optionId(activeIndex())}
					aria-label="File extensions"
					class="gamemode-filter-menu"
					id={listboxId}
					onKeyDown={onMenuKeyDown}
					ref={menu}
					role="listbox"
					tabindex={-1}
				>
					<div class="gamemode-filter-grid">
						<button
							aria-selected={props.value === ""}
							class="gamemode-filter-option"
							data-active={activeIndex() === 0 ? "true" : undefined}
							data-index={0}
							data-selected={props.value === "" ? "true" : undefined}
							id={optionId(0)}
							onClick={() => select("")}
							onMouseEnter={() => setActiveIndex(0)}
							role="option"
							tabindex={-1}
							type="button"
						>
							<RandomColourBadge content="any" />
						</button>
						<For each={props.options}>
							{(extension, index) => {
								const itemIndex = () => index() + 1;
								return (
									<button
										aria-selected={props.value === extension}
										class="gamemode-filter-option"
										data-active={
											activeIndex() === itemIndex() ? "true" : undefined
										}
										data-index={itemIndex()}
										data-selected={
											props.value === extension ? "true" : undefined
										}
										id={optionId(itemIndex())}
										onClick={() => select(extension)}
										onMouseEnter={() => setActiveIndex(itemIndex())}
										role="option"
										tabindex={-1}
										type="button"
									>
										<RandomColourBadge content={`.${extension}`} />
									</button>
								);
							}}
						</For>
					</div>
				</div>
			</Show>
		</div>
	);
}
