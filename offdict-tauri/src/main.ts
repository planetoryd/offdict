import "../node_modules/spectre.css/src/spectre.scss";
import App from "./App.svelte";
import { appWindow } from "@tauri-apps/api/window";

const app = new App({
  target: document.getElementById("app"),
});

document.addEventListener("keydown", (e) => {
  console.log(e.key);
  if (e.ctrlKey || e.altKey || e.shiftKey) return;
  if (e.key === "Escape") {
    app.$set({ dropdown: false }); // TODO: hide dropdown when dropdown is shown, hide window when dropdown is disabled
    appWindow.hide();
    return;
  }
  if (
    e.key === "Enter" ||
    e.key === "Control" ||
    ["Super", "PageDown", "PageUp"].includes(e.key)
  )
    return;
  // document.querySelector("input.form-input").focus();
});
window.viewlist = new Map();

export default app;
