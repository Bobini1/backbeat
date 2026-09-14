import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

// Fixed dev server port so it matches `build.devUrl` in `src-tauri/tauri.conf.json`.
const TAURI_DEV_PORT = 1420;

export default defineConfig({
	plugins: [solid()],
	clearScreen: false,
	server: {
		port: TAURI_DEV_PORT,
		strictPort: true,
		watch: {
			// Don't reload the frontend for changes to the Rust side.
			ignored: ["**/src-tauri/**"],
		},
	},
});
