// Fechas de los recordatorios. En la BD son ms Unix UTC; aquí se convierten
// a hora local para mostrarlas y para el <input type="datetime-local">.

const DAY_MS = 86_400_000;
const pad = (n: number) => String(n).padStart(2, "0");

const timeFmt = new Intl.DateTimeFormat("es-ES", { hour: "numeric", minute: "2-digit" });
const weekdayFmt = new Intl.DateTimeFormat("es-ES", { weekday: "short" });
const dateFmt = new Intl.DateTimeFormat("es-ES", { day: "numeric", month: "short" });
const dateYearFmt = new Intl.DateTimeFormat("es-ES", { day: "numeric", month: "short", year: "numeric" });

function startOfDay(ms: number): number {
  const d = new Date(ms);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/** ms → «2026-11-15T18:00» (hora local), para datetime-local. */
export function toLocalInput(ms: number): string {
  const d = new Date(ms);
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** «2026-11-15T18:00» (hora local) → ms, o null si no es válido. */
export function fromLocalInput(value: string): number | null {
  if (!value) return null;
  const ms = new Date(value).getTime();
  return Number.isNaN(ms) ? null : ms;
}

/** «hoy 18:00», «mañana 9:00», «ayer 9:00», «jue 9:00», «17 nov 9:00». */
export function formatReminder(ms: number, now = Date.now()): string {
  // Math.round absorbe los días de 23 o 25 h de los cambios de hora.
  const days = Math.round((startOfDay(ms) - startOfDay(now)) / DAY_MS);
  const time = timeFmt.format(ms);
  if (days === 0) return `hoy ${time}`;
  if (days === 1) return `mañana ${time}`;
  if (days === -1) return `ayer ${time}`;
  if (days > 1 && days < 7) return `${weekdayFmt.format(ms)} ${time}`;
  const sameYear = new Date(ms).getFullYear() === new Date(now).getFullYear();
  return `${(sameYear ? dateFmt : dateYearFmt).format(ms)} ${time}`;
}

function at(base: number, dayOffset: number, hour: number): number {
  const d = new Date(base);
  d.setDate(d.getDate() + dayOffset);
  d.setHours(hour, 0, 0, 0);
  return d.getTime();
}

export interface Preset {
  label: string;
  ms: number;
}

/** Atajos del selector de recordatorio, calculados respecto a `now`. */
export function presets(now = Date.now()): Preset[] {
  const fiveMin = 5 * 60_000;
  const list: Preset[] = [
    { label: "En 1 hora", ms: Math.ceil((now + 3_600_000) / fiveMin) * fiveMin },
  ];
  if (new Date(now).getHours() < 17) list.push({ label: "Esta tarde (18:00)", ms: at(now, 0, 18) });
  const tomorrow = at(now, 1, 9);
  list.push({ label: "Mañana (9:00)", ms: tomorrow });
  const weekday = new Date(now).getDay(); // 0 = domingo
  const toMonday = (1 - weekday + 7) % 7 || 7;
  const monday = at(now, toMonday, 9);
  if (monday !== tomorrow) list.push({ label: "El lunes (9:00)", ms: monday });
  return list;
}
