import { render } from "preact";
import "./index.css";
import { Provider } from "react-redux";
import { store } from "./store.ts";

import { App } from "./app.tsx";

render(<App />, document.getElementById("app")!);
