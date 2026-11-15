// Reordenar arrastrando, con pointer events y sin dependencias.
//
// No usamos drag & drop HTML5: en Windows choca con el `dragDropEnabled` de
// Tauri y en WebKitGTK (Linux) es poco fiable.
//
// Uso: `<div use:sortable={{ onReorder }}>` cuyos hijos directos llevan
// `data-id` y, dentro, un asa con `data-drag-handle`. Mientras se arrastra solo
// se mueven elementos con `transform`; al soltar se llama a `onReorder(ids)`,
// que debe actualizar el estado de forma síncrona, y tras `tick()` se limpian
// los transforms sobre el DOM ya reordenado por Svelte.
//
// CSS esperado: el contenedor recibe `.sorting` y el elemento arrastrado
// `.dragging` mientras dura el gesto.
import { tick } from "svelte";
import type { Action } from "svelte/action";

export interface SortableParams {
  onReorder: (ids: number[]) => void;
}

interface Item {
  el: HTMLElement;
  id: number;
  /** Centro vertical en coordenadas del contenido desplazable. */
  mid: number;
}

interface Drag {
  handle: HTMLElement;
  pointerId: number;
  el: HTMLElement;
  items: Item[];
  from: number;
  to: number;
  /** Cuánto se desplazan los demás para hacer hueco. */
  shift: number;
  /**
   * Límites del desplazamiento del elemento arrastrado: no sale de la lista.
   * Sin ellos, su `transform` ampliaría el área desplazable y el autoscroll
   * seguiría bajando hacia el vacío.
   */
  minDy: number;
  maxDy: number;
  startY: number;
  lastY: number;
  scroller: HTMLElement;
  startScroll: number;
  raf: number;
}

/** Zona (px) junto a los bordes del área desplazable que activa el autoscroll. */
const EDGE = 40;
const MAX_SCROLL_SPEED = 14;

function scrollParent(el: HTMLElement): HTMLElement {
  for (let p = el.parentElement; p; p = p.parentElement) {
    const { overflowY } = getComputedStyle(p);
    if (overflowY === "auto" || overflowY === "scroll") return p;
  }
  return document.scrollingElement as HTMLElement;
}

function viewportRect(scroller: HTMLElement) {
  return scroller === document.scrollingElement
    ? { top: 0, bottom: window.innerHeight }
    : scroller.getBoundingClientRect();
}

export const sortable: Action<HTMLElement, SortableParams> = (container, params) => {
  let opts = params;
  let drag: Drag | null = null;

  function onPointerDown(e: PointerEvent) {
    if (drag || e.button !== 0) return;
    const handle = (e.target as Element).closest<HTMLElement>("[data-drag-handle]");
    const el = handle?.closest<HTMLElement>("[data-id]");
    if (!handle || !el || el.parentElement !== container) return;
    e.preventDefault();

    const scroller = scrollParent(container);
    const st = scroller.scrollTop;
    const els = Array.from(container.children).filter(
      (c): c is HTMLElement => c instanceof HTMLElement && c.dataset.id !== undefined,
    );
    const items = els.map((c) => {
      const r = c.getBoundingClientRect();
      return { el: c, id: Number(c.dataset.id), mid: r.top + st + r.height / 2 };
    });
    const from = els.indexOf(el);
    const gap = parseFloat(getComputedStyle(container).rowGap) || 0;
    const rect = el.getBoundingClientRect();
    const first = els[0].getBoundingClientRect();
    const last = els[els.length - 1].getBoundingClientRect();

    try {
      handle.setPointerCapture(e.pointerId);
    } catch {
      // Eventos sintéticos (tests) no tienen puntero activo; sin captura también funciona.
    }
    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
    handle.addEventListener("pointercancel", onCancel);
    window.addEventListener("keydown", onKey, true);

    drag = {
      handle,
      pointerId: e.pointerId,
      el,
      items,
      from,
      to: from,
      shift: rect.height + gap,
      minDy: first.top - rect.top,
      maxDy: last.bottom - rect.bottom,
      startY: e.clientY,
      lastY: e.clientY,
      scroller,
      startScroll: st,
      raf: requestAnimationFrame(autoScroll),
    };
    container.classList.add("sorting");
    el.classList.add("dragging");
  }

  function layout() {
    const d = drag!;
    const raw = d.lastY - d.startY + (d.scroller.scrollTop - d.startScroll);
    const dy = Math.min(d.maxDy, Math.max(d.minDy, raw));
    d.el.style.transform = `translateY(${dy}px)`;

    const center = d.items[d.from].mid + dy;
    // Nueva posición = cuántos elementos quedan por encima del centro. Con `<=`
    // para los de abajo: en el tope (dy = maxDy) el centro coincide con el del
    // último y debe contar, igual que arriba (dy = minDy) el primero no cuenta.
    let to = 0;
    d.items.forEach((it, i) => {
      if (i < d.from ? it.mid < center : i > d.from && it.mid <= center) to++;
    });
    d.to = to;

    d.items.forEach((it, i) => {
      if (i === d.from) return;
      let s = 0;
      if (d.from < to && i > d.from && i <= to) s = -d.shift;
      else if (d.from > to && i >= to && i < d.from) s = d.shift;
      it.el.style.transform = s ? `translateY(${s}px)` : "";
    });
  }

  function autoScroll() {
    const d = drag;
    if (!d) return;
    const r = viewportRect(d.scroller);
    let v = 0;
    if (d.lastY < r.top + EDGE) v = -(r.top + EDGE - d.lastY);
    else if (d.lastY > r.bottom - EDGE) v = d.lastY - (r.bottom - EDGE);
    if (v) {
      const speed = Math.sign(v) * Math.min(MAX_SCROLL_SPEED, Math.ceil((Math.abs(v) / EDGE) * MAX_SCROLL_SPEED));
      const before = d.scroller.scrollTop;
      d.scroller.scrollTop += speed;
      if (d.scroller.scrollTop !== before) layout();
    }
    d.raf = requestAnimationFrame(autoScroll);
  }

  function onMove(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.pointerId) return;
    drag.lastY = e.clientY;
    layout();
  }

  function onUp(e: PointerEvent) {
    if (drag && e.pointerId === drag.pointerId) void end(true);
  }

  function onCancel() {
    void end(false);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    // Que Esc cancele el arrastre y no llegue a nadie más (p. ej. ocultar la ventana).
    e.preventDefault();
    e.stopPropagation();
    void end(false);
  }

  async function end(commit: boolean) {
    const d = drag;
    if (!d) return;
    drag = null;
    cancelAnimationFrame(d.raf);
    d.handle.removeEventListener("pointermove", onMove);
    d.handle.removeEventListener("pointerup", onUp);
    d.handle.removeEventListener("pointercancel", onCancel);
    window.removeEventListener("keydown", onKey, true);
    if (d.handle.hasPointerCapture(d.pointerId)) d.handle.releasePointerCapture(d.pointerId);

    if (commit && d.to !== d.from) {
      const ids = d.items.map((it) => it.id);
      ids.splice(d.to, 0, ...ids.splice(d.from, 1));
      opts.onReorder(ids);
      await tick();
    }
    // Al quitar `.sorting` desaparece la transición, así que nada «vuelve» animado.
    container.classList.remove("sorting");
    d.el.classList.remove("dragging");
    for (const it of d.items) it.el.style.transform = "";
  }

  container.addEventListener("pointerdown", onPointerDown);

  return {
    update(p: SortableParams) {
      opts = p;
    },
    destroy() {
      container.removeEventListener("pointerdown", onPointerDown);
      void end(false);
    },
  };
};
