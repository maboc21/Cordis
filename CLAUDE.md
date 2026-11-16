# CLAUDE.md

Guía para trabajar en este repositorio con Claude Code. El manual de uso, la instalación y la ubicación de los datos están en [README.md](README.md): si cambias algo visible para el usuario (atajos, botones, ajustes, plataformas), actualízalo también.

> El `CLAUDE.md` de la carpeta padre (`Proyectos/`) es de otro proyecto (El Pueblo Duerme) y **no aplica aquí**.

## Qué es

**Cordis** es un organizador de tareas de escritorio, ligero y solo local (sin red, sin cuentas, sin sincronización). Ventana flotante tipo Spotlight que se abre con un atajo global, vive en la bandeja y avisa de los recordatorios con un popup propio que no se cierra solo.

Plataformas: Windows primero; el código debe funcionar también en Linux (X11 y Wayland).

Todo el texto de la interfaz está en **español de España**.

## Comandos

```bash
npm install
npm run tauri dev                  # arranca Vite (puerto 1420) + la app
npm run dev                        # solo la interfaz en el navegador, con backend simulado
npm run check                      # svelte-check (tipos de TS y Svelte)
npm run build                      # solo compila el frontend a dist/
cargo test --manifest-path src-tauri/Cargo.toml   # tests de Rust
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets
npm run tauri build -- --bundles msi              # Windows: binario release + .msi
npm run tauri build -- --bundles appimage,deb     # Linux (solo se puede en Linux)
npm run tauri icon src-tauri/icons/app-icon.svg   # regenerar iconos
```

**CI** (`.github/workflows/build.yml`, workflow «Compilar»). Dos trabajos en paralelo y uno de publicación:
- `windows` (`windows-latest`): `cargo test` y `.msi` → artefacto `cordis-windows`.
- `linux` (`ubuntu-22.04`, glibc antigua para que el `.AppImage` funcione en más distribuciones): prerrequisitos de Tauri, `svelte-check`, `cargo test`, `clippy -D warnings`, `.AppImage` y `.deb` → artefacto `cordis-linux`.
- `release`: solo con una etiqueta `v*`. Junta los artefactos y publica la release con `softprops/action-gh-release` (texto de instalación en el propio workflow).

Se lanza a mano desde Actions (solo artefactos, 30 días) o al subir una etiqueta `vX.Y.Z`, que **tiene que coincidir** con la versión de `package.json`, `Cargo.toml` y `tauri.conf.json` (el trabajo de Windows lo comprueba y falla si no). El artefacto `cordis-linux` pesa ~80 MB: el `.AppImage` incluye WebKitGTK y GTK; el `.deb` es pequeño porque los toma del sistema.

Tras `tauri icon`, borra lo que genera para Android, iOS, macOS y Microsoft Store (`android/`, `ios/`, `icon.icns`, `Square*Logo.png`, `StoreLogo.png`).

Requisitos en Windows: Node ≥ 20, Rust ≥ 1.90 (rustup, toolchain MSVC), Visual Studio con «Desarrollo para el escritorio con C++» y WebView2 (viene con Windows 11).
Requisitos en Linux: ver los prerrequisitos de Tauri 2 (`libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, etc.).

## Arquitectura

```
src/                     Svelte 5 + TypeScript, sin SvelteKit ni librerías de UI
  main.ts                monta App o ReminderPopup según la ventana; en dev y fuera de Tauri
                         instala el backend simulado (el popup se ve en /#reminder)
  App.svelte             entrada + lista desplazable + pie (contadores / errores)
  app.css                estilo «Papel»: @font-face de Lora y variables de color (claro/oscuro
                         con prefers-color-scheme)
  components/
    TaskInput.svelte     campo de nueva tarea (Enter); se enfoca al enfocar la ventana
    TaskList.svelte      pendientes (reordenables) + sección «Hechas» plegable
    TaskItem.svelte      casilla, título editable, notas, borrar, asa de arrastre
    Settings.svelte      vista de Ajustes: grabador del atajo global, aviso de Wayland
    ReminderPicker.svelte  atajos rápidos (en 1 h, esta tarde, mañana, lunes) + datetime-local
    ReminderPopup.svelte   interfaz del popup de recordatorios (ventana `reminder`)
  lib/dates.ts           ms ↔ datetime-local, «hoy 18:00» / «mañana 9:00», presets
  lib/shortcut.ts        KeyboardEvent → «Ctrl+Alt+Space» y su presentación en español
  lib/api.ts             envoltorios tipados de invoke(); la UI solo habla con Rust por aquí
  lib/types.ts           espejo de los structs de Rust (camelCase)
  lib/tasks.svelte.ts    `store`: estado de la lista con runes y métodos que llaman a api
  lib/reorder.ts         acción `use:sortable` (arrastrar con pointer events)
  lib/dev-mock.ts        backend en memoria con mockIPC, solo para `npm run dev` en navegador
src-tauri/
  tauri.conf.json        ventana "main" sin bordes, siempre encima, centrada, fuera de la barra de tareas
  capabilities/          permisos de la ventana (Tauri 2)
  src/main.rs            solo llama a lib::run()
  src/lib.rs             builder: single-instance (primero), global-shortcut, setup (BD, ventana,
                         bandeja, atajo), --toggle / --hidden, comandos
  src/error.rs           Error común; se serializa como texto en español para la UI
  src/commands.rs        #[tauri::command] finos: cogen la conexión, leen el reloj y delegan
  src/window.rs          mostrar/ocultar/toggle, ocultar al perder el foco, cerrar = ocultar
  src/tray.rs            icono de bandeja y menú Mostrar / Ajustes / Salir
  src/shortcut.rs        plugin de atajo global, parse() con validación, registro, is_wayland()
  src/reminders.rs       planificador: plan()/sleep_ms() puros y testeados + bucle tokio
  src/popup.rs           ventana `reminder`: cola de avisos, creación sin foco, esquina inferior derecha
  src/backup.rs          formato JSON de la copia, export/parse/import con tests
  src/db/mod.rs          Db (Mutex<Connection>), open(), pragmas, now_ms()
  src/db/migrations.rs   lista versionada de SQL + PRAGMA user_version
  src/db/tasks.rs        Task, CRUD y reorder(), con sus tests
  src/db/settings.rs     clave/valor con accesores tipados (shortcut)
```

### Copia de seguridad y arranque automático

- **Formato** (`backup.rs`): `{ app: "cordis", version: 1, exportedAt, tasks: [...] }` en camelCase. `parse` acepta también `"organizador-tareas"` (copias de antes del cambio de nombre). Sin ids; el orden del array es el de la lista. Es independiente de la tabla: si cambia el esquema, se adapta `parse`; si cambia el formato, se sube `FORMAT_VERSION`. Una copia de una versión más nueva se rechaza.
- `parse` valida y normaliza (títulos recortados y no vacíos, notas vacías → `null`, `completedAt` solo en las hechas) **sin tocar la BD**. `import` borra y reinserta todo en una transacción.
- Flujo de importar en dos pasos: `preview_import` abre el diálogo, valida y devuelve cuántas tareas trae y cuántas hay; la interfaz pide confirmación; `import_tasks(path)` vuelve a leer, importa y llama a `Scheduler::reschedule()`.
- Los comandos con diálogo son **`async`**: los `blocking_*` del plugin de diálogos se bloquearían en el hilo principal. Los diálogos llevan `set_parent` (si no, quedan detrás de la ventana, que es *always on top*) y se abren con `window::pause_autohide`, un guard que evita que perder el foco oculte la ventana mientras están abiertos.
- **Autoarranque**: plugin `autostart` con `--hidden`. El estado lo guarda el sistema (registro `Run` en Windows, `.desktop` en `~/.config/autostart` en Linux), no la BD. Ojo: activarlo desde `tauri dev` registra la ruta del ejecutable de debug.

### Recordatorios

- `reminders.rs` tiene un bucle propio en `tauri::async_runtime` (tokio). Cada vuelta (`tick`) lee `tasks::pending_reminders`, calcula con `plan(pendientes, now)` qué está vencido y cuál es el siguiente, marca los vencidos con `mark_reminded` **antes** de mostrarlos (mejor perder un aviso que repetirlo), los manda al popup (`popup::push`) y emite `tasks-changed` para que la lista recargue.
- **Avisos en un popup propio, no en notificaciones del sistema** (decisión del usuario: las notificaciones se cierran solas y pasan desapercibidas). Se quitó `tauri-plugin-notification`.
- **Popup** (`popup.rs` + `ReminderPopup.svelte`): ventana `reminder` sin bordes, siempre encima, fuera de la barra de tareas, en la esquina inferior derecha del área de trabajo (`Monitor::work_area`, sin la barra de tareas) del monitor principal. Se crea **sin foco** (`focused(false)`) para no quitarle el teclado al usuario, y se destruye al cerrarse: no existe mientras no hay avisos. Cola de ids en `PopupState`; si llegan avisos con el popup abierto se añaden y se emite `popup-updated`. Cada tarea tiene «Hecha» y «Posponer» (10 min, 1 h, mañana 9:00); abajo, «Abrir la lista»; arriba, cerrar. Se cierra solo cuando la cola se vacía. La interfaz mide su contenido con un `ResizeObserver` y llama a `popup_resize`; Rust limita la altura (120-480 px) y la recoloca para que crezca hacia arriba.
- `set_task_done` y `delete_task` llaman a `popup::refresh`, que quita del popup lo que ya no está pendiente (o lo cierra). Las acciones del popup emiten `tasks-changed` para que la lista principal recargue.
- La ventana `reminder` está en la capability `default` junto a `main` (necesita escuchar eventos).
- `main.ts` lee la etiqueta de la ventana de `__TAURI_INTERNALS__.metadata` en vez de importar `@tauri-apps/api/window`, que añade ~20 kB al bundle.
- Espera con `select!` entre `sleep(sleep_ms(next))` y `Notify`. `sleep_ms` limita el sueño a 60 s porque el reloj de tokio es monótono y no ve cambios de hora ni (según el equipo) la suspensión; sin recordatorios pendientes no despierta nunca.
- **Hay que llamar a `Scheduler::reschedule()`** desde cualquier comando que pueda cambiar el próximo recordatorio (`set_task_reminder`, `set_task_done`, `delete_task`, `import_tasks`). `Notify::notify_one` guarda el permiso si el bucle no está esperando, así que no se pierden avisos.
- La primera vuelta es inmediata al arrancar: avisa en el popup lo que venció con la app cerrada (también con `--hidden`).
- Al desmarcar una tarea hecha cuyo recordatorio ya pasó y no se avisó, se avisa en ese momento.
- La interfaz no deja poner un recordatorio en el pasado (el backend sí lo acepta, para importar).

### Ventana, bandeja y atajo

- La ventana arranca **oculta** (`visible: false`) y `setup` la muestra salvo con `--hidden`. **Si el atajo no se puede registrar** (fuera de Wayland), abre Ajustes aunque haya `--hidden`: sin atajo, la bandeja de Windows suele estar escondida y no habría forma evidente de abrirla. Ajustes muestra el error (`shortcut::Status`).
- `window::show(app, view)` la centra en el monitor del ratón, la enfoca, guarda la vista y emite `show-view` (`"list"` | `"settings"`); la interfaz cambia de vista y recarga la lista. El `show-view` del arranque llega antes de que la interfaz escuche, por eso al cargar pregunta con `current_view`.
- Conflicto conocido: **Claude Desktop usa `Ctrl+Alt+Espacio`** para su acceso rápido. Por eso el valor por defecto es `Ctrl+Shift+Espacio` (decisión del usuario, que cambia la especificación original).
- El grabador de atajos rechaza combinaciones Ctrl+Alt que escriben un carácter con AltGr (teclados europeos: `@`, `#`, `€`, `]`…), comparando `e.key` con el carácter base de la tecla (`navigator.keyboard.getLayoutMap()` si existe).
- Se oculta al perder el foco (salvo con las devtools abiertas, en debug), con Esc (`hide_window`) y al cerrarla (`CloseRequested` → `prevent_close`). Solo «Salir» de la bandeja termina la app.
- **Arrastrar y fijar**: la franja superior (asa), el pie y la cabecera de Ajustes llevan `data-tauri-drag-region` (en Tauri 2 no se hereda: va en cada elemento, no en los botones) y la capability incluye `core:window:allow-start-dragging`. Si el usuario mueve la ventana, queda **fijada**: no se oculta al perder el foco, para poder copiar de otras ventanas y pegar en ella. Se detecta en `WindowEvent::Moved` comparando con la posición en la que la puso `show` (`placed_at`): un movimiento distinto es del usuario. La chincheta del pie fija o suelta a mano (`set_pinned`, evento `pinned-changed`). `show` y `hide` la sueltan: siempre se vuelve a abrir centrada, la posición no se guarda (decisión del usuario).
- `window::toggle` (atajo, clic izquierdo en la bandeja, `--toggle`): si la ventana se acaba de ocultar por perder el foco (< 350 ms), no hace nada. Si está visible sin foco (fijada, con el usuario en otra ventana), solo le devuelve el foco, sin moverla. Pulsar la bandeja primero quita el foco y luego llega el clic; sin esto, la ventana se ocultaría y volvería a salir.
- **Single-instance** tiene que ser el primer plugin. Una segunda instancia pasa sus argumentos a la primera y termina: con `--toggle` la conmuta; sin argumentos, la muestra.
- **Atajo**: por defecto `Ctrl+Shift+Space` (`settings::DEFAULT_SHORTCUT`; la interfaz lo recibe en `defaultShortcut`). Se guarda en `settings.shortcut` con el formato del plugin. `shortcut::parse` exige Ctrl, Alt o Super. `set_shortcut` registra primero y guarda solo si funciona; si falla, reactiva el anterior. Mientras la interfaz graba uno nuevo, `suspend_shortcut` lo desactiva para que pulsarlo no oculte la ventana. Un fallo al registrar al arrancar no impide arrancar.
- Los comandos son síncronos (hilo principal), que es lo que necesita el registro de atajos en Windows.

### Interfaz

- **Estado**: todo pasa por `store` (`lib/tasks.svelte.ts`). Los componentes no llaman a `api` directamente. Si un comando falla, `store.fail()` muestra el error en el pie durante 5 s; si el cambio era optimista (borrar, reordenar), se recarga la lista.
- **Reordenar** (`lib/reorder.ts`): hijos directos del contenedor con `data-id` y un asa `data-drag-handle`. Durante el gesto solo se usan `transform`; al soltar, `onReorder(ids)` debe actualizar el estado **de forma síncrona** (antes de cualquier `await`), porque la acción espera a `tick()` y luego limpia los transforms sobre el DOM ya reordenado. El elemento arrastrado no sale de los límites de la lista. Esc cancela. Autoscroll cerca de los bordes del área desplazable.
- **Teclado**: Enter añade; doble clic, Enter o F2 sobre un título lo edita (Enter guarda, Esc cancela, vaciarlo descarta la edición); Alt+↑/↓ mueve una pendiente; en notas, Ctrl+Enter cierra y Esc descarta. Los Esc internos hacen `stopPropagation` para que en la fase 4 no oculten la ventana.
- **Sección «Hechas»**: ordenada por `completedAt` descendente, no se reordena. Si está plegada o no se guarda en `localStorage` (preferencia visual, no dato).
- **CSP**: `default-src 'self'` bloquea imágenes `data:`; los iconos son SVG inline y la marca de la casilla es CSS.
- **Backend simulado** (`lib/dev-mock.ts`): imita las reglas de `db/tasks.rs`. Si cambias esas reglas, cámbialo también. Ojo: en el panel del navegador de Claude, si está oculto, `requestAnimationFrame` no corre y el autoscroll no se puede probar ahí.

### Base de datos

- Fichero: `<app_data_dir>/tareas.db`. En Windows, `%APPDATA%\com.cordis.desktop\tareas.db`.
- Migraciones en `db/migrations.rs`: cada entrada lleva de la versión `i` a la `i + 1` en su propia transacción. **Nunca edites una migración publicada**; añade otra al final. Si la BD es más nueva que la app, se niega a abrirla (`Error::DbTooNew`).
- Las funciones de `db::tasks` reciben `&Connection` y `now` como parámetros (no leen el reloj) para que los tests sean deterministas. Los tests usan `db::open_in_memory()`.
- Comandos de tareas: `list_tasks`, `create_task`, `rename_task`, `set_task_notes`, `set_task_urgent`, `set_task_done`, `set_task_reminder`, `delete_task`, `reorder_tasks` (y `app_version`). Los comandos son síncronos: Tauri los ejecuta en el hilo principal, lo cual está bien porque cada consulta tarda microsegundos.

## Decisiones tomadas

- **Estilo visual «Papel»** (elegido por el usuario entre seis propuestas): libreta de notas con fondo crema, tipografía **Lora** (serif), tinta azul (`--accent`), un margen rojo vertical en la lista (`--margin-line`, dibujado como fondo de `.scroll`; por eso las filas son transparentes y solo la arrastrada lleva fondo) y las urgentes marcadas con fosforito (`--urgent-bg`). El campo de nueva tarea es solo una línea de tinta. De noche, «papel oscuro» con tinta clara. Todos los colores son variables de `app.css`; los componentes no llevan colores fijos.
- **Lora va dentro de la app** (paquete `@fontsource/lora`, licencia OFL 1.1). Se declara a mano en `app.css` apuntando solo a los `.woff2` latinos de 400 y 600 (+43 KB); los CSS del paquete añadirían copias `.woff` que no hacen falta. Si se necesita otro peso o la cursiva, añadir su `@font-face` igual.
- **Ligereza ante todo.** Nada de frameworks de UI ni dependencias pesadas. Perfil release con `opt-level="s"`, LTO, `codegen-units=1`, `panic="abort"` y `strip`.
- **TypeScript 6**, no 7: `svelte-check` 4 todavía no admite TS 7.
- **SQLite desde Rust** con `rusqlite` (feature `bundled`, sin depender de la librería del sistema). Una única `Mutex<Connection>` compartida por comandos y planificador, con WAL.
- **Migraciones** propias con `PRAGMA user_version` (lista de SQL versionada), sin crate extra.
- **Fechas** como `INTEGER` en ms Unix UTC. Rust solo compara números, así que no se usa `chrono`. La interfaz convierte a hora local.
- **Urgentes** (migración 2, columna `urgent`): la lista se ordena por `urgent DESC, position` (en Rust, `tasks::ORDER`; en la interfaz, `byOrder`). `set_urgent` recoloca la tarea: al marcarla queda la última de las urgentes (`MAX(position) + 1`) y al desmarcarla, la primera de las normales (`MIN(position) - 1`; las posiciones pueden ser negativas). `reorder` separa urgentes y normales conservando el orden dentro de cada grupo, así que arrastrar no puede mezclarlas. Se destacan con franja naranja, fondo suave y título en negrita (`--urgent*` en `app.css`). En las hechas no se destacan. La copia de seguridad incluye `urgent` (con `serde(default)`, así que las copias antiguas siguen valiendo sin subir `FORMAT_VERSION`).
- **`position`** entera. Las tareas nuevas van al final (`MAX + 1`). `reorder(ids)` reparte entre esas tareas las mismas posiciones que ya ocupaban, en el nuevo orden: permite reordenar solo las pendientes sin mover las hechas, y es todo o nada (ids repetidos o inexistentes no cambian nada). Puede haber huecos tras borrar; no importa.
- **Marcar como hecha** guarda `completed_at` (sin pisarlo si ya lo estaba); desmarcar lo borra. La tarea conserva su `position`, así que al desmarcarla vuelve a su sitio.
- **Cambiar `remind_at`** siempre pone `reminded = 0`.
- **`rust-version = 1.90`**: lo exigen `rusqlite` 0.43 y los plugins de atajo global e instancia única.
- **Esc**: en Ajustes vuelve a la lista; en la lista oculta la ventana. Los Esc con significado propio (editar, notas, arrastrar, grabar atajo) hacen `preventDefault` + `stopPropagation`.
- **Arrastrar para reordenar** con pointer events hechos a mano (el drag & drop HTML5 choca con `dragDropEnabled` de Tauri en Windows y falla en WebKitGTK).
- **Ajustes** en una tabla clave/valor `settings`, y en la misma ventana como otra vista (no una segunda ventana, para ahorrar RAM).
- **Planificador de recordatorios**: tarea tokio (runtime de Tauri) con `select!` entre `sleep(próximo)` y un `Notify` que despiertan los comandos que cambian tareas. `plan()` es pura y tiene tests. Los avisos van al popup (ver «Recordatorios»).
- **Autoarranque** con `--hidden`: arranca en la bandeja sin mostrar la ventana.
- **Exportar/importar** con `tauri-plugin-dialog`, llamado desde Rust. Importar **reemplaza** todas las tareas, previa confirmación.
- **Instalador de Windows** en español (`bundle.windows.wix.language = es-ES`).
- **Tareas hechas** bajan a una sección «Hechas» plegable al final.
- **Wayland**: no hay atajos globales; el ejecutable acepta `--toggle` y, por single-instance, conmuta la ventana de la instancia abierta.
- **Git**: solo se hace commit cuando el usuario lo pide.
- **Nombre: Cordis** (elegido por el usuario). Es el `productName`, el título, el tooltip de la bandeja y el ejecutable (`cordis`, librería `cordis_lib`). El `identifier` es `com.cordis.desktop` (antes `com.organizadortareas.desktop`; se cambió cuando la BD era solo de pruebas). De él dependen la carpeta de datos y la clave de single-instance: **cambiarlo deja atrás la BD**. El `upgradeCode` de WiX está fijado al del primer `.msi` («Organizador de tareas») para que el instalador nuevo lo actualice en vez de instalarse al lado; no lo cambies.

## Limitaciones conocidas

- Wayland: el compositor no deja centrar la ventana ni ponerla siempre encima, y no hay atajos globales (Ajustes muestra el comando `--toggle`).
- GNOME: la bandeja necesita la extensión AppIndicator.
- Linux: los clics en el icono de la bandeja no llegan a la app (AppIndicator); solo funciona el menú.
- Windows: si la ventana se muestra desde otro proceso (`--toggle`), Windows puede negarle el foco (protección contra robo de foco). Con el atajo y la bandeja no pasa.
- Por comprobar en la app real: si el desplegable nativo del `datetime-local` quita el foco a la ventana, se ocultaría al abrirlo (los diálogos de archivo ya pausan el ocultado).
- Wayland: el compositor no deja colocar el popup en la esquina; aparecerá donde decida. Tampoco informa de la posición de la ventana, así que arrastrarla puede no fijarla: hay que usar la chincheta.
- Windows: con la ventana fijada, el clic en el icono de la bandeja no la oculta (al pulsarlo ya ha perdido el foco); se oculta con Esc o con el atajo.
- `.AppImage` y `.deb` se generan en Linux; no hay compilación cruzada desde Windows. Para eso está el workflow de GitHub Actions.
- El `.msi` no está firmado: Windows SmartScreen avisa al abrirlo («Windows protegió su PC»).

## Forma de trabajar

- Fases pequeñas. Al terminar cada una: explicar cómo probarla y parar hasta que el usuario confirme.
- Tests para la lógica de Rust: CRUD, reordenación y cálculo del próximo recordatorio (`db::tasks`, `reminders`, `shortcut`, `db::settings`, `db::migrations`).
- Antes de una decisión de arquitectura no cubierta aquí, preguntar.
- Mantener este archivo al día.

## Estado

| Fase | Contenido | Estado |
|------|-----------|--------|
| 1 | Esqueleto (Vite + Svelte + TS + Tauri 2, ventana sin bordes) | Hecha |
| 2 | Base de datos, migraciones y CRUD con tests | Hecha |
| 3 | Interfaz y reordenación | Hecha |
| 4 | Bandeja, atajo global, single-instance y `--toggle` | Hecha |
| 5 | Recordatorios | Hecha |
| 6 | Autoarranque, exportar/importar y empaquetado | Hecha. `.msi` en Windows; `.AppImage`/`.deb` en CI (primera ejecución en verde, ~14 min). Sin probar todavía en un Linux real |

Medidas del build de release en Windows (v0.1.0): ejecutable 4,6 MiB, `.msi` 2,5 MiB. Compilación release completa desde cero: ~3 min.
