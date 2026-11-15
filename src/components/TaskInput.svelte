<script lang="ts">
  import { onMount } from "svelte";
  import { store } from "../lib/tasks.svelte";

  let value = $state("");
  let input: HTMLInputElement;
  let busy = false;

  async function onKeydown(e: KeyboardEvent) {
    if (e.key !== "Enter" || e.isComposing || busy) return;
    e.preventDefault();
    const title = value.trim();
    if (!title) return;
    busy = true;
    if (await store.add(title)) value = "";
    busy = false;
  }

  export function focus() {
    input?.focus();
  }

  onMount(() => {
    focus();
    // Al volver a mostrar la ventana, se puede escribir directamente.
    window.addEventListener("focus", focus);
    return () => window.removeEventListener("focus", focus);
  });
</script>

<input
  bind:this={input}
  bind:value
  class="new-task"
  type="text"
  placeholder="Añade una tarea y pulsa Enter"
  aria-label="Nueva tarea"
  maxlength="500"
  spellcheck="false"
  onkeydown={onKeydown}
/>

<style>
  .new-task {
    margin: 0 12px 6px;
    padding: 12px 14px;
    font: inherit;
    font-size: 1.1rem;
    color: var(--fg);
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 8px;
    outline: none;
  }

  .new-task:focus {
    border-color: var(--accent);
  }

  .new-task::placeholder {
    color: var(--fg-muted);
  }
</style>
