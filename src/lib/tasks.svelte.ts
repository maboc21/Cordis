// Estado de la lista de tareas (Svelte 5 runes). Es la única fuente de verdad
// en la interfaz: los componentes leen de aquí y llaman a sus métodos, que a
// su vez llaman a Rust. Si un comando falla se muestra el error y, cuando el
// cambio era optimista, se recarga la lista desde la base de datos.
import * as api from "./api";
import type { Task } from "./types";

const ERROR_MS = 5000;

const byPosition = (a: Task, b: Task) => a.position - b.position || a.id - b.id;

class TaskStore {
  /** Todas las tareas, ordenadas por `position`. */
  tasks = $state<Task[]>([]);
  loaded = $state(false);
  error = $state<string | null>(null);

  pending = $derived(this.tasks.filter((t) => !t.done));
  /** Hechas, la más reciente primero. */
  done = $derived(
    this.tasks
      .filter((t) => t.done)
      .sort((a, b) => (b.completedAt ?? 0) - (a.completedAt ?? 0)),
  );

  #errorTimer: ReturnType<typeof setTimeout> | undefined;

  fail(e: unknown) {
    this.error = String(e);
    clearTimeout(this.#errorTimer);
    this.#errorTimer = setTimeout(() => (this.error = null), ERROR_MS);
  }

  async load() {
    try {
      this.tasks = (await api.listTasks()).sort(byPosition);
      this.loaded = true;
    } catch (e) {
      this.fail(e);
    }
  }

  /** Sustituye una tarea por la versión que devuelve Rust. */
  #put(task: Task) {
    const i = this.tasks.findIndex((t) => t.id === task.id);
    if (i >= 0) this.tasks[i] = task;
  }

  async #call(fn: () => Promise<Task>) {
    try {
      this.#put(await fn());
    } catch (e) {
      this.fail(e);
    }
  }

  async add(title: string): Promise<boolean> {
    try {
      this.tasks.push(await api.createTask(title));
      return true;
    } catch (e) {
      this.fail(e);
      return false;
    }
  }

  rename(id: number, title: string) {
    return this.#call(() => api.renameTask(id, title));
  }

  setNotes(id: number, notes: string | null) {
    return this.#call(() => api.setTaskNotes(id, notes));
  }

  setDone(id: number, done: boolean) {
    return this.#call(() => api.setTaskDone(id, done));
  }

  setReminder(id: number, remindAt: number | null) {
    return this.#call(() => api.setTaskReminder(id, remindAt));
  }

  async remove(id: number) {
    this.tasks = this.tasks.filter((t) => t.id !== id);
    try {
      await api.deleteTask(id);
    } catch (e) {
      this.fail(e);
      await this.load();
    }
  }

  /**
   * Deja `ids` en ese orden. Se aplica al momento (antes del primer `await`,
   * para que la acción de arrastre vea el DOM ya reordenado tras `tick()`)
   * con la misma regla que `db::tasks::reorder`: las tareas se reparten las
   * posiciones que ya ocupaban.
   */
  reorder(ids: number[]) {
    const byId = new Map(this.tasks.map((t) => [t.id, t]));
    const slots = ids.map((id) => byId.get(id)!.position).sort((a, b) => a - b);
    ids.forEach((id, i) => (byId.get(id)!.position = slots[i]));
    this.tasks.sort(byPosition);

    api.reorderTasks(ids).catch((e) => {
      this.fail(e);
      void this.load();
    });
  }

  /** Mueve una tarea pendiente `delta` puestos (teclado: Alt+↑/↓). */
  move(id: number, delta: number) {
    const ids = this.pending.map((t) => t.id);
    const from = ids.indexOf(id);
    const to = from + delta;
    if (from < 0 || to < 0 || to >= ids.length) return;
    ids.splice(to, 0, ...ids.splice(from, 1));
    this.reorder(ids);
  }
}

export const store = new TaskStore();
