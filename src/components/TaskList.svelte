<script lang="ts">
  import { sortable } from "../lib/reorder";
  import { store } from "../lib/tasks.svelte";
  import TaskItem from "./TaskItem.svelte";

  const DONE_OPEN_KEY = "doneOpen";

  // Preferencia visual por equipo; si localStorage falla, empieza plegada.
  let doneOpen = $state(readDoneOpen());

  function readDoneOpen() {
    try {
      return localStorage.getItem(DONE_OPEN_KEY) === "1";
    } catch {
      return false;
    }
  }

  function toggleDone() {
    doneOpen = !doneOpen;
    try {
      localStorage.setItem(DONE_OPEN_KEY, doneOpen ? "1" : "0");
    } catch {
      /* sin persistencia, no pasa nada */
    }
  }

  const onReorder = (ids: number[]) => store.reorder(ids);
</script>

{#if store.loaded && store.tasks.length === 0}
  <p class="empty">Todavía no hay tareas.</p>
{:else}
  {#if store.loaded && store.pending.length === 0}
    <p class="all-done">No hay nada pendiente.</p>
  {/if}

  <div class="list" role="list" aria-label="Pendientes" use:sortable={{ onReorder }}>
    {#each store.pending as task (task.id)}
      <TaskItem {task} sortable />
    {/each}
  </div>

  {#if store.done.length > 0}
    <button class="done-header" aria-expanded={doneOpen} onclick={toggleDone}>
      <svg class="chevron" viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
        <path d="M6 4l4 4-4 4" />
      </svg>
      Hechas ({store.done.length})
    </button>

    {#if doneOpen}
      <div class="list" role="list" aria-label="Hechas">
        {#each store.done as task (task.id)}
          <TaskItem {task} />
        {/each}
      </div>
    {/if}
  {/if}
{/if}

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .empty,
  .all-done {
    margin: 0;
    padding: 24px 0;
    text-align: center;
    color: var(--fg-muted);
  }

  .empty {
    padding-top: 80px;
  }

  .done-header {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 10px 0 2px;
    padding: 4px 6px;
    font: inherit;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-muted);
    background: none;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .done-header:hover {
    color: var(--fg);
  }

  .done-header:focus-visible {
    outline: 2px solid var(--accent);
  }

  .chevron {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    transition: transform 120ms ease;
  }

  [aria-expanded="true"] .chevron {
    transform: rotate(90deg);
  }
</style>
