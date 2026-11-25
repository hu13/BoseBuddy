import { invoke } from "@tauri-apps/api/core";
import { setHostPlatform } from "./platform";
import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import "./styles.css";

async function start() {
  try {
    setHostPlatform(await invoke<string>("host_platform"));
  } catch {
    setHostPlatform("unknown");
  }
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
}
void start();
