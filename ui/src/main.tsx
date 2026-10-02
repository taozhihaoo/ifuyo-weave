import React from "react";
import ReactDOM from "react-dom/client";
import App from "./app/App";
import { registerBuiltinCommands } from "./commands/builtin";
import { applyTokens } from "./design/applyTokens";
import "./design/global.css";
import { bootstrapApp } from "./lib/bootstrap";

applyTokens();
registerBuiltinCommands();
void bootstrapApp();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
