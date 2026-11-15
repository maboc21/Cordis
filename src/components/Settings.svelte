<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import * as api from "../lib/api";
  import { format as formatAccel, fromKeyEvent, loadLayoutMap, type LayoutMap } from "../lib/shortcut";
  import type { Settings } from "../lib/types";

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
  <header>
    <button class="back" onclick={onBack} title="Volver (Esc)">
      <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true"><path d="M10 3L5 8l5 5" /></svg>
      Volver
    </button>
    <h1>Ajustes</h1>
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
          <code>organizador-tareas --toggle</code>
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
  </section>

  <footer>Organizador de tareas {version && `v${version}`}</footer>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 12px 16px;
  }

  header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }

  h1 {
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
