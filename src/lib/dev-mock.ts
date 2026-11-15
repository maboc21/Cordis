// Backend simulado en memoria para trabajar la interfaz en un navegador normal
// (`npm run dev` y abrir http://localhost:1420). Solo se carga en desarrollo y
// fuera de Tauri; no llega al build de producción.
//
// Imita las reglas de src-tauri/src/db/tasks.rs. Si cambian allí, cámbialas aquí.
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Task } from "./types";

export function installDevMock() {
  let nextId = 1;
  const tasks = new Map<number, Task>();

  const make = (title: string, minsAgo = 0): Task => {
    const pos = Math.max(-1, ...[...tasks.values()].map((t) => t.position)) + 1;
    const t: Task = {
      id: nextId++,
      title,
      notes: null,
      position: pos,
      done: false,
      createdAt: Date.now() - minsAgo * 60_000,
      completedAt: null,
      remindAt: null,
      reminded: false,
    };
    tasks.set(t.id, t);
    return t;
  };

  const get = (id: number) => {
    const t = tasks.get(id);
    if (!t) throw `No existe la tarea ${id}`;
    return t;
  };

  const title = (s: string) => {
    const t = s.trim();
    if (!t) throw "El título no puede estar vacío";
    return t;
  };

  for (const s of ["Comprar pan", "Llamar al dentista", "Revisar facturas de octubre", "Preparar la reunión del lunes", "Regar las plantas"]) make(s, 60);
  get(2).notes = "Pedir cita para revisión";
  Object.assign(make("Sacar la basura"), { done: true, completedAt: Date.now() - 3_600_000 });

  const defaultShortcut = "Ctrl+Shift+Space";
  let shortcut = defaultShortcut;
  let autostart = false;
  const settings = () => ({
    shortcut,
    defaultShortcut,
    shortcutActive: true,
    shortcutError: null,
    wayland: false,
    autostart,
  });
  // Los diálogos de archivo no existen en el navegador: se simula una copia.
  const fakePath = "C:\\Users\\yo\\Documents\\tareas-copia.json";

  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, any>;
    switch (cmd) {
      case "app_version":
        return "0.0.0-mock";
      // Eventos de Tauri: escuchar no hace nada en el navegador.
      case "plugin:event|listen":
        return 1;
      case "plugin:event|unlisten":
        return null;
      case "current_view":
        return "list";
      case "hide_window":
        console.info("[mock] hide_window");
        return null;
      case "get_settings":
        return settings();
      case "set_shortcut":
        if (!/^(?=.*\b(Ctrl|Alt|Super)\+).+\+[^+]+$/.test(a.shortcut))
          throw `«${a.shortcut}» no es un atajo válido`;
        shortcut = a.shortcut;
        return settings();
      case "suspend_shortcut":
      case "resume_shortcut":
        return null;
      case "set_autostart":
        autostart = a.enabled;
        return settings();
      case "export_tasks":
        return { path: fakePath.replace("tareas-copia.json", a.defaultName), count: tasks.size, current: tasks.size };
      case "preview_import":
        return { path: fakePath, count: 2, current: tasks.size };
      case "import_tasks":
        tasks.clear();
        make("Tarea importada 1");
        make("Tarea importada 2");
        return 2;
      case "list_tasks":
        return [...tasks.values()].sort((x, y) => x.position - y.position || x.id - y.id);
      case "create_task":
        return make(title(a.title));
      case "rename_task":
        return Object.assign(get(a.id), { title: title(a.title) });
      case "set_task_notes":
        return Object.assign(get(a.id), { notes: a.notes?.trim() || null });
      case "set_task_done": {
        const t = get(a.id);
        return Object.assign(t, {
          done: a.done,
          completedAt: a.done ? (t.completedAt ?? Date.now()) : null,
        });
      }
      case "set_task_reminder":
        return Object.assign(get(a.id), { remindAt: a.remindAt, reminded: false });
      case "delete_task":
        get(a.id);
        tasks.delete(a.id);
        return null;
      case "reorder_tasks": {
        const ids: number[] = a.ids;
        if (new Set(ids).size !== ids.length) throw "Ids repetidos";
        const slots = ids.map((id) => get(id).position).sort((x, y) => x - y);
        ids.forEach((id, i) => (get(id).position = slots[i]));
        return null;
      }
      default:
        throw `Comando no simulado: ${cmd}`;
    }
  });
}
