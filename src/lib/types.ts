// Espejo de `db::tasks::Task` en Rust (serializado en camelCase).
// Las fechas son milisegundos Unix UTC.
export interface Task {
  id: number;
  title: string;
  notes: string | null;
  position: number;
  done: boolean;
  createdAt: number;
  completedAt: number | null;
  remindAt: number | null;
  /** Ya se ha mostrado la notificación de `remindAt`. */
  reminded: boolean;
}

// Espejo de `commands::SettingsView`.
export interface Settings {
  /** Formato del plugin, p. ej. "Ctrl+Shift+Space". */
  shortcut: string;
  defaultShortcut: string;
  /** El atajo está registrado de verdad. */
  shortcutActive: boolean;
  /** Motivo por el que no se pudo registrar, si lo hay. */
  shortcutError: string | null;
  /** Sesión Wayland: sin atajos globales, hay que usar `--toggle`. */
  wayland: boolean;
}

/** Vista que pide Rust al mostrar la ventana (evento `show-view`). */
export type View = "list" | "settings";
