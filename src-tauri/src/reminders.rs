//! Planificador de recordatorios.
//!
//! Un bucle propio en tokio: calcula el próximo recordatorio, duerme hasta
//! entonces y lo muestra en el popup (`popup.rs`), que no se cierra solo.
//! Se recalcula cuando cambian las tareas (`Scheduler::reschedule`). Los que
//! vencieron con la app cerrada se avisan en la primera vuelta, al arrancar.
//!
//! La lógica (`plan` y `sleep_ms`) es pura y está testeada; `tick` solo la
//! conecta con la base de datos y el popup.

use std::sync::Arc;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::Notify;

use crate::db::{self, tasks, tasks::Task, Db};
use crate::popup;

/// Como mucho se duerme esto seguido aunque el próximo aviso esté más lejos.
/// El reloj de tokio es monótono: no ve cambios de hora del sistema ni, en
/// algunos equipos, el tiempo en suspensión. Despertar cada minuto y volver a
/// mirar el reloj de pared lo corrige con un coste despreciable. Sin
/// recordatorios pendientes no se despierta nunca.
const MAX_SLEEP_MS: i64 = 60_000;

// --- Lógica pura ---

#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    /// Recordatorios vencidos que hay que avisar ya, del más antiguo al más nuevo.
    pub due: Vec<Task>,
    /// Instante (ms) del siguiente recordatorio futuro, si hay alguno.
    pub next: Option<i64>,
}

/// Reparte los recordatorios pendientes entre «avisar ya» y «el siguiente».
/// Ignora por su cuenta las tareas hechas, ya avisadas o sin fecha, aunque la
/// consulta de la BD ya las filtre.
pub fn plan(pending: &[Task], now: i64) -> Plan {
    let mut due: Vec<Task> = Vec::new();
    let mut next: Option<i64> = None;
    for t in pending.iter().filter(|t| !t.done && !t.reminded) {
        let Some(at) = t.remind_at else { continue };
        if at <= now {
            due.push(t.clone());
        } else {
            next = Some(next.map_or(at, |n| n.min(at)));
        }
    }
    due.sort_by_key(|t| (t.remind_at, t.id));
    Plan { due, next }
}

/// Cuánto dormir hasta la próxima vuelta (`None`: hasta que cambien las tareas).
pub fn sleep_ms(next: Option<i64>, now: i64) -> Option<u64> {
    next.map(|at| (at - now).clamp(0, MAX_SLEEP_MS) as u64)
}

// --- Bucle ---

/// Estado gestionado: despierta al planificador para que recalcule.
pub struct Scheduler(Arc<Notify>);

impl Scheduler {
    pub fn reschedule(&self) {
        // Si el bucle no está esperando, el permiso queda guardado y la
        // siguiente espera vuelve al momento: no se pierden avisos.
        self.0.notify_one();
    }
}

pub fn start(app: &AppHandle) {
    let notify = Arc::new(Notify::new());
    app.manage(Scheduler(notify.clone()));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            match tick(&app) {
                Some(ms) => {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_millis(ms)) => {}
                        _ = notify.notified() => {}
                    }
                }
                None => notify.notified().await,
            }
        }
    });
}

/// Una vuelta: avisa lo vencido, lo marca y devuelve cuánto dormir.
fn tick(app: &AppHandle) -> Option<u64> {
    let now = db::now_ms();
    let db = app.state::<Db>();
    let plan = {
        let mut conn = db.lock();
        let pending = match tasks::pending_reminders(&conn) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Recordatorios: {e}");
                return Some(MAX_SLEEP_MS as u64);
            }
        };
        let plan = plan(&pending, now);
        let ids: Vec<i64> = plan.due.iter().map(|t| t.id).collect();
        // Se marcan antes de mostrarlos: si algo falla, mejor perder un aviso
        // que repetirlo en bucle cada minuto.
        if let Err(e) = tasks::mark_reminded(&mut conn, &ids) {
            eprintln!("Recordatorios: {e}");
        }
        plan
    };

    if !plan.due.is_empty() {
        let ids: Vec<i64> = plan.due.iter().map(|t| t.id).collect();
        popup::push(app, &ids);
        let _ = app.emit("tasks-changed", ());
    }
    sleep_ms(plan.next, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_700_000_000_000;

    fn task(id: i64, remind_at: Option<i64>) -> Task {
        Task {
            id,
            title: format!("t{id}"),
            notes: None,
            position: id,
            done: false,
            created_at: NOW - 1_000_000,
            completed_at: None,
            remind_at,
            reminded: false,
            urgent: false,
        }
    }

    fn ids(ts: &[Task]) -> Vec<i64> {
        ts.iter().map(|t| t.id).collect()
    }

    // --- plan ---

    #[test]
    fn sin_recordatorios_no_hay_nada_que_hacer() {
        assert_eq!(plan(&[], NOW), Plan { due: vec![], next: None });
        assert_eq!(plan(&[task(1, None)], NOW), Plan { due: vec![], next: None });
    }

    #[test]
    fn separa_vencidos_y_calcula_el_siguiente() {
        let p = plan(
            &[task(1, Some(NOW + 5_000)), task(2, Some(NOW - 10)), task(3, Some(NOW + 1_000))],
            NOW,
        );
        assert_eq!(ids(&p.due), [2]);
        assert_eq!(p.next, Some(NOW + 1_000));
    }

    #[test]
    fn el_que_vence_justo_ahora_se_avisa() {
        let p = plan(&[task(1, Some(NOW))], NOW);
        assert_eq!(ids(&p.due), [1]);
        assert_eq!(p.next, None);
    }

    #[test]
    fn los_vencidos_salen_del_mas_antiguo_al_mas_nuevo() {
        let p = plan(
            &[task(1, Some(NOW - 10)), task(2, Some(NOW - 3_000)), task(3, Some(NOW - 10))],
            NOW,
        );
        assert_eq!(ids(&p.due), [2, 1, 3]);
    }

    #[test]
    fn ignora_hechas_y_ya_avisadas() {
        let mut hecha = task(1, Some(NOW - 10));
        hecha.done = true;
        let mut avisada = task(2, Some(NOW - 10));
        avisada.reminded = true;
        let mut futura_hecha = task(3, Some(NOW + 10));
        futura_hecha.done = true;
        assert_eq!(plan(&[hecha, avisada, futura_hecha], NOW), Plan { due: vec![], next: None });
    }

    #[test]
    fn vencidos_de_hace_dias_se_avisan_al_arrancar() {
        let semana = 7 * 24 * 3_600_000;
        let p = plan(&[task(1, Some(NOW - semana)), task(2, Some(NOW + semana))], NOW);
        assert_eq!(ids(&p.due), [1]);
        assert_eq!(p.next, Some(NOW + semana));
    }

    // --- sleep_ms ---

    #[test]
    fn duerme_hasta_el_siguiente_con_un_maximo() {
        assert_eq!(sleep_ms(None, NOW), None);
        assert_eq!(sleep_ms(Some(NOW + 1_500), NOW), Some(1_500));
        assert_eq!(sleep_ms(Some(NOW + 3_600_000), NOW), Some(MAX_SLEEP_MS as u64));
        // Si el reloj ya lo ha pasado (no debería), no hay sueño negativo.
        assert_eq!(sleep_ms(Some(NOW - 5), NOW), Some(0));
    }
}
