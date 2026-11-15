// Envoltorios tipados de los comandos de Rust (src-tauri/src/commands.rs).
// La interfaz solo habla con el backend a través de este módulo.
// Si un comando falla, la promesa se rechaza con un texto en español.
import { invoke } from "@tauri-apps/api/core";
import type { FileSummary, Settings, Task, View } from "./types";

export const appVersion = () => invoke<string>("app_version");

export const listTasks = () => invoke<Task[]>("list_tasks");

export const createTask = (title: string) => invoke<Task>("create_task", { title });

export const renameTask = (id: number, title: string) =>
  invoke<Task>("rename_task", { id, title });

export const setTaskNotes = (id: number, notes: string | null) =>
  invoke<Task>("set_task_notes", { id, notes });

export const setTaskUrgent = (id: number, urgent: boolean) =>
  invoke<Task>("set_task_urgent", { id, urgent });

export const setTaskDone = (id: number, done: boolean) =>
  invoke<Task>("set_task_done", { id, done });

export const setTaskReminder = (id: number, remindAt: number | null) =>
  invoke<Task>("set_task_reminder", { id, remindAt });

export const deleteTask = (id: number) => invoke<void>("delete_task", { id });

/** Deja las tareas `ids` en ese orden; las demás no se mueven. */
export const reorderTasks = (ids: number[]) => invoke<void>("reorder_tasks", { ids });

export const hideWindow = () => invoke<void>("hide_window");

/** Fija (no se oculta al perder el foco) o suelta la ventana. */
export const setPinned = (pinned: boolean) => invoke<void>("set_pinned", { pinned });

export const isPinned = () => invoke<boolean>("is_pinned");

/** Vista con la que Rust mostró la ventana (para la carga inicial). */
export const currentView = () => invoke<View>("current_view");

export const getSettings = () => invoke<Settings>("get_settings");

/** Registra y guarda el atajo. Si falla, sigue activo el anterior. */
export const setShortcut = (shortcut: string) =>
  invoke<Settings>("set_shortcut", { shortcut });

/** Desactiva el atajo mientras se graba uno nuevo. */
export const suspendShortcut = () => invoke<void>("suspend_shortcut");

/** Vuelve a activar el atajo guardado. */
export const resumeShortcut = () => invoke<void>("resume_shortcut");

export const setAutostart = (enabled: boolean) =>
  invoke<Settings>("set_autostart", { enabled });

/** Pide dónde guardar y exporta. `null` si se cancela. */
export const exportTasks = (defaultName: string) =>
  invoke<FileSummary | null>("export_tasks", { defaultName });

/** Pide un archivo y lo valida sin importarlo. `null` si se cancela. */
export const previewImport = () => invoke<FileSummary | null>("preview_import");

/** Reemplaza todas las tareas por las del archivo. Devuelve cuántas hay. */
export const importTasks = (path: string) => invoke<number>("import_tasks", { path });

// --- Popup de recordatorios (ventana `reminder`) ---

export const popupTasks = () => invoke<Task[]>("popup_tasks");

/** Pide a Rust que ajuste la ventana a esta altura (px lógicos). */
export const popupResize = (height: number) => invoke<void>("popup_resize", { height });

export const popupDone = (id: number) => invoke<void>("popup_done", { id });

/** Vuelve a avisar en `until` (ms Unix UTC). */
export const popupSnooze = (id: number, until: number) =>
  invoke<void>("popup_snooze", { id, until });

export const popupOpenList = () => invoke<void>("popup_open_list");

export const popupClose = () => invoke<void>("popup_close");
