// En release no abre una consola aparte en Windows. ¡No quitar!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    organizador_tareas_lib::run()
}
