import { createSignal } from "solid-js";

/**
 * Color theme preference: "system" follows the OS, "light"/"dark" force it.
 * The pref is persisted to localStorage and the effective theme is written to
 * `<html data-theme="light|dark">`, which the stylesheet keys the light token
 * block off of. An inline pre-paint script in index.html sets the attribute
 * before the app mounts to avoid a flash; this module owns the reactive state
 * and keeps the attribute in sync, including when the OS preference flips while
 * in system mode.
 */
export type ThemePref = "dark" | "light" | "system";

const STORAGE_KEY = "backbeat.theme";
const VALID: ThemePref[] = ["system", "light", "dark"];

function readPref(): ThemePref {
	try {
		const v = localStorage.getItem(STORAGE_KEY);
		if (v && VALID.includes(v as ThemePref)) {
			return v as ThemePref;
		}
	} catch {
		/* localStorage unavailable -- fall back to system */
	}
	return "system";
}

function systemEffective(): "dark" | "light" {
	return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function effective(pref: ThemePref): "dark" | "light" {
	return pref === "system" ? systemEffective() : pref;
}

function apply(pref: ThemePref) {
	document.documentElement.dataset.theme = effective(pref);
}

const [themePref, setThemePrefSignal] = createSignal<ThemePref>(readPref());
export { themePref };

export function setThemePref(pref: ThemePref) {
	try {
		localStorage.setItem(STORAGE_KEY, pref);
	} catch {
		/* ignore storage failures */
	}
	setThemePrefSignal(pref);
	apply(pref);
}

/** Apply the stored pref and react to OS changes while in system mode. Call once
 *  at startup (after the pre-paint script in index.html has already set the
 *  attribute to avoid a flash). */
export function initTheme() {
	apply(themePref());
	const mq = window.matchMedia?.("(prefers-color-scheme: dark)");
	mq?.addEventListener("change", () => {
		if (themePref() === "system") {
			apply("system");
		}
	});
}
