import { createRenderEffect, type JSX } from "solid-js";

import { useHeader } from "../context/header-context";

/**
 * Declares the current route's page header into the shell. Renders nothing --
 * the shell reads the published state and draws the header band as part of the
 * chrome, above the ambient backing layer. `createRenderEffect` runs
 * synchronously during render so the shell header updates in the same paint as
 * the route change (no stale-title flash).
 */
export function PageHeader(props: { actions?: JSX.Element; subtitle?: string; title: string }) {
	const { setHeader } = useHeader();
	createRenderEffect(() => {
		setHeader({ title: props.title, subtitle: props.subtitle, actions: props.actions });
	});
	return null;
}
