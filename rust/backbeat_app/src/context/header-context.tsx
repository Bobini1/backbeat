import { type Accessor, createContext, createSignal, type JSX, useContext } from "solid-js";

/**
 * The page header is part of the shell chrome, not the scrolling route. Each
 * route declares its title/subtitle/actions by rendering `<PageHeader .../>`,
 * which publishes into this context; the shell renders the actual header band
 * from the signal. This lets the header sit above the ambient backing layer and
 * stay fixed while route content scrolls beneath it, and lets detail routes
 * publish dynamic titles (a fetched chart's name) and route-scoped actions
 * (Refresh, Open containing folder) into the same chrome.
 */
export interface HeaderState {
	actions?: JSX.Element;
	subtitle?: string;
	title: string;
}

const HeaderContext = createContext<{
	header: Accessor<HeaderState>;
	setHeader: (next: HeaderState) => void;
}>();

export function HeaderProvider(props: { children: JSX.Element }) {
	const [header, setHeader] = createSignal<HeaderState>({ title: "" });
	return (
		<HeaderContext.Provider value={{ header, setHeader }}>
			{props.children}
		</HeaderContext.Provider>
	);
}

export function useHeader() {
	const ctx = useContext(HeaderContext);
	if (!ctx) {
		throw new Error("useHeader must be used within a HeaderProvider");
	}
	return ctx;
}
