import "./app.css";
import App from "./App.svelte";
import { mount } from "svelte";

const app = mount(App, {
  target: document.getElementById("app")!,
});

// Signal that the app is mounted to hide loading screen
window.dispatchEvent(new Event("app-mounted"));

export default app;
