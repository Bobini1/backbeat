/**
 * Forward frontend log lines into the Rust `tracing` log file (see
 * `frontend_log` / `logging.rs`), so a single rotating log captures both Rust
 * and JS-side errors for user bug reports.
 *
 * Wraps `console.error` / `console.warn` / `console.log` (still calling the
 * originals so devtools output is unchanged) and listens for the global
 * `error` and `unhandledrejection` events. Forwards are fire-and-forget and
 * truncated to keep the log file bounded; failures to invoke are swallowed so
 * logging never throws into application code.
 */

import { frontendLog } from "./tauri";

const MAX_LEN = 2000;

function truncate(s: string): string {
	return s.length > MAX_LEN ? `${s.slice(0, MAX_LEN)}…[truncated]` : s;
}

function stringify(args: unknown[]): string {
	return args.map((a) => (typeof a === "string" ? a : safeStringify(a))).join(" ");
}

function safeStringify(value: unknown): string {
	if (value instanceof Error) {
		return value.stack ?? `${value.name}: ${value.message}`;
	}
	try {
		return JSON.stringify(value);
	} catch {
		return String(value);
	}
}

export function installFrontendLogBridge() {
	if (typeof window === "undefined") {
		return;
	}

	wrapConsole("error", "error");
	wrapConsole("warn", "warn");
	wrapConsole("log", "info");

	window.addEventListener("error", (ev) => {
		const loc = ev.filename ? `${ev.filename}:${ev.lineno}:${ev.colno}` : undefined;
		const msg = ev.error?.stack ? `${ev.error.stack}` : ev.message;
		void frontendLog("error", truncate(msg), loc).catch(() => {});
	});

	window.addEventListener("unhandledrejection", (ev) => {
		const reason = ev.reason;
		const msg = reason?.stack ? reason.stack : safeStringify(reason);
		void frontendLog("error", truncate(`Unhandled rejection: ${msg}`)).catch(() => {});
	});
}

function wrapConsole(method: "error" | "log" | "warn", level: "error" | "info" | "warn") {
	const original = console[method].bind(console);
	console[method] = (...args: unknown[]) => {
		original(...args);
		void frontendLog(level, truncate(stringify(args))).catch(() => {});
	};
}
