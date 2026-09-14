import { A, useLocation } from "@solidjs/router";
import { type Component, createMemo, type JSX, Show } from "solid-js";

import styles from "./NavMenu.module.css";
import NiceLabel from "./NiceLabel";

export interface NavMenuProps {
	children: (Section: Component<MenuSectionProps>, Item: Component<MenuItemProps>) => JSX.Element;
	name: string;
}

export interface MenuSectionProps {
	children?: JSX.Element;
	title?: string;
}

export interface MenuItemProps {
	/** A component that renders an SVG element to use as an icon */
	icon?: Component<JSX.SvgSVGAttributes<SVGSVGElement>>;
	label: string;
	path: string;
}

function MenuSection(props: MenuSectionProps) {
	const idString = () => props.title && props.title.replaceAll(" ", "-").toLowerCase();
	const headingId = () => idString() && `nav-section-heading-${idString()}`;
	const sectionId = () => idString() && `nav-section-${idString()}`;

	return (
		<div class={styles.section} id={sectionId()}>
			<Show when={props.title}>
				<NiceLabel as="h2" class={styles.sectionHeading} id={headingId()}>
					{props.title}
				</NiceLabel>
			</Show>
			<ul aria-labelledby={headingId()} class={styles.itemList}>
				{props.children}
			</ul>
		</div>
	);
}

function MenuItem(props: MenuItemProps) {
	const location = useLocation();

	const nested = createMemo(() => {
		if (props.path === "/") {
			return false;
		}

		return location.pathname.startsWith(props.path);
	});

	return (
		<li>
			<A
				class={styles.item}
				data-nested={nested()}
				end={props.path === "/"}
				href={props.path}
			>
				<div aria-hidden class={styles.itemIndicator} />
				<Show when={props.icon}>
					{(icon) => {
						const Component = icon();
						return (
							<span aria-hidden class={styles.itemIcon}>
								<Component height={20} stroke-width={2} width={20} />
							</span>
						);
					}}
				</Show>
				<span class={styles.itemLabel}>{props.label}</span>
			</A>
		</li>
	);
}

export default function NavMenu(props: NavMenuProps) {
	return (
		<nav
			aria-label={props.name}
			class={styles.nav}
			id={props.name.replaceAll(" ", "-").toLowerCase()}
		>
			{props.children(MenuSection, MenuItem)}
		</nav>
	);
}
