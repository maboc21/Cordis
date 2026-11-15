<script lang="ts">
  import { onMount, untrack } from "svelte";
  import { fromLocalInput, presets, toLocalInput } from "../lib/dates";
  import { store } from "../lib/tasks.svelte";
  import type { Task } from "../lib/types";

  let { task, onClose }: { task: Task; onClose: () => void } = $props();

  const now = Date.now();
  const options = presets(now);
  // Si ya tiene un recordatorio futuro, el campo empieza con él (solo el
  // valor inicial: luego es lo que escriba el usuario).
  let custom = $state(
    untrack(() => (task.remindAt && task.remindAt > now ? toLocalInput(task.remindAt) : "")),
  );
  let error = $state<string | null>(null);
  let root: HTMLElement;

  onMount(() => root.querySelector("button")?.focus());

  async function save(ms: number | null) {
    await store.setReminder(task.id, ms);
    onClose();
  }

  function saveCustom() {
    const ms = fromLocalInput(custom);
    if (ms === null) {
      error = "Elige fecha y hora";
    } else if (ms <= Date.now()) {
      error = "Esa hora ya ha pasado";
    } else {
      void save(ms);
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      onClose();
    } else if (e.key === "Enter" && (e.target as HTMLElement).tagName === "INPUT") {
      e.preventDefault();
      saveCustom();
    }
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div bind:this={root} class="picker" role="group" aria-label="Recordatorio" onkeydown={onKeydown}>
  <div class="presets">
    {#each options as p (p.label)}
      <button onclick={() => save(p.ms)}>{p.label}</button>
    {/each}
  </div>

  <div class="custom">
    <input
      type="datetime-local"
      bind:value={custom}
      min={toLocalInput(now)}
      aria-label="Fecha y hora del recordatorio"
      oninput={() => (error = null)}
    />
    <button class="primary" onclick={saveCustom} disabled={!custom}>Poner</button>
    {#if task.remindAt}
      <button class="remove" onclick={() => save(null)}>Quitar</button>
    {/if}
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</div>

<style>
  .picker {
    margin: 6px 0 2px -5px;
    padding: 8px;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .presets,
  .custom {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .custom {
    margin-top: 8px;
    align-items: center;
  }

  button {
    padding: 3px 9px;
    font: inherit;
    font-size: 0.82rem;
    color: var(--fg);
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: 12px;
    cursor: pointer;
  }

  button:hover:not(:disabled) {
    border-color: var(--fg-muted);
  }

  button:focus-visible,
  input:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  button:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .primary {
    color: var(--on-accent);
    background: var(--accent);
    border-color: var(--accent);
  }

  .remove {
    color: var(--danger);
    background: none;
    border-color: transparent;
  }

  input {
    padding: 3px 6px;
    font: inherit;
    font-size: 0.85rem;
    color: var(--fg);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .error {
    margin: 6px 0 0;
    font-size: 0.8rem;
    color: var(--danger);
  }
</style>
