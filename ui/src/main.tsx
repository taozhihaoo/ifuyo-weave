import React from "react";
import ReactDOM from "react-dom/client";
import App from "./app/App";
import { registerBuiltinCommands } from "./commands/builtin";
import "./design/global.css";
import { applyTheme, bootstrapApp } from "./lib/bootstrap";

// 启动即按系统偏好应用主题：IPC 不可用或持久化配置尚未读到时，
// 深色用户不会先闪一帧浅色。配置加载后 bootstrapApp 再按保存值重设。
applyTheme("system");
registerBuiltinCommands();
void bootstrapApp();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
