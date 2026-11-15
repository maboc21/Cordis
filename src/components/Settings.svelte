<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import * as api from "../lib/api";
  import { format as formatAccel, fromKeyEvent, loadLayoutMap, type LayoutMap } from "../lib/shortcut";
  import { store } from "../lib/tasks.svelte";
  import type { FileSummary, Settings } from "../lib/types";

  let { onBack }: { onBack: () => void } = $props();

  let settings = $state<Settings | null>(null);
  let version = $state("");
  let recording = $state(false);
  let preview = $state("");
  let message = $state<{ kind: "ok" | "error" | "hint"; text: string } | null>(null);
  let layout = $state<LayoutMap | null>(null);

  const format = (accel: string) => formatAccel(accel, layout);

  onMount(async () => {
    try {
      [settings, version, layout] = await Promise.all([
        api.getSettings(),
        api.appVersion(),
        loadLayoutMap(),
      ]);
    } catch (e) {
      message = { kind: "error", text: String(e) };
    }
  });

  // Si se sale de la vista a mitad de grabación, el atajo debe volver.
  onDestroy(() => {
    if (recording) void api.resumeShortcut();
  });

  async function startRecording() {
    if (recording) return;
    recording = true;
    preview = "";
    message = null;
    await api.suspendShortcut();
  }

  async function finish(accel: string | null) {
    if (!recording) return;
    recording = false;
    preview = "";
    if (!accel) {
      await api.resumeShortcut();
      return;
    }
    try {
      settings = await api.setShortcut(accel);
      message = { kind: "ok", text: `Atajo cambiado a ${format(accel)}` };
    } catch (e) {
      message = { kind: "error", text: String(e) };
      settings = await api.getSettings().catch(() => settings);
    }
  }

  function onRecordKey(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape" && !e.ctrlKey && !e.altKey && !e.metaKey) {
      void finish(null);
      return;
    }
    const r = fromKeyEvent(e, layout);
    if (r.kind === "partial") preview = r.label;
    else if (r.kind === "needs-modifier")
      message = { kind: "hint", text: "Incluye Ctrl, Alt o Win en la combinación" };
    else if (r.kind === "altgr")
      message = {
        kind: "hint",
        text: `Con tu teclado, esa combinación escribe «${r.char}» (AltGr). Elige otra para no bloquear ese carácter.`,
      };
    else void finish(r.value);
  }

  // --- Arranque automático ---

  async function toggleAutostart(e: Event) {
    const enabled = (e.currentTarget as HTMLInputElement).checked;
    try {
      settings = await api.setAutostart(enabled);
    } catch (err) {
      message = { kind: "error", text: String(err) };
      settings = await api.getSettings().catch(() => settings);
    }
  }

  // --- Copia de seguridad ---

  let backupMsg = $state<{ kind: "ok" | "error"; text: string } | null>(null);
  let pendingImport = $state<FileSummary | null>(null);
  let busy = $state(false);

  const plural = (n: number, one: string, many: string) => `${n} ${n === 1 ? one : many}`;
  const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

  function todayName() {
    const d = new Date();
    const pad = (n: number) => String(n).padStart(2, "0");
    return `cordis-${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}.json`;
  }

  async function run(fn: () => Promise<void>) {
    if (busy) return;
    busy = true;
    backupMsg = null;
    try {
      await fn();
    } catch (e) {
      backupMsg = { kind: "error", text: String(e) };
    } finally {
      busy = false;
    }
  }

  const doExport = () =>
    run(async () => {
      pendingImport = null;
      const r = await api.exportTasks(todayName());
      if (r)
        backupMsg = {
          kind: "ok",
          text: `${plural(r.count, "tarea exportada", "tareas exportadas")} a ${fileName(r.path)}`,
        };
    });

  const pickImport = () =>
    run(async () => {
      pendingImport = await api.previewImport();
    });

  const confirmImport = () =>
    run(async () => {
      if (!pendingImport) return;
      const n = await api.importTasks(pendingImport.path);
      pendingImport = null;
      await store.load();
      backupMsg = { kind: "ok", text: `${plural(n, "tarea importada", "tareas importadas")}` };
    });

  async function restoreDefault() {
    if (!settings) return;
    await finish(null);
    try {
      settings = await api.setShortcut(settings.defaultShortcut);
      message = { kind: "ok", text: "Atajo restablecido" };
    } catch (e) {
      message = { kind: "error", text: String(e) };
    }
  }
</script>

<div class="settings">
  <header data-tauri-drag-region>
    <button class="back" onclick={onBack} title="Volver (Esc)">
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
      Volver
    </button>
    <h1 data-tauri-drag-region>Ajustes</h1>
  </header>

  <section>
    <h2>Atajo para mostrar u ocultar</h2>

    {#if settings}
      <div class="row">
        <button
          class="recorder"
          class:recording
          onclick={startRecording}
          onkeydown={onRecordKey}
          onblur={() => finish(null)}
          aria-label="Cambiar atajo"
        >
          {#if recording}
            {preview || "Pulsa la combinación…"}
          {:else}
            {format(settings.shortcut)}
          {/if}
        </button>
        <button class="link" onclick={restoreDefault} disabled={settings.shortcut === settings.defaultShortcut}>
          Restablecer
        </button>
      </div>

      <p class="help">
        {recording ? "Esc para cancelar." : "Haz clic en el atajo y pulsa la nueva combinación."}
      </p>

      {#if settings.wayland}
        <p class="note">
          En Wayland las aplicaciones no pueden registrar atajos globales. Asigna este comando a
          una tecla en los ajustes de teclado de tu escritorio:
          <code>cordis --toggle</code>
        </p>
      {:else if !settings.shortcutActive && !recording}
        <p class="note warn">
          El atajo no está activo. Puede que lo esté usando otra aplicación: prueba con otro.
          {#if settings.shortcutError}
            <span class="detail">{settings.shortcutError}</span>
          {/if}
        </p>
      {/if}
    {/if}

    {#if message}
      <p class="message {message.kind}" role="status">{message.text}</p>
    {/if}

    {#if settings}
      <h2>Inicio</h2>
      <label class="switch">
        <input type="checkbox" checked={settings.autostart} onchange={toggleAutostart} />
        Arrancar con el sistema (en la bandeja, sin abrir la ventana)
      </label>
    {/if}

    <h2>Copia de seguridad</h2>
    <div class="row">
      <button class="action" onclick={doExport} disabled={busy}>Exportar…</button>
      <button class="action" onclick={pickImport} disabled={busy}>Importar…</button>
    </div>

    {#if pendingImport}
      <div class="confirm" role="alertdialog" aria-label="Confirmar importación">
        <p>
          <strong>{fileName(pendingImport.path)}</strong> tiene
          {plural(pendingImport.count, "tarea", "tareas")}.
          {#if pendingImport.current === 0}
            La lista actual está vacía.
          {:else if pendingImport.current === 1}
            Al importarlo se <strong>reemplazará</strong> la tarea actual.
          {:else}
            Al importarlo se <strong>reemplazarán</strong> las {pendingImport.current} tareas actuales.
          {/if}
        </p>
        <div class="row">
          <button class="action danger" onclick={confirmImport} disabled={busy}>
            Importar y reemplazar
          </button>
          <button class="link" onclick={() => (pendingImport = null)}>Cancelar</button>
        </div>
      </div>
    {/if}

    {#if backupMsg}
      <p class="message {backupMsg.kind}" role="status">{backupMsg.text}</p>
    {/if}
  </section>

  <footer>Cordis {version && `v${version}`}</footer>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
    padding: 0 16px 12px;
  }

  header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }

  h1 {
    flex: 1;
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
  }

  h2 {
    margin: 0 0 8px;
    font-size: 0.85rem;
    font-weight: 600;
    color: var(--fg-muted);
  }

  section {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  section h2:not(:first-child) {
    margin-top: 20px;
  }

  .switch {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.9rem;
    cursor: pointer;
  }

  .switch input {
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--accent);
  }

  .action {
    padding: 6px 14px;
    font-size: 0.9rem;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .action:hover:not(:disabled) {
    border-color: var(--fg-muted);
  }

  .action:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .action.danger {
    color: var(--on-accent);
    background: var(--danger);
    border-color: var(--danger);
  }

  .confirm {
    margin-top: 10px;
    padding: 10px 12px;
    font-size: 0.88rem;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .confirm p {
    margin: 0 0 10px;
    overflow-wrap: anywhere;
  }

  button {
    font: inherit;
    color: var(--fg);
    cursor: pointer;
  }

  button:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }

  .back {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px 4px 4px;
    font-size: 0.85rem;
    color: var(--fg-muted);
    background: none;
    border: none;
    border-radius: 4px;
  }

  .back:hover {
    color: var(--fg);
    background: var(--bg-hover);
  }

  .back svg {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .recorder {
    min-width: 220px;
    padding: 8px 14px;
    text-align: left;
    background: var(--bg-input);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .recorder:hover {
    border-color: var(--fg-muted);
  }

  .recorder.recording {
    color: var(--accent);
    border-color: var(--accent);
  }

  .link {
    padding: 0;
    font-size: 0.85rem;
    color: var(--accent);
    background: none;
    border: none;
  }

  .link:disabled {
    color: var(--fg-muted);
    cursor: default;
    opacity: 0.6;
  }

  .help,
  .note,
  .message {
    margin: 8px 0 0;
    font-size: 0.85rem;
  }

  .help {
    color: var(--fg-muted);
  }

  .note {
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--bg-hover);
  }

  .note.warn,
  .message.error {
    color: var(--danger);
  }

  .detail {
    display: block;
    margin-top: 4px;
    font-size: 0.8rem;
    color: var(--fg-muted);
    user-select: text;
  }

  .message.ok {
    color: var(--accent);
  }

  .message.hint {
    color: var(--fg-muted);
  }

  code {
    display: block;
    margin-top: 6px;
    font-family: ui-monospace, "Cascadia Mono", Consolas, monospace;
    user-select: text;
  }

  footer {
    font-size: 0.75rem;
    color: var(--fg-muted);
  }
</style>
