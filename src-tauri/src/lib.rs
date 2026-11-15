pub mod commands;
pub mod state;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();

    tauri::Builder::default()
        .manage(state)
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone());
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_history,
            commands::get_favorites,
            commands::toggle_favorite,
            commands::is_favorite,
            commands::remove_history,
            commands::set_manga_status,
            commands::record_history,
            commands::get_reading_progress,
            commands::save_reading_progress,
            commands::get_settings,
            commands::save_settings,
            commands::get_sources,
            commands::fetch_source_page,
            commands::search_global,
            commands::get_manga_details,
            commands::get_chapters,
            commands::get_pages,
            commands::fetch_image_base64,
            commands::get_downloads,
            commands::queue_download,
            commands::cancel_download,
            commands::open_cloudflare_solver,
            commands::save_source_cookies,
            commands::clear_source_cookies,
            commands::get_source_cookies,
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
