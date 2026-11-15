# Cordis

Organizador de tareas de escritorio, ligero y solo local. Se abre con un atajo de teclado, como un buscador: escribes la tarea, pulsas Enter y vuelves a lo que estabas haciendo. Vive en la bandeja del sistema y te avisa de los recordatorios con una ventanita que no se cierra sola.

- **Sin cuentas, sin red, sin sincronización.** Tus tareas se guardan en tu equipo y en ningún otro sitio.
- **Ligero.** Unos 4,5 MiB de ejecutable; el instalador de Windows ocupa 2,5 MiB.
- **Windows y Linux** (X11 y Wayland).

## Instalación

### Windows 10 y 11 (64 bits)

1. Descarga `Cordis_x.y.z_x64_es-ES.msi` y ábrelo.
2. Si Windows muestra «Windows protegió su PC», pulsa **Más información** → **Ejecutar de todas formas**. Sale porque el instalador no está firmado.
3. Al terminar, abre Cordis desde el menú Inicio.

No hace falta instalar nada más. Cordis usa WebView2, que ya viene con Windows 11 y con Windows 10 actualizado; si falta, el instalador lo descarga (en ese caso necesita internet).

### Linux

Los paquetes se generan con GitHub Actions: pestaña **Actions** → **Linux** → la última ejecución → artefacto `cordis-linux`.

- **AppImage** (cualquier distribución, sin instalar):
  ```bash
  chmod +x Cordis_*.AppImage
  ./Cordis_*.AppImage
  ```
- **.deb** (Ubuntu, Debian y derivadas):
  ```bash
  sudo apt install ./Cordis_*.deb
  ```

En GNOME, para ver el icono de la bandeja hace falta la extensión **AppIndicator**.

## Uso

### Abrir y cerrar

- **`Ctrl+Shift+Espacio`** abre y cierra la ventana. Se puede cambiar en Ajustes.
- También con un clic en el icono de la bandeja (en Linux, desde su menú: **Mostrar**).
- La ventana se oculta sola al cambiar a otra ventana, al pulsar **Esc** o al cerrarla. Cordis sigue en la bandeja; para salir del todo, menú de la bandeja → **Salir**.

### Tareas

- Escribe y pulsa **Enter** para añadir una tarea.
- Marca la casilla para darla por hecha. Las hechas bajan a la sección **Hechas**, que se puede plegar. Si la desmarcas, vuelve a su sitio.
- **Doble clic**, **Enter** o **F2** sobre el título para editarlo (Enter guarda, Esc cancela).
- Cada tarea tiene estos botones, que aparecen al pasar el ratón (los que están activos se ven siempre):
  - **!** Urgente: la sube arriba, debajo de las otras urgentes, y la resalta.
  - **Campana**: recordatorio.
  - **Líneas**: notas. `Ctrl+Enter` las cierra; `Esc` descarta los cambios.
  - **✕** Borrar (sin confirmación).

  En las tareas hechas solo quedan las notas y borrar.
- **Reordenar**: arrastra desde el asa de la izquierda, o usa **Alt+↑ / Alt+↓**. Las urgentes y las normales no se mezclan.

### Recordatorios

- Con la campana eliges un atajo (en 1 hora, esta tarde, mañana, el lunes) o una fecha y hora concretas.
- Cuando llega la hora aparece un aviso en la esquina inferior derecha que **no se cierra solo** ni te quita el foco del teclado. Desde él puedes:
  - marcar la tarea como **Hecha**;
  - **Posponer** 10 minutos, 1 hora o hasta mañana a las 9:00;
  - **Abrir la lista**, o cerrar el aviso.
- Los recordatorios que vencen con Cordis cerrado se avisan al volver a abrirlo.

### Mover y fijar la ventana

Arrastra la ventana por la franja superior o por el pie. Al moverla queda **fijada**: ya no se oculta al cambiar a otra ventana, para que puedas copiar de otro sitio y pegar en Cordis. La chincheta del pie fija o suelta la ventana a mano. Al volver a abrirla aparece de nuevo centrada.

### Ajustes

El engranaje del pie, o **Ajustes** en el menú de la bandeja:

- **Atajo**: haz clic en él y pulsa la nueva combinación. Tiene que llevar `Ctrl`, `Alt` o `Win`.
- **Arrancar con el sistema**: Cordis se inicia en la bandeja, sin abrir la ventana.
- **Exportar…** guarda todas las tareas en un archivo `.json`.
- **Importar…** carga un archivo exportado. **Sustituye todas las tareas actuales**; antes te dice cuántas trae y pide confirmación.

### Wayland

En Wayland las aplicaciones no pueden registrar atajos globales. Asigna este comando a una tecla en los ajustes de teclado de tu escritorio:

```bash
cordis --toggle
```

## Dónde están tus datos

Una base de datos SQLite, `tareas.db`:

- **Windows**: `%APPDATA%\com.cordis.desktop\`
- **Linux**: `~/.local/share/com.cordis.desktop/`

Para llevarte las tareas a otro equipo, usa **Exportar** e **Importar** en Ajustes.

## Limitaciones conocidas

- **Wayland**: no hay atajos globales (ver arriba); el compositor decide dónde aparecen la ventana y los avisos, y puede no dejarla siempre encima. Si arrastrar la ventana no la fija, usa la chincheta.
- **Linux**: los clics en el icono de la bandeja no llegan a la app; usa su menú.
- **Windows**: con la ventana fijada, el clic en la bandeja no la oculta; usa Esc o el atajo.
- **Windows**: el atajo `Ctrl+Alt+Espacio` lo usa Claude Desktop; por eso el de Cordis es `Ctrl+Shift+Espacio`.

## Desarrollo

Hecho con [Tauri 2](https://tauri.app) (Rust) y Svelte 5 + TypeScript, sin frameworks de interfaz. SQLite desde Rust con `rusqlite`.

### Requisitos

- **Windows**: Node ≥ 20, Rust ≥ 1.90 (rustup, toolchain MSVC), Visual Studio con «Desarrollo para el escritorio con C++».
- **Linux**: Node ≥ 20, Rust ≥ 1.90 y los [prerrequisitos de Tauri 2](https://v2.tauri.app/start/prerequisites/#linux) (`libwebkit2gtk-4.1-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libxdo-dev`…).

### Comandos

```bash
npm install
npm run tauri dev                                  # la app en modo desarrollo
npm run dev                                        # solo la interfaz en el navegador, con datos simulados
npm run check                                      # tipos de TypeScript y Svelte
cargo test --manifest-path src-tauri/Cargo.toml    # tests de Rust
npm run tauri build -- --bundles msi               # instalador de Windows
npm run tauri build -- --bundles appimage,deb      # paquetes de Linux (solo en Linux)
```

El instalador queda en `src-tauri/target/release/bundle/`.

La arquitectura, las decisiones de diseño y las convenciones del código están en [CLAUDE.md](CLAUDE.md).

## Licencia

Cordis se distribuye con licencia [MIT](LICENSE): puedes usarlo, modificarlo y redistribuirlo libremente, siempre que conserves el aviso de copyright. Se ofrece tal cual, sin garantía.

Incluye la tipografía [Lora](https://github.com/cyrealtype/Lora-Cyrillic), de Cyreal, con licencia SIL Open Font License 1.1.
