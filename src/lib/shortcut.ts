// Conversión entre teclas del navegador y el formato de atajo del plugin
// global-shortcut ("Ctrl+Alt+Space"), y su presentación en español.
//
// Ojo con los teclados europeos: en Windows, Ctrl+Alt equivale a AltGr, y
// combinaciones como Ctrl+Alt+2 (@) o Ctrl+Alt++ (]) escriben caracteres.
// Registrarlas como atajo global impediría escribir esos caracteres, así que
// se rechazan.

/** Carácter que escribe cada tecla física sin modificadores (KeyboardLayoutMap). */
export type LayoutMap = ReadonlyMap<string, string>;

/** Mapa de la distribución de teclado actual, si el motor lo soporta (WebView2 sí, WebKitGTK no). */
export async function loadLayoutMap(): Promise<LayoutMap | null> {
  try {
    const kb = (navigator as { keyboard?: { getLayoutMap(): Promise<LayoutMap> } }).keyboard;
    return kb ? await kb.getLayoutMap() : null;
  } catch {
    return null;
  }
}

const MODIFIER_CODE = /^(Control|Alt|Shift|Meta|OS|AltGraph)(Left|Right)?$/;

/** Nombre de tecla para el plugin a partir de `KeyboardEvent.code`. */
function keyName(code: string): string | null {
  if (MODIFIER_CODE.test(code)) return null;
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return code; // Space, F1…F24, ArrowUp, Backquote, BracketRight, Numpad1…
}

/** Carácter que la tecla escribiría sola, o null si no se sabe. */
function baseChar(code: string, layout: LayoutMap | null): string | null {
  const fromLayout = layout?.get(code);
  if (fromLayout) return fromLayout;
  if (code.startsWith("Key")) return code.slice(3).toLowerCase();
  if (code.startsWith("Digit")) return code.slice(5);
  return null;
}

/**
 * ¿Esta pulsación con Ctrl+Alt escribe un carácter (AltGr)? Se detecta porque
 * `e.key` no es el carácter base de la tecla.
 */
function isAltGrChar(e: KeyboardEvent, layout: LayoutMap | null): boolean {
  if (!(e.ctrlKey && e.altKey) || e.metaKey || e.key.length !== 1 || e.key === " ") return false;
  if (e.getModifierState?.("AltGraph")) return true;
  // Con Shift, `e.key` es el carácter en mayúscula/símbolo y no se puede comparar.
  if (e.shiftKey) return false;
  const base = baseChar(e.code, layout);
  return base !== null && e.key.toLowerCase() !== base.toLowerCase();
}

export type Recorded =
  | { kind: "shortcut"; value: string }
  | { kind: "partial"; label: string }
  | { kind: "needs-modifier" }
  | { kind: "altgr"; char: string };

/** Interpreta una pulsación mientras se graba un atajo. */
export function fromKeyEvent(e: KeyboardEvent, layout: LayoutMap | null = null): Recorded {
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");

  const key = keyName(e.code);
  if (!key) return { kind: "partial", label: format(mods.join("+")) };
  if (!mods.some((m) => m !== "Shift")) return { kind: "needs-modifier" };
  if (isAltGrChar(e, layout)) return { kind: "altgr", char: e.key };
  return { kind: "shortcut", value: [...mods, key].join("+") };
}

const LABELS: Record<string, string> = {
  Space: "Espacio",
  Super: "Win",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Enter: "Intro",
  Backspace: "Retroceso",
  Delete: "Supr",
  Insert: "Insert",
  PageUp: "RePág",
  PageDown: "AvPág",
  Home: "Inicio",
  End: "Fin",
};

/**
 * «Ctrl+Alt+Space» → «Ctrl + Alt + Espacio». Con el mapa de teclado, las
 * teclas de símbolos se muestran como en el teclado del usuario
 * («BracketRight» → «+» en español).
 */
export function format(accel: string, layout: LayoutMap | null = null): string {
  return accel
    .split("+")
    .filter(Boolean)
    .map((p) => LABELS[p] ?? layout?.get(p)?.toUpperCase() ?? p)
    .join(" + ");
}
