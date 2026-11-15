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
  /** Fijada: no se oculta al perder el foco (arrastrarla la fija; ver window.rs). */
  let pinned = $state(false);

  onMount(() => {
    void store.load();
    // El `show-view` del arranque llega antes de escuchar: se pregunta.
    api.currentView().then((v) => (view = v), () => {});
    api.isPinned().then((p) => (pinned = p), () => {});
    const unlistenPinned = listen<boolean>("pinned-changed", (e) => (pinned = e.payload));
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
      void unlistenPinned.then((f) => f());
    };
  });

  function togglePinned() {
    api.setPinned(!pinned).catch((e) => store.fail(e));
  }

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
  <!-- Asa para arrastrar la ventana. En Tauri 2 `data-tauri-drag-region`
       no se hereda: lo lleva cada elemento que debe servir para arrastrar. -->
  <div
    class="drag-bar"
    class:pinned
    data-tauri-drag-region
    title={pinned ? "Arrastra para moverla" : "Arrastra para moverla (y que no se oculte al cambiar de ventana)"}
  >
    <span class="pill" data-tauri-drag-region></span>
  </div>

  {#if view === "settings"}
    <Settings onBack={() => (view = "list")} />
  {:else}
    <TaskInput />

    <section class="scroll">
      <TaskList />
    </section>

    <footer data-tauri-drag-region>
      {#if store.error}
        <span class="error" role="alert">{store.error}</span>
      {:else}
        <span data-tauri-drag-region>
          {store.pending.length}
          {store.pending.length === 1 ? "pendiente" : "pendientes"}
          {#if store.done.length}· {store.done.length} {store.done.length === 1 ? "hecha" : "hechas"}{/if}
        </span>
      {/if}
      <span class="spacer" data-tauri-drag-region></span>
      <button
        class="icon"
        class:active={pinned}
        title={pinned
          ? "Fijada: no se oculta al cambiar de ventana. Clic para soltarla"
          : "Fijar: que no se oculte al cambiar de ventana"}
        aria-label="Fijar ventana"
        aria-pressed={pinned}
        onclick={togglePinned}
      >
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path d="M6 2h4l-.5 4 2.5 2.5v1H4v-1L6.5 6zM8 9.5V14" />
        </svg>
      </button>
      <button class="icon" title="Ajustes" aria-label="Ajustes" onclick={() => (view = "settings")}>
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

  .drag-bar {
    flex: none;
    display: grid;
    place-items: center;
    height: 14px;
    cursor: move;
  }

  .pill {
    width: 36px;
    height: 4px;
    border-radius: 2px;
    background: var(--pill);
    transition: background 120ms ease;
  }

  .drag-bar:hover .pill {
    background: var(--fg-muted);
  }

  .drag-bar.pinned .pill {
    background: var(--accent);
  }

  .scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 0 8px 8px;
    /* Margen rojo de la libreta, entre el asa de arrastre y la casilla. Las
       filas son transparentes para que se vea. */
    background: linear-gradient(var(--margin-line), var(--margin-line)) 26px 0 / 1.5px 100%
      no-repeat;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 6px 4px 12px;
    font-size: 0.75rem;
    color: var(--fg-muted);
    border-top: 1px solid var(--border);
    cursor: move;
  }

  .spacer {
    flex: 1;
    align-self: stretch;
  }

  footer > span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .error {
    color: var(--danger);
  }

  .icon {
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

  .icon:hover {
    color: var(--fg);
    background: var(--bg-hover);
  }

  .icon.active {
    color: var(--accent);
  }

  .icon:focus-visible {
    outline: 2px solid var(--accent);
  }

  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
