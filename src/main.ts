import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

// En producción, sin el menú contextual del navegador (Recargar, Inspeccionar…)
// salvo en campos de texto, donde sirve para copiar y pegar.
if (import.meta.env.PROD) {
  document.addEventListener("contextmenu", (e) => {
    const t = e.target as HTMLElement;
    if (!t.closest("input, textarea")) e.preventDefault();
  });
}

async function start() {
  // En un navegador normal (fuera de Tauri) y en desarrollo, backend simulado.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installDevMock } = await import("./lib/dev-mock");
    installDevMock();
  }
  mount(App, { target: document.getElementById("app")! });
}

void start();
