import { mount } from "svelte";
import App from "./App.svelte";
import ReminderPopup from "./components/ReminderPopup.svelte";
import "./app.css";

// En producción, sin el menú contextual del navegador (Recargar, Inspeccionar…)
// salvo en campos de texto, donde sirve para copiar y pegar.
if (import.meta.env.PROD) {
  document.addEventListener("contextmenu", (e) => {
    const t = e.target as HTMLElement;
    if (!t.closest("input, textarea")) e.preventDefault();
  });
}

/**
 * Etiqueta de la ventana de Tauri, o null en un navegador normal. Se lee de
 * los metadatos que inyecta Tauri (lo mismo que hace `getCurrentWindow()`),
 * porque importar `@tauri-apps/api/window` solo para esto añade ~20 kB.
 */
function windowLabel(): string | null {
  type Internals = { metadata?: { currentWindow?: { label?: string } } };
  const internals = (window as { __TAURI_INTERNALS__?: Internals }).__TAURI_INTERNALS__;
  return internals?.metadata?.currentWindow?.label ?? null;
}

async function start() {
  // En un navegador normal (fuera de Tauri) y en desarrollo, backend simulado.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installDevMock } = await import("./lib/dev-mock");
    installDevMock();
  }
  // El mismo bundle sirve a las dos ventanas: la principal y el popup de
  // recordatorios (`reminder`, ver src-tauri/src/popup.rs). En el navegador,
  // el popup se ve con http://localhost:1420/#reminder.
  const isPopup = windowLabel() === "reminder" || location.hash === "#reminder";
  mount(isPopup ? ReminderPopup : App, { target: document.getElementById("app")! });
}

void start();
