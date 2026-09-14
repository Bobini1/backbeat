import { A, Route, Router } from "@solidjs/router";
import { onMount } from "solid-js";

import { PageHeader } from "./components/PageHeader";
import Shell from "./components/window/Shell";
import { markStartup } from "./lib/startup";
import AssetDetail from "./routes/AssetDetail";
import ChartDetail from "./routes/ChartDetail";
import Charts from "./routes/Charts";
import CourseDetail from "./routes/CourseDetail";
import Courses from "./routes/Courses";
import CourseSettings from "./routes/CourseSettings";
import DiffTableDetail from "./routes/DiffTableDetail";
import { DiffTableLevel } from "./routes/DiffTableLevel";
import DiffTables from "./routes/DiffTables";
import DiffTableSettings from "./routes/DiffTableSettings";
import Downloads from "./routes/Downloads";
import Home from "./routes/Home";
import ManualImport from "./routes/ManualImport";
import PackDetail from "./routes/PackDetail";
import Packs from "./routes/Packs";
import PackSettings from "./routes/PackSettings";
import Servers from "./routes/Servers";
import Settings from "./routes/Settings";

export default function App() {
	onMount(() => {
		markStartup("app mounted");
	});

	return (
		<Router root={Shell}>
			<Route component={Home} path="/" />
			<Route component={Charts} path="/bundles" />
			<Route component={ChartDetail} path="/bundles/:bundleId" />
			<Route component={AssetDetail} path="/assets/:assetId" />
			<Route component={Packs} path="/packs" />
			<Route component={PackSettings} path="/packs/:url/settings" />
			<Route component={PackDetail} path="/packs/:url" />
			<Route component={Courses} path="/courses" />
			<Route component={CourseSettings} path="/courses/:url/settings" />
			<Route component={CourseDetail} path="/courses/:url" />
			<Route component={DiffTables} path="/tables" />
			<Route component={DiffTableSettings} path="/tables/:url/settings" />
			<Route component={DiffTableDetail} path="/tables/:url" />
			<Route component={DiffTableLevel} path="/tables/:url/:level" />
			<Route component={Servers} path="/servers" />
			<Route component={ManualImport} path="/manual-import" />
			<Route component={Downloads} path="/downloads" />
			<Route component={Settings} path="/settings" />
			<Route component={NotFound} path="*404" />
		</Router>
	);
}

function NotFound() {
	return (
		<>
			<PageHeader subtitle="That route doesn't exist." title="Page not found" />
			<p class="page-subtitle">
				<A class="section-link" href="/">
					Go home &rarr;
				</A>
			</p>
		</>
	);
}
