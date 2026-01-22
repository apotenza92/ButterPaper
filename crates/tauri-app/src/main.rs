#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use state::AppState;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::pdf::open_pdf,
            commands::pdf::render_page,
            commands::pdf::render_thumbnail,
            commands::pdf::navigate_page,
            commands::pdf::set_zoom,
            commands::pdf::get_page_dimensions,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
