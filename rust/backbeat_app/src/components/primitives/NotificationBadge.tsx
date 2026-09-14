import { children, type JSX, Show, splitProps } from "solid-js";

import styles from "./NotificationBadge.module.css";

export interface NotificationBadgeProps extends JSX.HTMLAttributes<HTMLSpanElement> {
	badgeCount?: null | number;
	badgeShow?: boolean;
	badgeVariant?: "base" | "danger" | "warning";
}

/**
 * Overlays a notification badge on top of an element, anchoring to the top-right
 */
export default function NotificationBadge(props: NotificationBadgeProps) {
	const resolved = children(() => props.children);

	const [badgeProps, elementProps] = splitProps(props, [
		"badgeCount",
		"badgeShow",
		"badgeVariant",
	]);

	const show = () => badgeProps.badgeShow;
	const count = () => badgeProps.badgeCount;
	const variant = () => badgeProps.badgeVariant;

	const value = () =>
		(count() ?? 0) > 99 ? "99+" : count() === 0 && variant() === "danger" ? "!" : count();

	return (
		<div class={styles.container}>
			{resolved()}
			<Show when={show() && (count() || variant() === "danger")}>
				<span
					{...elementProps}
					class={[styles.badge, props.class]
						.flat()
						.filter((x) => typeof x === "string")
						.join(" ")}
					data-variant={variant()}
				>
					{value()}
				</span>
			</Show>
		</div>
	);
}
