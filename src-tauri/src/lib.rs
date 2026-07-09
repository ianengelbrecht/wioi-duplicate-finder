// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod commands;
pub mod db;
pub mod models;
pub mod parsers;
pub mod repositories;
pub mod services;

use tauri::Manager;
use db::shutdown_database;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            println!("Second instance attempted to start");
            println!("Args: {:?}", argv);
            println!("CWD: {:?}", cwd);

            // Bring the existing window to the front
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("app".to_string()),
                    }),
                ])
                .build(),
        )
        .setup(|_app| Ok(()))
        .invoke_handler(tauri::generate_handler![
            commands::initialize_database,
            commands::register_user,
            commands::login_user,
            commands::get_user_by_id,
            commands::get_all_users,
            commands::update_user_profile,
            commands::update_user_by_admin,
            commands::create_session,
            commands::get_sessions,
            commands::search_reference,
            commands::autocomplete_scientific_name,
            commands::lookup_taxon_by_name,
            commands::autocomplete_recorded_by,
            commands::autocomplete_agent,
            commands::check_agent_exists,
            commands::add_agent,
            commands::autocomplete_locality,
            commands::save_captured_record,
            commands::get_captured_records,
            commands::delete_captured_record,
            commands::delete_session,
            commands::rename_session,
            commands::select_export_path,
            commands::save_export_settings,
            commands::get_export_settings,
            commands::export_session_csv,
            commands::autocomplete_geography,
            commands::get_table_counts,
            commands::resolve_wcvp_families,
            commands::get_default_backup_dir,
            commands::select_backup_directory,
            commands::perform_manual_backup,
            commands::select_backup_file,
            commands::restore_database_from_backup,
            commands::select_csv_file,
            commands::get_reference_metadata,
            commands::import_reference_dataset,
            commands::get_wcvp_metadata,
            commands::import_wcvp_dataset,
            commands::select_database_file,
            commands::configure_database
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            shutdown_database(app_handle);
        }
    });
}
