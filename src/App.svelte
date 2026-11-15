<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import Settings from "./components/Settings.svelte";
  import TaskInput from "./components/TaskInput.svelte";
  import TaskList from "./components/TaskList.svelte";
  import * as api from "./lib/api";
  import { store } from "./lib/tasks.svelte";
  import type { View } from "./lib/types";

  let view = $state<View>("list");

  onMount(() => {
    void store.load();
    // El `show-view` del arranque llega antes de escuchar: se pregunta.
    api.currentView().then((v) => (view = v), () => {});
    // Rust avisa cada vez que muestra la ventana y dice qué vista abrir
    // (la bandeja puede pedir Ajustes). De paso se recarga la lista.
    const unlisten = listen<View>("show-view", (e) => {
      view = e.payload;
      void store.load();
    });
    // El planificador marca recordatorios como avisados.
    const unlistenTasks = listen("tasks-changed", () => void store.load());
    return () => {
      void unlisten.then((f) => f());
      void unlistenTasks.then((f) => f());
    };
  });

  // Esc: en Ajustes vuelve a la lista; en la lista oculta la ventana.
  // Los Esc con significado propio (editar, notas, arrastrar, grabar atajo)
  // hacen stopPropagation y no llegan aquí.
  function onKeydown(e: KeyboardEvent) {
    if (e.key !== "Escape" || e.defaultPrevented) return;
    e.preventDefault();
    if (view === "settings") view = "list";
    else void api.hideWindow();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<main>
  {#if view === "settings"}
    <Settings onBack={() => (view = "list")} />
  {:else}
    <TaskInput />

    <section class="scroll">
      <TaskList />
    </section>

    <footer>
      {#if store.error}
        <span class="error" role="alert">{store.error}</span>
      {:else}
        <span>
          {store.pending.length}
          {store.pending.length === 1 ? "pendiente" : "pendientes"}
          {#if store.done.length}· {store.done.length} {store.done.length === 1 ? "hecha" : "hechas"}{/if}
        </span>
      {/if}
      <button class="gear" title="Ajustes" aria-label="Ajustes" onclick={() => (view = "settings")}>
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <circle cx="8" cy="8" r="2.2" />
          <path
            d="M8 1.5v1.8M8 12.7v1.8M1.5 8h1.8M12.7 8h1.8M3.4 3.4l1.3 1.3M11.3 11.3l1.3 1.3M3.4 12.6l1.3-1.3M11.3 4.7l1.3-1.3"
          />
        </svg>
      </button>
    </footer>
  {/if}
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 8px 8px;
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 4px 6px 4px 12px;
    font-size: 0.75rem;
    color: var(--fg-muted);
    border-top: 1px solid var(--border);
  }

  footer > span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .error {
    color: var(--danger);
  }

  .gear {
    flex: none;
    display: grid;
    place-items: center;
    width: 24px;
    height: 22px;
    padding: 0;
    color: var(--fg-muted);
    background: none;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .gear:hover {
    color: var(--fg);
    background: var(--bg-hover);
  }

  .gear:focus-visible {
    outline: 2px solid var(--accent);
  }

  .gear svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
  }
</style>
