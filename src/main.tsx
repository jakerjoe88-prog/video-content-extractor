import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./index.css";

// 全局错误捕获
window.onerror = function (message, source, lineno, colno, error) {
  console.error("Global error:", { message, source, lineno, colno, error });
};

window.onunhandledrejection = function (event) {
  console.error("Unhandled promise rejection:", event.reason);
};

// 立即渲染应用 - 窗口已默认可见，无需等待
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
