<script lang="ts">
  import { tick } from "svelte";
  import { formatReminder } from "../lib/dates";
  import { store } from "../lib/tasks.svelte";
  import type { Task } from "../lib/types";
  import ReminderPicker from "./ReminderPicker.svelte";

  let { task, sortable = false }: { task: Task; sortable?: boolean } = $props();

  let row: HTMLElement;
  let titleInput = $state<HTMLInputElement>();
  let notesInput = $state<HTMLTextAreaElement>();

  let editing = $state(false);
  let draft = $state("");
  let notesOpen = $state(false);
  let notesDraft = $state("");
  let reminderOpen = $state(false);

  // Se recalcula cuando cambia la tarea (la lista se recarga al mostrar la
  // ventana), suficiente para que «hoy»/«mañana» estén al día.
  let reminderLabel = $derived(task.remindAt ? formatReminder(task.remindAt) : "");
  let overdue = $derived(!!task.remindAt && !task.done && !task.reminded && task.remindAt <= Date.now());

  // --- Título ---

  async function startEdit() {
    draft = task.title;
    editing = true;
    await tick();
    titleInput?.focus();
    titleInput?.select();
  }

  function commitEdit() {
    if (!editing) return;
    editing = false;
    const title = draft.trim();
    // Vaciar el título no borra la tarea: se descarta la edición.
    if (title && title !== task.title) void store.rename(task.id, title);
  }

  function onEditKey(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.isComposing) {
      e.preventDefault();
      commitEdit();
    } else if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      editing = false;
    }
  }

  function onTitleKey(e: KeyboardEvent) {
    if (e.key === "Enter" || e.key === "F2") {
      e.preventDefault();
      void startEdit();
    }
  }

  // --- Notas ---

  async function toggleNotes() {
    notesOpen = !notesOpen;
    if (!notesOpen) return;
    notesDraft = task.notes ?? "";
    await tick();
    notesInput?.focus();
  }

  function commitNotes() {
    const notes = notesDraft.trim();
    if (notes !== (task.notes ?? "")) void store.setNotes(task.id, notes || null);
  }

  function onNotesKey(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      e.stopPropagation();
      notesDraft = task.notes ?? "";
      notesOpen = false;
    } else if (e.key === "Enter" && e.ctrlKey) {
      e.preventDefault();
      notesInput?.blur();
      notesOpen = false;
    }
  }

  // --- Reordenar con teclado ---

  async function onRowKey(e: KeyboardEvent) {
    if (!sortable || !e.altKey || editing) return;
    const delta = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0;
    if (!delta) return;
    e.preventDefault();
    const focused = document.activeElement as HTMLElement | null;
    store.move(task.id, delta);
    await tick();
    // Al mover nodos del DOM se puede perder el foco; lo devolvemos.
    if (focused && row.contains(focused)) focused.focus();
  }
</script>

<!-- El keydown llega por burbujeo desde la casilla o los botones (Alt+↑/↓). -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  bind:this={row}
  class="task"
  class:done={task.done}
  class:urgent={task.urgent && !task.done}
  data-id={task.id}
  role="listitem"
  onkeydown={onRowKey}
>
  {#if sortable}
    <span class="grip" data-drag-handle title="Arrastra para reordenar (o Alt+↑/↓)">
      <svg viewBox="0 0 10 16" width="10" height="16" aria-hidden="true">
        <circle cx="3" cy="3" r="1.3" /><circle cx="7" cy="3" r="1.3" />
        <circle cx="3" cy="8" r="1.3" /><circle cx="7" cy="8" r="1.3" />
        <circle cx="3" cy="13" r="1.3" /><circle cx="7" cy="13" r="1.3" />
      </svg>
    </span>
  {:else}
    <span class="grip-space"></span>
  {/if}

  <label class="check">
    <input
      type="checkbox"
      checked={task.done}
      onchange={() => store.setDone(task.id, !task.done)}
      aria-label={task.done ? "Marcar como pendiente" : "Marcar como hecha"}
    />
    <span class="box" aria-hidden="true"></span>
  </label>

  <div class="body">
    {#if editing}
      <input
        bind:this={titleInput}
        bind:value={draft}
        class="edit"
        type="text"
        maxlength="500"
        spellcheck="false"
        aria-label="Editar tarea"
        onkeydown={onEditKey}
        onblur={commitEdit}
      />
    {:else}
      <span
        class="title"
        role="button"
        tabindex="0"
        title="Doble clic para editar"
        ondblclick={startEdit}
        onkeydown={onTitleKey}>{task.title}</span
      >
      {#if task.notes && !notesOpen}
        <span class="notes-preview">{task.notes}</span>
      {/if}
    {/if}

    {#if task.remindAt && !reminderOpen}
      <button
        class="reminder"
        class:overdue
        class:reminded={task.reminded}
        title={task.reminded ? "Ya avisado. Clic para cambiarlo" : "Clic para cambiarlo"}
        onclick={() => (reminderOpen = true)}
      >
        <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
          <path d="M4 11V7a4 4 0 0 1 8 0v4l1 1.5H3zM6.5 14h3" />
        </svg>
        {reminderLabel}{task.reminded ? " · avisado" : ""}
      </button>
    {/if}

    {#if reminderOpen}
      <ReminderPicker {task} onClose={() => (reminderOpen = false)} />
    {/if}

    {#if notesOpen}
      <textarea
        bind:this={notesInput}
        bind:value={notesDraft}
        class="notes"
        rows="3"
        placeholder="Notas (Ctrl+Enter para cerrar)"
        aria-label="Notas"
        onblur={commitNotes}
        onkeydown={onNotesKey}
      ></textarea>
    {/if}
  </div>

  <div class="actions">
    {#if !task.done}
      <button
        class="icon urgent-toggle"
        class:active={task.urgent}
        title={task.urgent ? "Quitar urgente" : "Marcar como urgente"}
        aria-label="Urgente"
        aria-pressed={task.urgent}
        onclick={() => store.setUrgent(task.id, !task.urgent)}
      >
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
          <path d="M8 2.5v7" />
          <circle cx="8" cy="12.8" r="0.4" />
        </svg>
      </button>
      <button
        class="icon"
        class:active={!!task.remindAt && !task.reminded}
        title={reminderOpen ? "Cerrar recordatorio" : "Recordatorio"}
        aria-label="Recordatorio"
        aria-expanded={reminderOpen}
        onclick={() => (reminderOpen = !reminderOpen)}
      >
        <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
          <path d="M4 11V7a4 4 0 0 1 8 0v4l1 1.5H3zM6.5 14h3" />
        </svg>
      </button>
    {/if}
    <button
      class="icon"
      class:active={!!task.notes}
      title={notesOpen ? "Cerrar notas" : "Notas"}
      aria-label="Notas"
      aria-expanded={notesOpen}
      onclick={toggleNotes}
    >
      <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
        <path d="M3 3.5h10M3 6.5h10M3 9.5h7M3 12.5h5" />
      </svg>
    </button>
    <button
      class="icon delete"
      title="Borrar"
      aria-label="Borrar tarea"
      onclick={() => store.remove(task.id)}
    >
      <svg viewBox="0 0 16 16" width="15" height="15" aria-hidden="true">
        <path d="M4 4l8 8M12 4l-8 8" />
      </svg>
    </button>
  </div>
</div>

<style>
  .task {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 6px;
    padding: 6px 8px 6px 2px;
    border-radius: 3px;
    /* Transparente para que se vea el margen rojo de la lista. */
    background: transparent;
  }

  .task:hover,
  .task:focus-within {
    background: var(--bg-hover);
  }

  /* --- Urgente --- */

  /* Papel: lo urgente va marcado con fosforito. */
  .task.urgent {
    background: var(--urgent-bg);
  }

  .task.urgent:hover,
  .task.urgent:focus-within {
    background: var(--urgent-bg-hover);
  }

  .task.urgent .title {
    font-weight: 600;
  }

  .icon.urgent-toggle.active {
    color: var(--urgent);
  }

  .icon.urgent-toggle svg {
    stroke-width: 2.2;
  }

  /* --- Arrastre (clases que pone reorder.ts) --- */

  :global(.sorting) .task:not(:global(.dragging)) {
    transition: transform 150ms ease;
  }

  .task:global(.dragging) {
    z-index: 1;
    background: var(--bg-input);
    box-shadow: var(--shadow-drag);
  }

  .grip,
  .grip-space {
    flex: none;
    width: 14px;
    height: 22px;
  }

  .grip {
    display: grid;
    place-items: center;
    color: var(--fg-muted);
    cursor: grab;
    touch-action: none;
    opacity: 0;
  }

  .grip svg {
    fill: currentColor;
  }

  .task:hover .grip,
  .task:global(.dragging) .grip {
    opacity: 1;
  }

  :global(.sorting) .grip {
    cursor: grabbing;
  }

  /* --- Casilla --- */

  .check {
    flex: none;
    position: relative;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    cursor: pointer;
  }

  .check input {
    position: absolute;
    inset: 0;
    margin: 0;
    opacity: 0;
    cursor: pointer;
  }

  .box {
    width: 17px;
    height: 17px;
    border: 1.5px solid var(--fg-muted);
    border-radius: 3px;
    position: relative;
  }

  .check input:checked + .box {
    background: var(--accent);
    border-color: var(--accent);
  }

  .check input:checked + .box::after {
    content: "";
    position: absolute;
    left: 5px;
    top: 1px;
    width: 4px;
    height: 9px;
    border: solid var(--on-accent);
    border-width: 0 2px 2px 0;
    transform: rotate(45deg);
  }

  .check input:focus-visible + .box {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  /* --- Texto --- */

  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding-top: 1px;
  }

  .title {
    overflow-wrap: anywhere;
    white-space: pre-wrap;
    cursor: default;
    outline: none;
    border-radius: 3px;
  }

  .title:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .done .title {
    color: var(--fg-muted);
    text-decoration: line-through;
  }

  .notes-preview {
    font-size: 0.85rem;
    color: var(--fg-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .edit,
  .notes {
    font: inherit;
    color: var(--fg);
    background: var(--bg-input);
    border: 1px solid var(--accent);
    border-radius: 4px;
    outline: none;
  }

  .edit {
    margin: -2px 0 -2px -5px;
    padding: 1px 4px;
  }

  .notes {
    margin: 4px 0 0 -5px;
    padding: 4px;
    font-size: 0.9rem;
    resize: vertical;
  }

  /* --- Acciones --- */

  .actions {
    flex: none;
    display: flex;
    gap: 2px;
  }

  .icon {
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
    opacity: 0;
  }

  .icon svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.6;
    stroke-linecap: round;
  }

  .task:hover .icon,
  .task:focus-within .icon,
  .icon.active,
  .icon[aria-expanded="true"] {
    opacity: 1;
  }

  .icon.active {
    color: var(--accent);
  }

  /* --- Etiqueta del recordatorio --- */

  .reminder {
    align-self: flex-start;
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin: 2px 0 0 -3px;
    padding: 0 4px;
    font: inherit;
    font-size: 0.8rem;
    color: var(--accent);
    background: none;
    border: none;
    border-radius: 3px;
    cursor: pointer;
  }

  .reminder:hover {
    background: var(--bg-active);
  }

  .reminder:focus-visible {
    outline: 2px solid var(--accent);
  }

  .reminder svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 1.5;
    stroke-linejoin: round;
    stroke-linecap: round;
  }

  .reminder.overdue {
    color: var(--danger);
  }

  .reminder.reminded,
  .done .reminder {
    color: var(--fg-muted);
  }

  .icon:hover {
    background: var(--bg-active);
    color: var(--fg);
  }

  .icon.delete:hover {
    color: var(--danger);
  }

  .icon:focus-visible {
    outline: 2px solid var(--accent);
    opacity: 1;
  }
</style>
