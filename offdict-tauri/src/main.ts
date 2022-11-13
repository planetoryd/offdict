import "../node_modules/spectre.css/src/spectre.scss";
import App from "./App.svelte";

const app = new App({
  target: document.getElementById("app"),
});

document.addEventListener("keydown", (e) => {
  console.log(e.key);
  if (
    e.key === "Enter" ||
    e.key === "Control" ||
    ["Escape", "Super"].includes(e.key)
  )
    return;
  document.querySelector("input.form-input").focus();
});

export default app;
