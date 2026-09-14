/* @refresh reload */
import { render } from "solid-js/web";

import App from "./App";
import { installKeyboardGuards } from "./lib/keyboard";
import { installFrontendLogBridge } from "./lib/log-bridge";
import { markStartup, markStartupAt, reportStartupResources } from "./lib/startup";
import { initTheme } from "./lib/theme";
import "./styles.css";

declare global {
	interface Window {
		__backbeatStartupMarks?: Array<[string, number]>;
	}
}

for (const [label, elapsedMs] of window.__backbeatStartupMarks ?? []) {
	markStartupAt(label, elapsedMs);
}
markStartup("frontend module loaded");
installFrontendLogBridge();
installKeyboardGuards();
initTheme();
markStartup("theme initialized");

const root = document.getElementById("root");

if (!root) {
	throw new Error("root element not found");
}

markStartup("render start");
render(() => <App />, root);
markStartup("render returned");
reportStartupResources();
