<script lang="ts">
  // Interfaz de la ventana `reminder` (src-tauri/src/popup.rs). No se cierra
  // sola: cada aviso se marca como hecho, se pospone o se cierra a mano.
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import { formatReminder, snoozeOptions } from "../lib/dates";
  import type { Task } from "../lib/types";

  let tasks = $state<Task[]>([]);
  let snoozing = $state<number | null>(null);
  let error = $state<string | null>(null);
  let root: HTMLElement;

  async function load() {
    try {
      tasks = await api.popupTasks();
    } catch (e) {
      error = String(e);
    }
  }

  async function act(fn: () => Promise<void>) {
    error = null;
    try {
      await fn();
    } catch (e) {
      error = String(e);
    }
  }

  const done = (id: number) => act(() => api.popupDone(id));
  const snooze = (id: number, until: number) => act(() => api.popupSnooze(id, until));

  onMount(() => {
    void load();
    const unlisteners = [listen("popup-updated", load), listen("tasks-changed", load)];

    // La ventana se ajusta al contenido: al llegar avisos, al desplegar
    // «Posponer»… Rust la mantiene pegada a la esquina.
    const ro = new ResizeObserver(() => void api.popupResize(Math.ceil(root.scrollHeight)));
    ro.observe(root);

    return () => {
      ro.disconnect();
      unlisteners.forEach((u) => void u.then((f) => f()));
    };
  });

  const firstLine = (s: string) => s.split("\n")[0];
</script>

<div class="popup" bind:this={root}>
  <header>
    <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
      <path d="M4 11V7a4 4 0 0 1 8 0v4l1 1.5H3zM6.5 14h3" />
    </svg>
    <span class="heading">
      {tasks.length > 1 ? `${tasks.length} recordatorios` : "Recordatorio"}
    </span>
    <button class="close" title="Cerrar" aria-label="Cerrar" onclick={() => act(api.popupClose)}>
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" /></svg>
    </button>
  </header>

  <ul>
    {#each tasks as t (t.id)}
      <li>
        <div class="title">{t.title}</div>
        {#if t.notes}
          <div class="notes">{firstLine(t.notes)}</div>
        {/if}
        {#if t.remindAt}
          <div class="when">{formatReminder(t.remindAt)}</div>
        {/if}

        <div class="actions">
          {#if snoozing === t.id}
            <span class="label">Posponer:</span>
            {#each snoozeOptions() as o, i (o.label)}
              <!-- La hora se calcula al pulsar, no al abrir el menú. -->
              <button onclick={() => snooze(t.id, snoozeOptions()[i].ms)}>{o.label}</button>
            {/each}
            <button class="link" onclick={() => (snoozing = null)}>Cancelar</button>
          {:else}
            <button class="primary" onclick={() => done(t.id)}>
              <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true"><path d="M3 8.5l3 3 7-7" /></svg>
              Hecha
            </button>
            <button onclick={() => (snoozing = t.id)}>Posponer…</button>
          {/if}
        </div>
      </li>
    {/each}
  </ul>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <footer>
    <button class="link" onclick={() => act(api.popupOpenList)}>Abrir la lista</button>
  </footer>
</div>

<style>
  /* En esta ventana se puede desplazar si el contenido supera la altura máxima. */
  :global(html:has(.popup)),
  :global(body:has(.popup)) {
    overflow-y: auto;
  }

  .popup {
    display: flex;
    flex-direction: column;
    border-top: 3px solid var(--accent);
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 8px 6px 14px;
    color: var(--accent);
    font-weight: 600;
    font-size: 0.9rem;
  }

  header > svg,
  .close svg,
  .primary svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .heading {
    flex: 1;
  }

  ul {
    margin: 0;
    padding: 0 8px;
    list-style: none;
  }

  li {
    padding: 8px 6px 10px;
  }

  li + li {
    border-top: 1px solid var(--border);
  }

  .title {
    font-weight: 600;
    overflow-wrap: anywhere;
  }

  .notes,
  .when {
    font-size: 0.82rem;
    color: var(--fg-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
  }

  .label {
    font-size: 0.82rem;
    color: var(--fg-muted);
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    font: inherit;
    font-size: 0.82rem;
    color: var(--fg);
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
  }

  button:hover {
    border-color: var(--fg-muted);
  }

  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .primary {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
  }

  .link {
    padding: 4px 2px;
    color: var(--accent);
    background: none;
    border-color: transparent;
  }

  .close {
    padding: 4px;
    color: var(--fg-muted);
    background: none;
    border-color: transparent;
  }

  .close:hover {
    color: var(--fg);
    background: var(--bg-hover);
  }

  .error {
    margin: 0 14px 6px;
    font-size: 0.8rem;
    color: var(--danger);
  }

  footer {
    display: flex;
    justify-content: flex-end;
    padding: 2px 12px 8px;
    border-top: 1px solid var(--border);
  }
</style>
