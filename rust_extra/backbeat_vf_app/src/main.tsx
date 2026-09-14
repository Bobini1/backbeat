import { render } from "solid-js/web";
import "@fontsource-variable/hanken-grotesk";
import "@fontsource-variable/jetbrains-mono";

import "./styles.css";
import { App } from "./App";

render(() => <App />, document.getElementById("root")!);
