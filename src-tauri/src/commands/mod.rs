use tauri::AppHandle;

use crate::db;
use crate::models::{
    CapturedRecord, ExportSettingsDto, ReferenceSpecimen, SessionDto, TaxonAutocompleteResult,
    UserDto,
};
use crate::services::{
    AgentService, AuthService, ExportService, GeographyService, ReferenceService, SessionService,
    SpecimenService, TaxonomyService,
};

#[tauri::command]
pub fn initialize_database(app: AppHandle) -> Result<(), String> {
    db::init_database(&app)
}

#[tauri::command]
pub fn register_user(app: AppHandle, username: String, password: String) -> Result<String, String> {
    AuthService::register_user(&app, &username, &password)
}

#[tauri::command]
pub fn login_user(
    app: AppHandle,
    username: String,
    password: String,
) -> Result<Option<UserDto>, String> {
    AuthService::login_user(&app, &username, &password)
}

#[tauri::command]
pub fn create_session(app: AppHandle, user_id: i32, name: String) -> Result<SessionDto, String> {
    SessionService::create_session(&app, user_id, &name)
}

#[tauri::command]
pub fn get_sessions(app: AppHandle, user_id: i32) -> Result<Vec<SessionDto>, String> {
    SessionService::get_sessions(&app, user_id)
}

#[tauri::command]
pub fn rename_session(app: AppHandle, id: i32, name: String) -> Result<(), String> {
    SessionService::rename_session(&app, id, &name)
}

#[tauri::command]
pub fn delete_session(app: AppHandle, id: i32) -> Result<(), String> {
    SessionService::delete_session(&app, id)
}

#[tauri::command]
pub fn get_captured_records(
    app: AppHandle,
    session_id: i32,
) -> Result<Vec<CapturedRecord>, String> {
    SpecimenService::get_captured_records(&app, session_id)
}

#[tauri::command]
pub fn save_captured_record(
    app: AppHandle,
    record: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let captured_record: CapturedRecord =
        serde_json::from_value(record).map_err(|e| format!("Invalid record format: {}", e))?;
    SpecimenService::save_captured_record(&app, captured_record)
}

#[tauri::command]
pub fn delete_captured_record(app: AppHandle, id: i32) -> Result<(), String> {
    SpecimenService::delete_captured_record(&app, id)
}

#[tauri::command]
pub fn search_reference(
    app: AppHandle,
    filters: serde_json::Value,
) -> Result<Vec<ReferenceSpecimen>, String> {
    TaxonomyService::search_reference(&app, filters)
}

#[tauri::command]
pub fn autocomplete_scientific_name(
    app: AppHandle,
    query: String,
) -> Result<Vec<TaxonAutocompleteResult>, String> {
    TaxonomyService::autocomplete_scientific_name(&app, &query)
}

#[tauri::command]
pub fn resolve_wcvp_families(
    app: AppHandle,
    queries: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    TaxonomyService::resolve_wcvp_families(&app, queries)
}

#[tauri::command]
pub fn autocomplete_recorded_by(app: AppHandle, query: String) -> Result<Vec<String>, String> {
    AgentService::autocomplete_recorded_by(&app, &query)
}

#[tauri::command]
pub fn autocomplete_agent(app: AppHandle, query: String) -> Result<Vec<String>, String> {
    AgentService::autocomplete_agent(&app, &query)
}

#[tauri::command]
pub fn check_agent_exists(app: AppHandle, name: String) -> Result<bool, String> {
    AgentService::check_agent_exists(&app, &name)
}

#[tauri::command]
pub fn add_agent(app: AppHandle, name: String) -> Result<(), String> {
    AgentService::add_agent(&app, &name)
}

#[tauri::command]
pub fn autocomplete_locality(app: AppHandle, query: String) -> Result<Vec<String>, String> {
    GeographyService::autocomplete_locality(&app, &query)
}

#[tauri::command]
pub fn autocomplete_geography(
    app: AppHandle,
    field: String,
    query: String,
    country: String,
    state_province: String,
    county: String,
) -> Result<Vec<String>, String> {
    GeographyService::autocomplete_geography(
        &app,
        &field,
        &query,
        &country,
        &state_province,
        &county,
    )
}

#[tauri::command]
pub fn get_table_counts(app: AppHandle) -> Result<serde_json::Value, String> {
    GeographyService::get_table_counts(&app)
}

#[tauri::command]
pub fn save_export_settings(
    app: AppHandle,
    user_id: i32,
    format: String,
    mappings: String,
) -> Result<(), String> {
    ExportService::save_export_settings(&app, user_id, &format, &mappings)
}

#[tauri::command]
pub fn get_export_settings(app: AppHandle, user_id: i32) -> Result<ExportSettingsDto, String> {
    ExportService::get_export_settings(&app, user_id)
}

#[tauri::command]
pub fn export_session_csv(
    app: AppHandle,
    session_id: i32,
    filepath: String,
    csv_content: String,
) -> Result<String, String> {
    ExportService::export_session_csv(&app, session_id, &filepath, &csv_content)
}

#[tauri::command]
pub fn select_export_path(default_name: String) -> Option<String> {
    let file = rfd::FileDialog::new()
        .set_file_name(&default_name)
        .add_filter("CSV File", &["csv"])
        .save_file();

    file.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_default_backup_dir(app: AppHandle) -> Result<String, String> {
    let db_path = db::get_db_path(&app).ok_or_else(|| "Database is not configured.".to_string())?;
    let app_dir = db_path
        .parent()
        .ok_or("Failed to get app directory parent")?;
    let backups_dir = app_dir.join("backups");
    Ok(backups_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn select_backup_directory() -> Option<String> {
    let dir = rfd::FileDialog::new().pick_folder();

    dir.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn perform_manual_backup(app: AppHandle) -> Result<String, String> {
    let db_path = db::get_db_path(&app).ok_or_else(|| "Database is not configured.".to_string())?;
    if !db_path.exists() {
        return Err("Database file does not exist.".to_string());
    }

    let mut custom_backups_dir = None;
    if let Ok(conn) = db::get_connection(&app) {
        if let Ok(mappings_str) = conn.query_row(
            "SELECT mappings FROM export_settings LIMIT 1",
            rusqlite::params![],
            |row| row.get::<_, String>(0),
        ) {
            if let Ok(mappings) = serde_json::from_str::<serde_json::Value>(&mappings_str) {
                if let Some(custom_path) = mappings.get("backupLocation").and_then(|v| v.as_str()) {
                    let trim_path = custom_path.trim();
                    if !trim_path.is_empty() {
                        custom_backups_dir = Some(std::path::PathBuf::from(trim_path));
                    }
                }
            }
        }
    }

    let backups_dir = custom_backups_dir.unwrap_or_else(|| {
        let app_dir = db_path.parent().unwrap();
        app_dir.join("backups")
    });

    if let Err(e) = std::fs::create_dir_all(&backups_dir) {
        return Err(format!("Failed to create backups directory: {}", e));
    }

    let now = chrono::Local::now();
    let backup_filename = format!("manual_backup_{}.db", now.format("%Y-%m-%d_%H-%M-%S"));
    let backup_path = backups_dir.join(&backup_filename);

    std::fs::copy(&db_path, &backup_path)
        .map_err(|e| format!("Failed to copy database to backup: {}", e))?;

    Ok(backup_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn select_backup_file() -> Option<String> {
    let file = rfd::FileDialog::new()
        .add_filter("Database File", &["db"])
        .pick_file();

    file.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn select_database_file() -> Option<String> {
    let file = rfd::FileDialog::new()
        .set_title("Select Herbarium Database File")
        .add_filter("SQLite Database", &["db", "sqlite", "sqlite3"])
        .pick_file();

    file.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn configure_database(app: AppHandle, path: String) -> Result<(), String> {
    db::set_database_path(&app, &path)?;
    db::init_database(&app)
}

#[tauri::command]
pub fn restore_database_from_backup(app: AppHandle, backup_path: String) -> Result<(), String> {
    let db_path = db::get_db_path(&app).ok_or_else(|| "Database is not configured.".to_string())?;
    let backup_file = std::path::Path::new(&backup_path);
    if !backup_file.exists() {
        return Err("Selected backup file does not exist.".to_string());
    }

    // Copy backup to database file
    std::fs::copy(backup_file, &db_path)
        .map_err(|e| format!("Failed to copy database file: {}", e))?;

    // Delete WAL and SHM files if they exist to avoid corruption/out-of-sync recovery
    let wal_path = db_path.with_extension("db-wal");
    if wal_path.exists() {
        let _ = std::fs::remove_file(wal_path);
    }
    let shm_path = db_path.with_extension("db-shm");
    if shm_path.exists() {
        let _ = std::fs::remove_file(shm_path);
    }

    Ok(())
}

#[tauri::command]
pub fn select_csv_file() -> Option<String> {
    let file = rfd::FileDialog::new()
        .add_filter("CSV File", &["csv"])
        .pick_file();

    file.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_reference_metadata(app: AppHandle) -> Result<serde_json::Value, String> {
    ReferenceService::get_metadata(&app)
}

#[tauri::command]
pub async fn import_reference_dataset(
    app: AppHandle,
    filepath: String,
    append: bool,
) -> Result<(), String> {
    ReferenceService::import_reference_dataset(&app, &filepath, append)
}

#[tauri::command]
pub async fn get_wcvp_metadata(app: AppHandle) -> Result<serde_json::Value, String> {
    ReferenceService::get_wcvp_metadata(&app)
}

#[tauri::command]
pub async fn import_wcvp_dataset(
    app: AppHandle,
    filepath: String,
    version: i32,
) -> Result<(), String> {
    ReferenceService::import_wcvp_dataset(&app, &filepath, version)
}
