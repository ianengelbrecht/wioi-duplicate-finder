use chrono::{Local, NaiveDate, NaiveDateTime};
use log::{error, info, warn};
use pbkdf2::pbkdf2_hmac_array;
use regex::Regex;
use rusqlite::{functions::FunctionFlags, params, Connection, Result};
use sha2::Sha256;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

use crate::parsers::{
    normalize_locality, normalize_search_recorded_by, normalize_taxon_name, split_names,
};

/// Encodes binary data to standard hex string.
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Hashing function for securing user credentials.
pub fn hash_password(password: &str) -> String {
    let salt = "herbarium_duplicate_finder_salt_2026";
    let password_bytes =
        pbkdf2_hmac_array::<Sha256, 32>(password.as_bytes(), salt.as_bytes(), 10_000);
    to_hex(&password_bytes)
}

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
struct AppConfig {
    database_path: Option<String>,
}

fn get_config_path(app: &AppHandle) -> PathBuf {
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| {
        let mut path = std::env::current_dir().unwrap_or_default();
        path.push("duplicate-finder-data");
        path
    });
    let _ = fs::create_dir_all(&app_dir);
    app_dir.join("config.json")
}

fn load_config(app: &AppHandle) -> AppConfig {
    let path = get_config_path(app);
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }
    }
    AppConfig::default()
}

fn save_config(app: &AppHandle, config: &AppConfig) -> std::result::Result<(), String> {
    let path = get_config_path(app);
    let content = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    fs::write(&path, content).map_err(|e| e.to_string())
}

/// Validates if the selected file is a valid SQLite database containing the required tables.
pub fn validate_database_file(path: &Path) -> std::result::Result<(), String> {
    if !path.exists() {
        return Err("Database file does not exist.".to_string());
    }
    if path.is_dir() {
        return Err("Selected path is a directory, not a database file.".to_string());
    }

    let conn =
        Connection::open(path).map_err(|e| format!("Failed to open SQLite database: {}", e))?;

    // Check if key tables exist: gbif, wcvp, captured_records
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name IN ('gbif', 'wcvp_taxonomy', 'captured_records', 'sessions', 'users', 'export_settings');")
        .map_err(|e| format!("Failed to query table metadata: {}", e))?;

    let found_tables: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| format!("Failed to read table names: {}", e))?
        .filter_map(Result::ok)
        .collect();

    let required = [
        "gbif",
        "wcvp_taxonomy",
        "captured_records",
        "sessions",
        "users",
        "export_settings",
    ];
    let missing: Vec<&str> = required
        .iter()
        .filter(|&&t| !found_tables.iter().any(|f| f == t))
        .copied()
        .collect();

    if !missing.is_empty() {
        return Err(format!(
            "Database is missing key tables: {}. Is this the correct reference database?",
            missing.join(", ")
        ));
    }

    Ok(())
}

/// Configures the application to use the specified database path.
pub fn set_database_path(app: &AppHandle, path: &str) -> std::result::Result<(), String> {
    let p = Path::new(path);
    validate_database_file(p)?;

    let mut config = load_config(app);
    config.database_path = Some(path.to_string());
    save_config(app, &config)?;

    Ok(())
}

/// Resolves the writeable SQLite database path.
pub fn get_db_path(app: &AppHandle) -> Option<PathBuf> {
    let config = load_config(app);
    if let Some(path) = config.database_path {
        Some(PathBuf::from(path))
    } else {
        let app_dir = app.path().app_data_dir().ok()?;
        Some(app_dir.join("duplicate-finder.db"))
    }
}

/// Registers a custom regexp function on the connection.
pub fn register_regexp_function(conn: &Connection) -> Result<()> {
    conn.create_scalar_function(
        "regexp",
        2,
        FunctionFlags::SQLITE_DETERMINISTIC | FunctionFlags::SQLITE_DIRECTONLY,
        |ctx| {
            let pattern: String = ctx.get(0)?;
            let text: String = ctx.get(1)?;

            thread_local! {
                static CACHE: std::cell::RefCell<std::collections::HashMap<String, Regex>> =
                    std::cell::RefCell::new(std::collections::HashMap::new());
            }

            let is_match = CACHE.with(|cache| {
                let mut cache = cache.borrow_mut();
                let re = cache.entry(pattern.clone()).or_insert_with(|| {
                    regex::RegexBuilder::new(&pattern)
                        .case_insensitive(true)
                        .build()
                        .unwrap_or_else(|_| Regex::new(".*").unwrap())
                });
                re.is_match(&text)
            });

            Ok(is_match)
        },
    )
}

/// Factory function to open a SQLite database connection with custom pragmas.
pub fn get_connection(app: &AppHandle) -> std::result::Result<Connection, String> {
    let db_path = get_db_path(app).ok_or_else(|| "Database path is not configured.".to_string())?;
    if !db_path.exists() {
        return Err("Database file does not exist at the configured path.".to_string());
    }
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    // Always enable WAL, normal synchronous mode, and foreign keys
    let _ = conn.execute("PRAGMA journal_mode=WAL;", []);
    let _ = conn.execute("PRAGMA synchronous=NORMAL;", []);
    conn.execute("PRAGMA foreign_keys=ON;", [])
        .map_err(|e| e.to_string())?;

    register_regexp_function(&conn).map_err(|e| e.to_string())?;

    Ok(conn)
}

fn set_last_quick_check_datetime(conn: &Connection) {
    let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = conn.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES ('last_quick_check', ?1);",
        [&now_str],
    );
}

fn is_clean_shutdown(conn: &Connection) -> bool {
    let query = "SELECT value FROM app_metadata WHERE key = 'clean_shutdown';";
    if let Ok(val_str) = conn.query_row(query, [], |r| r.get::<_, String>(0)) {
        return val_str == "1";
    }
    false
}

fn set_clean_shutdown(conn: &Connection, clean: bool) {
    let val = if clean { "1" } else { "0" };
    let _ = conn.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES ('clean_shutdown', ?1);",
        [val],
    );
}

fn is_reference_normalized(conn: &Connection) -> bool {
    let query = "SELECT value FROM app_metadata WHERE key = 'reference_normalized';";
    if let Ok(val_str) = conn.query_row(query, [], |r| r.get::<_, String>(0)) {
        return val_str == "1";
    }
    false
}

/// Initializes the database on startup.
/// Checks database existence, structure validity, runs integrity check, and runs migrations.
pub fn init_database(app: &AppHandle) -> std::result::Result<(), String> {
    let _ = app.emit("db-init-progress", "Resolving database path...");
    let db_path = get_db_path(app).ok_or_else(|| "DATABASE_UNCONFIGURED".to_string())?;
    let config = load_config(app);
    let is_default_path = config.database_path.is_none();

    if !db_path.exists() {
        if is_default_path {
            let mut copied = false;
            let _ = app.emit(
                "db-init-progress",
                "Copying default database from resources...",
            );
            if let Ok(resource_path) = app.path().resolve(
                "resources/duplicate-finder.db",
                tauri::path::BaseDirectory::Resource,
            ) {
                if resource_path.exists() {
                    if let Some(parent) = db_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if fs::copy(&resource_path, &db_path).is_ok() {
                        info!("Copied bundled database from resources to {:?}", db_path);
                        copied = true;
                    }
                }
            }
            if !copied {
                let _ = app.emit("db-init-progress", "Creating new empty database...");
                if let Some(parent) = db_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                // Try to create the database file
                if let Ok(_conn) = Connection::open(&db_path) {
                    info!("Created a new empty database file at {:?}", db_path);
                } else {
                    return Err(format!(
                        "Failed to create default database at {}",
                        db_path.to_string_lossy()
                    ));
                }
            }
        } else {
            return Err(format!("DATABASE_MISSING:{}", db_path.to_string_lossy()));
        }
    }

    let is_new = if let Ok(metadata) = fs::metadata(&db_path) {
        metadata.len() == 0
    } else {
        false
    };

    // If it's not a brand new empty database, validate it first
    if !is_new {
        let _ = app.emit("db-init-progress", "Validating database structure...");
        validate_database_file(&db_path).map_err(|e| format!("DATABASE_INVALID:{}", e))?;
    }

    info!("Opened database at absolute path: {:?}", db_path);
    let mut conn = get_connection(app)?;

    // Ensure app_metadata table exists so we can store metadata
    let _ = conn.execute(
        "CREATE TABLE IF NOT EXISTS app_metadata (
            key TEXT PRIMARY KEY,
            value TEXT
        );",
        [],
    );

    // If it's a brand new empty database, mark reference data as normalized
    if is_new {
        let _ = conn.execute(
            "INSERT OR REPLACE INTO app_metadata (key, value) VALUES ('reference_normalized', '1');",
            [],
        );
    }

    // Run quick integrity check and FTS checks on startup only if not clean last shutdown and NOT a new database
    if !is_new {
        let is_clean = is_clean_shutdown(&conn);
        if !is_clean {
            let _ = app.emit("db-init-progress", "Running database integrity checks...");
            info!("Running startup database integrity quick_check...");
            match conn.query_row("PRAGMA quick_check;", [], |r| r.get::<_, String>(0)) {
                Ok(res) => {
                    if res != "ok" {
                        error!("Database quick_check failed on startup: {}", res);
                        return Err(format!(
                            "DATABASE_INVALID:Database integrity check failed: {}",
                            res
                        ));
                    } else {
                        info!("Database quick_check passed on startup.");
                        set_last_quick_check_datetime(&conn);
                    }
                }
                Err(e) => {
                    error!("Failed to run database quick_check on startup: {}", e);
                    return Err(format!(
                        "DATABASE_INVALID:Database integrity check failed: {}",
                        e
                    ));
                }
            }

            let _ = app.emit("db-init-progress", "Running FTS search index checks...");
            info!("Running startup FTS index integrity checks...");
            if let Err(e) = conn.execute(
                "INSERT INTO gbif_fts(gbif_fts) VALUES('integrity-check');",
                [],
            ) {
                let _ = app.emit(
                    "db-init-progress",
                    "Rebuilding and repairing FTS search index...",
                );
                warn!(
                    "gbif_fts index integrity check failed: {}. Attempting rebuild...",
                    e
                );
                if let Err(rebuild_err) =
                    conn.execute("INSERT INTO gbif_fts(gbif_fts) VALUES('rebuild');", [])
                {
                    error!("Failed to rebuild gbif_fts index: {}", rebuild_err);
                } else {
                    info!("Successfully rebuilt and repaired gbif_fts index!");
                }
            }
        } else {
            info!(
                "Skipping database integrity and FTS checks on startup (clean shutdown verified)."
            );
        }
    }

    // Set clean_shutdown to 0 on startup to mark it as active/running
    set_clean_shutdown(&conn, false);

    // Setup tables
    let _ = app.emit("db-init-progress", "Running database migrations...");
    run_migrations(&mut conn).map_err(|e| e.to_string())?;

    // Validate structure after running migrations if it was a new empty database
    if is_new {
        validate_database_file(&db_path).map_err(|e| format!("DATABASE_INVALID:{}", e))?;
    }

    // Auto-normalize imported CSV reference records if they exist and are empty
    if !is_reference_normalized(&conn) {
        let _ = app.emit("db-init-progress", "Normalizing reference data...");
        auto_normalize_reference_data(&mut conn).map_err(|e| e.to_string())?;
    } else {
        info!("Skipping reference data normalization check (already normalized).");
    }

    // Startup population of the agents table
    let _ = app.emit("db-init-progress", "Populating collector catalog...");
    populate_agents_table(&mut conn, false).map_err(|e| e.to_string())?;

    let _ = app.emit("db-init-progress", "Database ready!");
    Ok(())
}

/// Runs database backup, optimizations, and integrity checks on shutdown.
pub fn shutdown_database(app: &AppHandle) {
    let db_path = match get_db_path(app) {
        Some(p) => p,
        None => return,
    };
    if !db_path.exists() {
        return;
    }
    if let Ok(conn) = get_connection(app) {
        info!("Running database optimization and integrity checks on shutdown...");
        let _ = conn.execute("PRAGMA optimize;", []);

        match conn.query_row("PRAGMA quick_check;", [], |r| r.get::<_, String>(0)) {
            Ok(res) => {
                if res != "ok" {
                    warn!("Database quick_check failed on shutdown: {}", res);
                    set_clean_shutdown(&conn, false);
                } else {
                    info!("Database quick_check passed on shutdown.");
                    set_last_quick_check_datetime(&conn);
                    set_clean_shutdown(&conn, true);
                }
            }
            Err(e) => {
                error!("Failed to run database quick_check on shutdown: {}", e);
                set_clean_shutdown(&conn, false);
            }
        }
    }

    // Perform database backup
    perform_database_backup(app);
}

fn prune_database_backups(backups_dir: &Path, today: NaiveDate) {
    use chrono::{Datelike, Duration};
    use std::collections::HashSet;

    let entries = match fs::read_dir(backups_dir) {
        Ok(e) => e,
        Err(err) => {
            error!("Failed to read backups directory: {}", err);
            return;
        }
    };

    struct BackupFile {
        path: PathBuf,
        datetime: NaiveDateTime,
        age_days: i64,
    }

    let mut backups = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                if filename.starts_with("backup_")
                    && filename.ends_with(".db")
                    && filename.len() >= 29
                {
                    let ts_str = &filename[7..filename.len() - 3];
                    if let Ok(dt) = NaiveDateTime::parse_from_str(ts_str, "%Y-%m-%d_%H-%M-%S") {
                        let date = dt.date();
                        let age_days = (today - date).num_days();
                        backups.push(BackupFile {
                            path,
                            datetime: dt,
                            age_days,
                        });
                    }
                }
            }
        }
    }

    // Sort backups by datetime descending (latest first)
    backups.sort_by_key(|b| std::cmp::Reverse(b.datetime));

    // Pre-calculate target daily dates, ISO weeks, and calendar months to keep
    let mut allowed_daily_dates = HashSet::new();
    for i in 1..=7 {
        if let Some(d) = today.checked_sub_signed(Duration::days(i)) {
            allowed_daily_dates.insert(d);
        }
    }

    let mut allowed_weekly_weeks = HashSet::new();
    for i in 0..4 {
        if let Some(d) = today.checked_sub_signed(Duration::days(i * 7)) {
            let iso = d.iso_week();
            allowed_weekly_weeks.insert((iso.year(), iso.week()));
        }
    }

    let mut allowed_monthly_months = HashSet::new();
    for i in 0..6 {
        let mut year = today.year();
        let mut month = today.month() as i32 - i;
        while month <= 0 {
            month += 12;
            year -= 1;
        }
        allowed_monthly_months.insert((year, month as u32));
    }

    let mut kept_days = HashSet::new();
    let mut kept_weeks = HashSet::new();
    let mut kept_months = HashSet::new();

    for backup in backups {
        let mut keep = false;
        let date = backup.datetime.date();
        let age_days = backup.age_days;

        // Rule 1: Keep all backups for the current day (age <= 0)
        if age_days <= 0 {
            keep = true;
            // Also mark the week and month of today's backups as kept/satisfied,
            // since today's latest backup is the latest backup for this week and month.
            let iso = date.iso_week();
            kept_weeks.insert((iso.year(), iso.week()));
            kept_months.insert((date.year(), date.month()));
        }

        // Rule 2: Keep only the latest backup for each day of the last seven days (1 <= age_days <= 7)
        if !keep && allowed_daily_dates.contains(&date) && !kept_days.contains(&date) {
            keep = true;
            kept_days.insert(date);
            // Also mark the week and month of this backup as kept/satisfied
            let iso = date.iso_week();
            kept_weeks.insert((iso.year(), iso.week()));
            kept_months.insert((date.year(), date.month()));
        }

        // Rule 3: Keep only the latest backup for each week of the last four weeks
        if !keep {
            let iso = date.iso_week();
            let week_key = (iso.year(), iso.week());
            if allowed_weekly_weeks.contains(&week_key) && !kept_weeks.contains(&week_key) {
                keep = true;
                kept_weeks.insert(week_key);
                // Also mark the month of this backup as kept/satisfied
                kept_months.insert((date.year(), date.month()));
            }
        }

        // Rule 4: Keep only the latest backup for each month of the last six months
        if !keep {
            let month_key = (date.year(), date.month());
            if allowed_monthly_months.contains(&month_key) && !kept_months.contains(&month_key) {
                keep = true;
                kept_months.insert(month_key);
            }
        }

        if !keep {
            info!("Pruning old database backup file: {:?}", backup.path);
            let _ = fs::remove_file(&backup.path);
        } else {
            info!(
                "Keeping database backup file: {:?} (age: {} days)",
                backup.path.file_name().unwrap(),
                backup.age_days
            );
        }
    }
}

pub fn perform_database_backup(app: &AppHandle) {
    let db_path = match get_db_path(app) {
        Some(p) => p,
        None => return,
    };
    if !db_path.exists() {
        return;
    }

    let mut custom_backups_dir = None;
    if let Ok(conn) = get_connection(app) {
        if let Ok(backup_loc) = conn.query_row(
            "SELECT backup_location FROM export_settings LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        ) {
            let trim_path = backup_loc.trim();
            if !trim_path.is_empty() {
                custom_backups_dir = Some(PathBuf::from(trim_path));
            }
        }
    }

    let backups_dir = custom_backups_dir.unwrap_or_else(|| {
        let app_dir = db_path.parent().unwrap();
        app_dir.join("backups")
    });

    if let Err(e) = fs::create_dir_all(&backups_dir) {
        error!("Failed to create backups directory: {}", e);
        return;
    }

    let now = Local::now();
    let backup_filename = format!("backup_{}.db", now.format("%Y-%m-%d_%H-%M-%S"));
    let backup_path = backups_dir.join(&backup_filename);

    if let Err(e) = fs::copy(&db_path, &backup_path) {
        error!("Failed to copy database to backup: {}", e);
        return;
    }
    info!("Database backup created: {:?}", backup_path);

    prune_database_backups(&backups_dir, now.naive_local().date());
}

fn run_migrations(conn: &mut Connection) -> Result<()> {
    // 0. Drop old tables, triggers, and virtual tables if they exist
    let _ = conn.execute("DROP TRIGGER IF EXISTS parsed_gbif_ai", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS parsed_gbif_ad", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS parsed_gbif_au", []);
    let _ = conn.execute("DROP TABLE IF EXISTS parsed_gbif_fts", []);
    let _ = conn.execute("DROP TABLE IF EXISTS parsed_gbif", []);

    // 1. Reference Data Tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS gbif (
            gbifID INTEGER PRIMARY KEY,
            collectionCode TEXT,
            catalogNumber TEXT,
            recordNumber TEXT,
            recordedBy TEXT,
            normalizedRecordedBy TEXT,
            searchRecordedBy VARCHAR(100),
            year INTEGER,
            month INTEGER,
            day INTEGER,
            verbatimEventDate VARCHAR(30),
            country TEXT,
            stateProvince TEXT,
            county TEXT,
            municipality TEXT,
            islandGroup TEXT,
            island TEXT,
            locality TEXT,
            verbatimLocality TEXT,
            locationRemarks TEXT,
            verbatimCoordinates TEXT,
            decimalLatitude REAL,
            decimalLongitude REAL,
            habitat TEXT,
            verbatimElevation TEXT,
            elevation VARCHAR(10),
            occurrenceRemarks TEXT,
            fieldNotes TEXT,
            typeStatus TEXT,
            identificationQualifier TEXT,
            family TEXT,
            scientificName TEXT,
            identifiedBy TEXT,
            yearIdentified INTEGER,
            monthIdentified INTEGER,
            dayIdentified INTEGER,
            identificationRemarks TEXT,
            normalized_scientific_name TEXT,
            normalized_locality TEXT,
            fieldNumber TEXT,
            cleanedFieldNumber TEXT
        );",
        [],
    )?;

    // Migrations for existing databases: check and add normalized_locality if missing
    let col_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('gbif') WHERE name='normalized_locality'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !col_exists {
        let _ = conn.execute("ALTER TABLE gbif ADD COLUMN normalized_locality TEXT", []);
    }

    let fn_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('gbif') WHERE name='fieldNumber'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !fn_exists {
        let _ = conn.execute("ALTER TABLE gbif ADD COLUMN fieldNumber TEXT", []);
    }

    let cfn_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('gbif') WHERE name='cleanedFieldNumber'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !cfn_exists {
        let _ = conn.execute("ALTER TABLE gbif ADD COLUMN cleanedFieldNumber TEXT", []);
    }

    let gbif_ig_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('gbif') WHERE name='islandGroup'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !gbif_ig_exists {
        let _ = conn.execute("ALTER TABLE gbif ADD COLUMN islandGroup TEXT", []);
    }

    let gbif_island_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('gbif') WHERE name='island'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !gbif_island_exists {
        let _ = conn.execute("ALTER TABLE gbif ADD COLUMN island TEXT", []);
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS wcvp_taxonomy (
            plant_name_id TEXT,
            ipni_id TEXT,
            taxon_rank TEXT,
            taxon_status TEXT,
            family TEXT,
            genus_hybrid TEXT,
            genus TEXT,
            species_hybrid TEXT,
            species TEXT,
            infraspecific_rank TEXT,
            infraspecies TEXT,
            parenthetical_author TEXT,
            primary_author TEXT,
            publication_author TEXT,
            place_of_publication TEXT,
            volume_and_page TEXT,
            first_published TEXT,
            nomenclatural_remarks TEXT,
            geographic_area TEXT,
            lifeform_description TEXT,
            climate_description TEXT,
            taxon_name TEXT,
            normalized_taxon_name TEXT,
            taxon_authors TEXT,
            accepted_plant_name_id TEXT,
            basionym_plant_name_id TEXT,
            replaced_synonym_author TEXT,
            homotypic_synonym TEXT,
            parent_plant_name_id TEXT,
            powo_id TEXT,
            hybrid_formula TEXT,
            reviewed TEXT,
            fullname TEXT
        );",
        [],
    )?;

    let wcvp_fn_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('wcvp_taxonomy') WHERE name='fullname'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !wcvp_fn_exists {
        info!("Adding fullname column to wcvp_taxonomy...");
        conn.execute("ALTER TABLE wcvp_taxonomy ADD COLUMN fullname TEXT", [])?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_fullname ON wcvp_taxonomy(fullname COLLATE NOCASE);",
            [],
        )?;
        crate::repositories::ReferenceRepository::populate_wcvp_fullname(conn)?;
    }

    // 2. Local capturing schema tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            given_name TEXT NOT NULL DEFAULT '',
            family_name TEXT NOT NULL DEFAULT '',
            initials TEXT NOT NULL DEFAULT '',
            is_admin INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
        [],
    )?;

    let given_name_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('users') WHERE name='given_name'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !given_name_exists {
        info!("Adding profile and admin columns to users...");
        conn.execute(
            "ALTER TABLE users ADD COLUMN given_name TEXT NOT NULL DEFAULT ''",
            [],
        )?;
        conn.execute(
            "ALTER TABLE users ADD COLUMN family_name TEXT NOT NULL DEFAULT ''",
            [],
        )?;
        conn.execute(
            "ALTER TABLE users ADD COLUMN initials TEXT NOT NULL DEFAULT ''",
            [],
        )?;
        conn.execute(
            "ALTER TABLE users ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0",
            [],
        )?;

        // Automatically set the first user to register as an admin using the users.created_at field
        let res = conn.execute(
            "UPDATE users SET is_admin = 1 WHERE id = (SELECT id FROM users ORDER BY created_at ASC LIMIT 1)",
            [],
        );
        if let Err(e) = res {
            warn!(
                "Could not set first registered user as admin in migration: {}",
                e
            );
        }
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            last_exported_at TEXT,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
        );",
        [],
    )?;

    let _ = conn.execute("ALTER TABLE sessions ADD COLUMN last_exported_at TEXT", []);
    let _ = conn.execute(
        "ALTER TABLE captured_records ADD COLUMN cultivated INTEGER DEFAULT 0",
        [],
    );

    conn.execute(
        "CREATE TABLE IF NOT EXISTS captured_records (
            id INTEGER PRIMARY KEY,
            session_id INTEGER NOT NULL,
            collectionCode TEXT,
            catalogNumber TEXT,
            duplicates TEXT,
            recordNumber TEXT,
            recordedBy TEXT,
            verbatimEventDate TEXT,
            year INTEGER,
            month INTEGER,
            day INTEGER,
            country TEXT,
            stateProvince TEXT,
            county TEXT,
            municipality TEXT,
            islandGroup TEXT,
            island TEXT,
            locality TEXT,
            locationRemarks TEXT,
            verbatimCoordinates TEXT,
            decimalLatitude REAL,
            decimalLongitude REAL,
            verbatimElevation TEXT,
            habitat TEXT,
            occurrenceRemarks TEXT,
            fieldNotes TEXT,
            typeStatus TEXT,
            identificationQualifier TEXT,
            scientificName TEXT,
            identifiedBy TEXT,
            yearIdentified INTEGER,
            monthIdentified INTEGER,
            dayIdentified INTEGER,
            identificationRemarks TEXT,
            taxonID TEXT,
            cultivated INTEGER DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            modified_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );",
        [],
    )?;

    let cr_ig_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('captured_records') WHERE name='islandGroup'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !cr_ig_exists {
        let _ = conn.execute(
            "ALTER TABLE captured_records ADD COLUMN islandGroup TEXT",
            [],
        );
    }

    let cr_island_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('captured_records') WHERE name='island'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !cr_island_exists {
        let _ = conn.execute("ALTER TABLE captured_records ADD COLUMN island TEXT", []);
    }

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS captured_records_modified_at
        AFTER UPDATE ON captured_records
        FOR EACH ROW
        BEGIN
            UPDATE captured_records
            SET modified_at = CURRENT_TIMESTAMP
            WHERE id = NEW.id;
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS export_settings (
            user_id INTEGER PRIMARY KEY,
            format TEXT NOT NULL,
            collection_code TEXT NOT NULL DEFAULT 'RHOIO',
            include_grid_reference INTEGER NOT NULL DEFAULT 0,
            include_islands INTEGER NOT NULL DEFAULT 0,
            backup_location TEXT NOT NULL DEFAULT '',
            home_country TEXT NOT NULL DEFAULT '',
            initials_require_periods INTEGER NOT NULL DEFAULT 1,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
        );",
        [],
    )?;

    // Migrations for export_settings table
    let es_col_exists = |conn: &Connection, col: &str| -> bool {
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM pragma_table_info('export_settings') WHERE name='{}'",
                col
            ),
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false)
    };

    if !es_col_exists(conn, "collection_code") {
        let _ = conn.execute(
            "ALTER TABLE export_settings ADD COLUMN collection_code TEXT NOT NULL DEFAULT 'RHOIO'",
            [],
        );
    }
    if !es_col_exists(conn, "include_grid_reference") {
        let _ = conn.execute("ALTER TABLE export_settings ADD COLUMN include_grid_reference INTEGER NOT NULL DEFAULT 0", []);
    }
    if !es_col_exists(conn, "include_islands") {
        let _ = conn.execute(
            "ALTER TABLE export_settings ADD COLUMN include_islands INTEGER NOT NULL DEFAULT 0",
            [],
        );
    }
    if !es_col_exists(conn, "backup_location") {
        let _ = conn.execute(
            "ALTER TABLE export_settings ADD COLUMN backup_location TEXT NOT NULL DEFAULT ''",
            [],
        );
    }
    if !es_col_exists(conn, "home_country") {
        let _ = conn.execute(
            "ALTER TABLE export_settings ADD COLUMN home_country TEXT NOT NULL DEFAULT ''",
            [],
        );
    }
    if !es_col_exists(conn, "initials_require_periods") {
        let _ = conn.execute(
            "ALTER TABLE export_settings ADD COLUMN initials_require_periods INTEGER NOT NULL DEFAULT 1",
            [],
        );
    }

    if es_col_exists(conn, "mappings") {
        // Migrate existing settings from mappings JSON to columns
        let mut stmt = conn.prepare("SELECT user_id, mappings FROM export_settings")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut updates = Vec::new();
        for (user_id, mappings_str) in rows.flatten() {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&mappings_str) {
                let collection_code = val
                    .get("collectionCode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("RHOIO")
                    .to_string();
                let include_grid_ref = val
                    .get("includeGridReference")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let backup_loc = val
                    .get("backupLocation")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                updates.push((user_id, collection_code, include_grid_ref, backup_loc));
            }
        }

        for (user_id, col_code, inc_grid, b_loc) in updates {
            let _ = conn.execute(
                "UPDATE export_settings SET collection_code = ?1, include_grid_reference = ?2, backup_location = ?3 WHERE user_id = ?4",
                params![col_code, inc_grid as i32, b_loc, user_id],
            );
        }

        // Now drop the mappings column
        let _ = conn.execute("ALTER TABLE export_settings DROP COLUMN mappings", []);
    }

    // Create standard indexes for query optimization
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_recordNumber ON gbif(recordNumber);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_year ON gbif(year);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_month ON gbif(month);",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_day ON gbif(day);", [])?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_date ON gbif(year, month, day);",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_normalizedRecordedBy ON gbif(normalizedRecordedBy COLLATE NOCASE);", [])?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_searchRecordedBy ON gbif(searchRecordedBy);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_country ON gbif(country COLLATE NOCASE);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_stateProvince ON gbif(stateProvince COLLATE NOCASE);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_county ON gbif(county COLLATE NOCASE);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_municipality ON gbif(municipality COLLATE NOCASE);",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_geo_hierarchy ON gbif(country COLLATE NOCASE, stateProvince COLLATE NOCASE, county COLLATE NOCASE, municipality COLLATE NOCASE);", [])?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_islandGroup ON gbif(islandGroup COLLATE NOCASE);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_island ON gbif(island COLLATE NOCASE);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_island_hierarchy ON gbif(country COLLATE NOCASE, islandGroup COLLATE NOCASE, island COLLATE NOCASE);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_family ON gbif(family COLLATE NOCASE);",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_normalized_sci_name ON gbif(normalized_scientific_name);", [])?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_gbif_collectionCode ON gbif(collectionCode);",
        [],
    )?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_plant_name_id ON wcvp_taxonomy(plant_name_id);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_accepted_plant_name_id ON wcvp_taxonomy(accepted_plant_name_id);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_basionym_plant_name_id ON wcvp_taxonomy(basionym_plant_name_id);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_parent_plant_name_id ON wcvp_taxonomy(parent_plant_name_id);", [])?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_taxon_name ON wcvp_taxonomy(taxon_name);",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_fullname ON wcvp_taxonomy(fullname COLLATE NOCASE);",
        [],
    )?;

    // Drop gbif_fts and recreate if it does not have the normalized_locality or cleanedFieldNumber column
    let fts_col_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('gbif_fts') WHERE name='normalized_locality'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    let fts_cfn_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('gbif_fts') WHERE name='cleanedFieldNumber'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !fts_col_exists || !fts_cfn_exists {
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ai", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ad", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_au", []);
        let _ = conn.execute("DROP TABLE IF EXISTS gbif_fts", []);
    }

    // 3. FTS5 Virtual Tables setup for gbif table
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS gbif_fts USING fts5(
            locality,
            locationRemarks,
            verbatimLocality,
            scientificName,
            normalized_scientific_name,
            normalized_locality,
            cleanedFieldNumber,
            content='gbif',
            content_rowid='gbifID'
        );",
        [],
    )?;

    recreate_gbif_triggers(conn)?;

    // Create agents table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agents (
            agentName TEXT PRIMARY KEY,
            searchAgentName TEXT NOT NULL,
            created_at TEXT,
            created_by INTEGER,
            last_used TEXT,
            last_used_by INTEGER
        );",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_agents_searchAgentName ON agents(searchAgentName);",
        [],
    )?;

    // Migrate existing databases to add created_at, created_by, last_used, and last_used_by columns if missing
    let agent_created_at_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('agents') WHERE name='created_at'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !agent_created_at_exists {
        let _ = conn.execute("ALTER TABLE agents ADD COLUMN created_at TEXT", []);
    }

    let agent_created_by_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('agents') WHERE name='created_by'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !agent_created_by_exists {
        let _ = conn.execute("ALTER TABLE agents ADD COLUMN created_by INTEGER", []);
    }

    let agent_last_used_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('agents') WHERE name='last_used'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !agent_last_used_exists {
        let _ = conn.execute("ALTER TABLE agents ADD COLUMN last_used TEXT", []);
    }

    let agent_last_used_by_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('agents') WHERE name='last_used_by'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if !agent_last_used_by_exists {
        let _ = conn.execute("ALTER TABLE agents ADD COLUMN last_used_by INTEGER", []);
    }

    // Migration: Drop wcvp_taxonomy_fts table and triggers if they exist, and create index on normalized_taxon_name
    let fts_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='wcvp_taxonomy_fts'",
            [],
            |r| r.get::<_, i32>(0).map(|c| c > 0),
        )
        .unwrap_or(false);

    if fts_exists {
        info!("Dropping wcvp_taxonomy_fts and triggers (migrating to regex-based lookup)...");
        let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_ai", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_ad", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_au", []);
        let _ = conn.execute("DROP TABLE IF EXISTS wcvp_taxonomy_fts", []);

        // Force re-normalization of records containing infraspecific rank indicators
        info!("Resetting normalized_taxon_name for infraspecific records to force re-normalization...");
        let _ = conn.execute(
            "UPDATE wcvp_taxonomy SET normalized_taxon_name = NULL 
             WHERE taxon_name LIKE '% subsp.%' 
                OR taxon_name LIKE '% var.%' 
                OR taxon_name LIKE '% f.%' 
                OR taxon_name LIKE '% ssp.%'
                OR taxon_name LIKE '% subg.%'
                OR taxon_name LIKE '% sect.%'
                OR taxon_name LIKE '% subsp %'
                OR taxon_name LIKE '% var %'
                OR taxon_name LIKE '% ssp %'",
            [],
        );
    }

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_normalized_name ON wcvp_taxonomy(normalized_taxon_name);",
        [],
    )?;

    Ok(())
}

fn recreate_gbif_triggers(conn: &Connection) -> Result<()> {
    let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_cfn_insert", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_cfn_update", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ai", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ad", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_au", []);

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS gbif_cfn_insert AFTER INSERT ON gbif BEGIN
            UPDATE gbif
            SET cleanedFieldNumber = (
                WITH RECURSIVE char_pos(pos, digit_seq, in_digit) AS (
                    SELECT 1, 
                           CASE WHEN substr(NEW.fieldNumber, 1, 1) GLOB '[0-9]' THEN substr(NEW.fieldNumber, 1, 1) ELSE '' END,
                           CASE WHEN substr(NEW.fieldNumber, 1, 1) GLOB '[0-9]' THEN 1 ELSE 0 END
                    UNION ALL
                    SELECT pos + 1,
                           CASE 
                             WHEN substr(NEW.fieldNumber, pos + 1, 1) GLOB '[0-9]' THEN 
                               CASE WHEN in_digit THEN digit_seq || substr(NEW.fieldNumber, pos + 1, 1) ELSE digit_seq || ' ' || substr(NEW.fieldNumber, pos + 1, 1) END
                             ELSE 
                               digit_seq
                           END,
                           CASE WHEN substr(NEW.fieldNumber, pos + 1, 1) GLOB '[0-9]' THEN 1 ELSE 0 END
                    FROM char_pos
                    WHERE pos < length(NEW.fieldNumber)
                )
                SELECT CASE 
                    WHEN NEW.fieldNumber IS NULL OR NEW.fieldNumber = '' THEN NULL
                    WHEN trim(digit_seq) IS NULL OR trim(digit_seq) = '' THEN '-'
                    ELSE trim(digit_seq)
                END FROM char_pos ORDER BY pos DESC LIMIT 1
            )
            WHERE gbifID = NEW.gbifID;
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS gbif_cfn_update AFTER UPDATE OF fieldNumber ON gbif BEGIN
            UPDATE gbif
            SET cleanedFieldNumber = (
                WITH RECURSIVE char_pos(pos, digit_seq, in_digit) AS (
                    SELECT 1, 
                           CASE WHEN substr(NEW.fieldNumber, 1, 1) GLOB '[0-9]' THEN substr(NEW.fieldNumber, 1, 1) ELSE '' END,
                           CASE WHEN substr(NEW.fieldNumber, 1, 1) GLOB '[0-9]' THEN 1 ELSE 0 END
                    UNION ALL
                    SELECT pos + 1,
                           CASE 
                             WHEN substr(NEW.fieldNumber, pos + 1, 1) GLOB '[0-9]' THEN 
                               CASE WHEN in_digit THEN digit_seq || substr(NEW.fieldNumber, pos + 1, 1) ELSE digit_seq || ' ' || substr(NEW.fieldNumber, pos + 1, 1) END
                             ELSE 
                               digit_seq
                           END,
                           CASE WHEN substr(NEW.fieldNumber, pos + 1, 1) GLOB '[0-9]' THEN 1 ELSE 0 END
                    FROM char_pos
                    WHERE pos < length(NEW.fieldNumber)
                )
                SELECT CASE 
                    WHEN NEW.fieldNumber IS NULL OR NEW.fieldNumber = '' THEN NULL
                    WHEN trim(digit_seq) IS NULL OR trim(digit_seq) = '' THEN '-'
                    ELSE trim(digit_seq)
                END FROM char_pos ORDER BY pos DESC LIMIT 1
            )
            WHERE gbifID = NEW.gbifID;
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS gbif_ai AFTER INSERT ON gbif BEGIN
            INSERT INTO gbif_fts(
                rowid,
                locality,
                locationRemarks,
                verbatimLocality,
                scientificName,
                normalized_scientific_name,
                normalized_locality,
                cleanedFieldNumber
            )
            VALUES (
                new.gbifID,
                new.locality,
                new.locationRemarks,
                new.verbatimLocality,
                new.scientificName,
                new.normalized_scientific_name,
                new.normalized_locality,
                new.cleanedFieldNumber
            );
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS gbif_ad AFTER DELETE ON gbif BEGIN
            INSERT INTO gbif_fts(
                gbif_fts,
                rowid,
                locality,
                locationRemarks,
                verbatimLocality,
                scientificName,
                normalized_scientific_name,
                normalized_locality,
                cleanedFieldNumber
            )
            VALUES (
                'delete',
                old.gbifID,
                old.locality,
                old.locationRemarks,
                old.verbatimLocality,
                old.scientificName,
                old.normalized_scientific_name,
                old.normalized_locality,
                old.cleanedFieldNumber
            );
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS gbif_au AFTER UPDATE OF locality, locationRemarks, verbatimLocality, scientificName, normalized_scientific_name, normalized_locality, cleanedFieldNumber ON gbif BEGIN
            INSERT INTO gbif_fts(
                gbif_fts,
                rowid,
                locality,
                locationRemarks,
                verbatimLocality,
                scientificName,
                normalized_scientific_name,
                normalized_locality,
                cleanedFieldNumber
            )
            VALUES (
                'delete',
                old.gbifID,
                old.locality,
                old.locationRemarks,
                old.verbatimLocality,
                old.scientificName,
                old.normalized_scientific_name,
                old.normalized_locality,
                old.cleanedFieldNumber
            );

            INSERT INTO gbif_fts(
                rowid,
                locality,
                locationRemarks,
                verbatimLocality,
                scientificName,
                normalized_scientific_name,
                normalized_locality,
                cleanedFieldNumber
            )
            VALUES (
                new.gbifID,
                new.locality,
                new.locationRemarks,
                new.verbatimLocality,
                new.scientificName,
                new.normalized_scientific_name,
                new.normalized_locality,
                new.cleanedFieldNumber
            );
        END;",
        [],
    )?;
    Ok(())
}

pub fn get_wcvp_version(conn: &Connection) -> std::result::Result<i32, String> {
    let query = "SELECT value FROM app_metadata WHERE key = 'wcvp_version';";
    match conn.query_row(query, [], |r| r.get::<_, String>(0)) {
        Ok(v_str) => {
            if let Ok(v) = v_str.parse::<i32>() {
                Ok(v)
            } else {
                Ok(15)
            }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(15),
        Err(e) => Err(e.to_string()),
    }
}

pub fn set_wcvp_version(conn: &Connection, version: i32) -> std::result::Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES ('wcvp_version', ?1);",
        [version.to_string()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn recreate_wcvp_triggers_and_rebuild_fts(
    _conn: &Connection,
) -> std::result::Result<(), String> {
    Ok(())
}

pub fn auto_normalize_reference_data(conn: &mut Connection) -> Result<()> {
    let count_gbif: i64 = conn.query_row(
        "SELECT COUNT(*) FROM gbif WHERE scientificName IS NOT NULL AND scientificName != '' AND (normalized_scientific_name IS NULL OR normalized_scientific_name = '')",
        [],
        |r| r.get(0)
    )?;

    let count_locality: i64 = conn.query_row(
        "SELECT COUNT(*) FROM gbif WHERE (normalized_locality IS NULL OR normalized_locality = '') AND (locality IS NOT NULL AND locality != '' OR locationRemarks IS NOT NULL AND locationRemarks != '' OR verbatimLocality IS NOT NULL AND verbatimLocality != '')",
        [],
        |r| r.get(0)
    )?;

    if count_gbif > 0 || count_locality > 0 {
        info!(
            "Detected un-normalized data in gbif ({} names, {} localities). Normalizing...",
            count_gbif, count_locality
        );

        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ai", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ad", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_au", []);

        let mut updates_names = Vec::new();
        if count_gbif > 0 {
            let mut stmt = conn.prepare(
                "SELECT gbifID, scientificName FROM gbif WHERE scientificName IS NOT NULL AND scientificName != '' AND (normalized_scientific_name IS NULL OR normalized_scientific_name = '')"
            )?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let name: String = row.get(1)?;
                let normalized = normalize_taxon_name(&name);
                let val = if normalized.trim().is_empty() {
                    "-".to_string()
                } else {
                    normalized
                };
                updates_names.push((id, val));
            }
        }

        let mut updates_locality = Vec::new();
        if count_locality > 0 {
            let mut stmt = conn.prepare(
                "SELECT gbifID, locality, locationRemarks, verbatimLocality FROM gbif WHERE (normalized_locality IS NULL OR normalized_locality = '') AND (locality IS NOT NULL AND locality != '' OR locationRemarks IS NOT NULL AND locationRemarks != '' OR verbatimLocality IS NOT NULL AND verbatimLocality != '')"
            )?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let id: i64 = row.get(0)?;
                let locality_val: Option<String> = row.get(1)?;
                let remarks_val: Option<String> = row.get(2)?;
                let verbatim_val: Option<String> = row.get(3)?;

                let combined = format!(
                    "{} {} {}",
                    locality_val.unwrap_or_default(),
                    remarks_val.unwrap_or_default(),
                    verbatim_val.unwrap_or_default()
                );
                let normalized = normalize_locality(&combined);
                let val = if normalized.trim().is_empty() {
                    "-".to_string()
                } else {
                    normalized
                };
                updates_locality.push((id, val));
            }
        }

        {
            let tx = conn.transaction()?;

            if !updates_names.is_empty() {
                let mut stmt_update = tx
                    .prepare("UPDATE gbif SET normalized_scientific_name = ?1 WHERE gbifID = ?2")?;
                for (id, normalized) in updates_names {
                    stmt_update.execute(params![normalized, id])?;
                }
            }

            if !updates_locality.is_empty() {
                let mut stmt_update =
                    tx.prepare("UPDATE gbif SET normalized_locality = ?1 WHERE gbifID = ?2")?;
                for (id, normalized) in updates_locality {
                    stmt_update.execute(params![normalized, id])?;
                }
            }

            tx.commit()?;
        }

        recreate_gbif_triggers(conn)?;

        info!("Rebuilding FTS5 full-text index for gbif...");
        conn.execute("INSERT INTO gbif_fts(gbif_fts) VALUES('rebuild');", [])?;
        info!("Rebuilt FTS5 index successfully!");
    }

    let count_cfn: i64 = conn.query_row(
        "SELECT COUNT(*) FROM gbif WHERE fieldNumber IS NOT NULL AND fieldNumber != '' AND (cleanedFieldNumber IS NULL OR cleanedFieldNumber = '')",
        [],
        |r| r.get(0)
    ).unwrap_or(0);

    if count_cfn > 0 {
        info!(
            "Detected {} un-normalized field numbers in gbif. Normalizing...",
            count_cfn
        );

        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ai", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ad", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_au", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_cfn_insert", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_cfn_update", []);

        conn.execute(
            "UPDATE gbif SET cleanedFieldNumber = (
                WITH RECURSIVE char_pos(pos, digit_seq, in_digit) AS (
                    SELECT 1, 
                           CASE WHEN substr(fieldNumber, 1, 1) GLOB '[0-9]' THEN substr(fieldNumber, 1, 1) ELSE '' END,
                           CASE WHEN substr(fieldNumber, 1, 1) GLOB '[0-9]' THEN 1 ELSE 0 END
                    UNION ALL
                    SELECT pos + 1,
                           CASE 
                             WHEN substr(fieldNumber, pos + 1, 1) GLOB '[0-9]' THEN 
                               CASE WHEN in_digit THEN digit_seq || substr(fieldNumber, pos + 1, 1) ELSE digit_seq || ' ' || substr(fieldNumber, pos + 1, 1) END
                             ELSE 
                               digit_seq
                           END,
                           CASE WHEN substr(fieldNumber, pos + 1, 1) GLOB '[0-9]' THEN 1 ELSE 0 END
                    FROM char_pos
                    WHERE pos < length(fieldNumber)
                )
                SELECT CASE WHEN trim(digit_seq) IS NULL OR trim(digit_seq) = '' THEN '-' ELSE trim(digit_seq) END FROM char_pos ORDER BY pos DESC LIMIT 1
            )
            WHERE fieldNumber IS NOT NULL AND fieldNumber != '' AND (cleanedFieldNumber IS NULL OR cleanedFieldNumber = '');",
            [],
        )?;

        recreate_gbif_triggers(conn)?;

        info!("Rebuilding FTS5 full-text index for gbif...");
        conn.execute("INSERT INTO gbif_fts(gbif_fts) VALUES('rebuild');", [])?;
        info!("Rebuilt FTS5 index successfully!");
    }

    let count_wcvp: i64 = conn.query_row(
        "SELECT COUNT(*) FROM wcvp_taxonomy WHERE taxon_name IS NOT NULL AND taxon_name != '' AND (normalized_taxon_name IS NULL OR normalized_taxon_name = '')",
        [],
        |r| r.get(0)
    )?;

    if count_wcvp > 0 {
        info!(
            "Detected {} un-normalized taxa in wcvp_taxonomy. Normalizing...",
            count_wcvp
        );

        let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_ai", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_ad", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_au", []);

        let mut updates_wcvp = Vec::new();
        {
            let mut stmt = conn.prepare(
                "SELECT plant_name_id, taxon_name FROM wcvp_taxonomy WHERE taxon_name IS NOT NULL AND taxon_name != '' AND (normalized_taxon_name IS NULL OR normalized_taxon_name = '')"
            )?;
            let mut rows = stmt.query([])?;
            while let Some(row) = rows.next()? {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let normalized = normalize_taxon_name(&name);
                let val = if normalized.trim().is_empty() {
                    "-".to_string()
                } else {
                    normalized
                };
                updates_wcvp.push((id, val));
            }
        }

        {
            let tx = conn.transaction()?;
            {
                let mut stmt_update = tx.prepare(
                    "UPDATE wcvp_taxonomy SET normalized_taxon_name = ?1 WHERE plant_name_id = ?2",
                )?;
                for (id, normalized) in updates_wcvp {
                    stmt_update.execute(params![normalized, id])?;
                }
            }
            tx.commit()?;
        }

        info!("WCVP normalization completed!");
    }

    // Mark reference data as normalized
    let _ = conn.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES ('reference_normalized', '1');",
        [],
    );

    Ok(())
}

pub fn populate_agents_table(
    conn: &mut Connection,
    force: bool,
) -> std::result::Result<(), String> {
    if !force {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM agents", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if count > 0 {
            return Ok(());
        }
    }

    info!("Agents table is empty. Populating from gbif...");

    let mut agents = std::collections::HashSet::new();

    {
        let mut stmt = conn
            .prepare("SELECT DISTINCT recordedBy, identifiedBy FROM gbif")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let recorded_by: Option<String> = row.get(0).map_err(|e| e.to_string())?;
            let identified_by: Option<String> = row.get(1).map_err(|e| e.to_string())?;

            if let Some(rb) = recorded_by {
                for agent in split_names(&rb) {
                    agents.insert(agent);
                }
            }
            if let Some(ib) = identified_by {
                for agent in split_names(&ib) {
                    agents.insert(agent);
                }
            }
        }
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;
    {
        let mut insert_stmt = tx
            .prepare_cached(
                "INSERT OR IGNORE INTO agents (agentName, searchAgentName) VALUES (?1, ?2)",
            )
            .map_err(|e| e.to_string())?;
        for agent in agents {
            if !agent.is_empty() {
                let search_name = normalize_search_recorded_by(&agent);
                let _ = insert_stmt.execute(params![agent, search_name]);
            }
        }
    }
    tx.commit().map_err(|e| e.to_string())?;

    info!("Agents table populated successfully!");
    Ok(())
}

pub fn finalize_reference_import(conn: &mut Connection) -> std::result::Result<(), String> {
    recreate_gbif_triggers(conn).map_err(|e| e.to_string())?;

    info!("Rebuilding FTS5 index for gbif...");
    conn.execute("INSERT INTO gbif_fts(gbif_fts) VALUES('rebuild');", [])
        .map_err(|e| e.to_string())?;

    populate_agents_table(conn, true).map_err(|e| e.to_string())?;

    // Mark reference data as normalized
    let _ = conn.execute(
        "INSERT OR REPLACE INTO app_metadata (key, value) VALUES ('reference_normalized', '1');",
        [],
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::fs;

    #[test]
    fn test_prune_database_backups() {
        let temp_dir = std::env::temp_dir().join("test_prune_database_backups");
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        // Let's assume today is June 20, 2026
        let today = NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();

        // Create some backup files
        let backup_files = vec![
            // Today's backups (keep all)
            "backup_2026-06-20_18-00-00.db",
            "backup_2026-06-20_17-00-00.db",
            // Last 7 days daily backups (keep latest of each day)
            "backup_2026-06-19_12-00-00.db", // Day 1
            "backup_2026-06-19_10-00-00.db", // Day 1 - delete
            "backup_2026-06-18_15-00-00.db", // Day 2
            "backup_2026-06-17_15-00-00.db", // Day 3
            "backup_2026-06-16_15-00-00.db", // Day 4
            "backup_2026-06-15_15-00-00.db", // Day 5
            "backup_2026-06-14_15-00-00.db", // Day 6
            "backup_2026-06-13_15-00-00.db", // Day 7
            // Last 4 weeks (weekly backups - keep latest of each ISO week)
            "backup_2026-06-12_12-00-00.db", // Week 24 - delete (since June 13 is newer and already kept under Daily)
            "backup_2026-06-12_10-00-00.db", // Week 24 - delete
            "backup_2026-06-05_12-00-00.db", // Week 23 - keep
            "backup_2026-06-05_10-00-00.db", // Week 23 - delete
            "backup_2026-05-29_12-00-00.db", // Week 22 - keep
            // Last 6 months (monthly backups - keep latest of each calendar month)
            "backup_2026-05-22_12-00-00.db", // May - delete (since May 29 is newer and already kept under Weekly)
            "backup_2026-05-21_12-00-00.db", // May - delete
            "backup_2026-04-15_12-00-00.db", // April - keep
            "backup_2026-04-14_12-00-00.db", // April - delete
            "backup_2026-03-15_12-00-00.db", // March - keep
            "backup_2026-02-15_12-00-00.db", // Feb - keep
            "backup_2026-01-15_12-00-00.db", // Jan - keep
            // Month 7: December 2025 (more than 6 months ago - delete)
            "backup_2025-12-15_12-00-00.db", // delete
        ];

        for name in &backup_files {
            let path = temp_dir.join(name);
            fs::write(&path, "dummy content").unwrap();
        }

        // Call the pruning logic
        prune_database_backups(&temp_dir, today);

        // Read remaining files
        let mut remaining: Vec<String> = fs::read_dir(&temp_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        remaining.sort();

        let mut expected = vec![
            "backup_2026-06-20_18-00-00.db", // Today 1
            "backup_2026-06-20_17-00-00.db", // Today 2
            "backup_2026-06-19_12-00-00.db", // Day 1
            "backup_2026-06-18_15-00-00.db", // Day 2
            "backup_2026-06-17_15-00-00.db", // Day 3
            "backup_2026-06-16_15-00-00.db", // Day 4
            "backup_2026-06-15_15-00-00.db", // Day 5
            "backup_2026-06-14_15-00-00.db", // Day 6
            "backup_2026-06-13_15-00-00.db", // Day 7
            "backup_2026-06-05_12-00-00.db", // Week 23
            "backup_2026-05-29_12-00-00.db", // Week 22
            "backup_2026-04-15_12-00-00.db", // April month
            "backup_2026-03-15_12-00-00.db", // March month
            "backup_2026-02-15_12-00-00.db", // Feb month
            "backup_2026-01-15_12-00-00.db", // Jan month
        ];
        expected.sort();

        assert_eq!(remaining, expected);

        // Clean up
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_wcvp_version_and_import() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE app_metadata (
                key TEXT PRIMARY KEY,
                value TEXT
            );",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE wcvp_taxonomy (
                plant_name_id TEXT,
                ipni_id TEXT,
                taxon_rank TEXT,
                taxon_status TEXT,
                family TEXT,
                genus_hybrid TEXT,
                genus TEXT,
                species_hybrid TEXT,
                species TEXT,
                infraspecific_rank TEXT,
                infraspecies TEXT,
                parenthetical_author TEXT,
                primary_author TEXT,
                publication_author TEXT,
                place_of_publication TEXT,
                volume_and_page TEXT,
                first_published TEXT,
                nomenclatural_remarks TEXT,
                geographic_area TEXT,
                lifeform_description TEXT,
                climate_description TEXT,
                taxon_name TEXT,
                normalized_taxon_name TEXT,
                taxon_authors TEXT,
                accepted_plant_name_id TEXT,
                basionym_plant_name_id TEXT,
                replaced_synonym_author TEXT,
                homotypic_synonym TEXT,
                parent_plant_name_id TEXT,
                powo_id TEXT,
                hybrid_formula TEXT,
                reviewed TEXT,
                fullname TEXT
            );",
            [],
        )
        .unwrap();

        let default_v = get_wcvp_version(&conn).unwrap();
        assert_eq!(default_v, 15);

        set_wcvp_version(&conn, 16).unwrap();
        let saved_v = get_wcvp_version(&conn).unwrap();
        assert_eq!(saved_v, 16);

        let temp_csv = std::env::temp_dir().join("test_wcvp.csv");
        let csv_content = "plant_name_id|family|genus|species|taxon_name|reviewed\n\
                           1001|Myrtaceae|Eucalyptus|globulus|Eucalyptus globulus|1\n\
                           1002|Fabaceae|Acacia|dealbata|Acacia dealbata|0\n";
        fs::write(&temp_csv, csv_content).unwrap();

        let mut conn_mut = conn;
        crate::repositories::ReferenceRepository::import_wcvp_csv(
            None,
            &mut conn_mut,
            temp_csv.to_str().unwrap(),
            17,
        )
        .unwrap();

        let v_after = get_wcvp_version(&conn_mut).unwrap();
        assert_eq!(v_after, 17);

        let count: i64 = conn_mut
            .query_row("SELECT COUNT(*) FROM wcvp_taxonomy", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let norm_name: String = conn_mut
            .query_row(
                "SELECT normalized_taxon_name FROM wcvp_taxonomy WHERE plant_name_id = '1001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(norm_name, "eucalyptus globulus");

        let csv_content_updated = "plant_name_id|family|genus|species|taxon_name|reviewed\n\
                                   1001|Myrtaceae|Eucalyptus|globulus|Eucalyptus globulus|0\n\
                                   1003|Rosaceae|Rosa|canina|Rosa canina|1\n";
        fs::write(&temp_csv, csv_content_updated).unwrap();
        crate::repositories::ReferenceRepository::import_wcvp_csv(
            None,
            &mut conn_mut,
            temp_csv.to_str().unwrap(),
            18,
        )
        .unwrap();

        let total_count: i64 = conn_mut
            .query_row("SELECT COUNT(*) FROM wcvp_taxonomy", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total_count, 3);

        let reviewed_1001: Option<String> = conn_mut
            .query_row(
                "SELECT reviewed FROM wcvp_taxonomy WHERE plant_name_id = '1001'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(reviewed_1001, Some("0".to_string()));

        let _ = fs::remove_file(&temp_csv);
    }

    #[test]
    fn test_autocomplete_scientific_name() {
        let conn = Connection::open_in_memory().unwrap();
        register_regexp_function(&conn).unwrap();

        conn.execute(
            "CREATE TABLE app_metadata (
                key TEXT PRIMARY KEY,
                value TEXT
            );",
            [],
        )
        .unwrap();

        conn.execute(
            "CREATE TABLE wcvp_taxonomy (
                plant_name_id TEXT,
                ipni_id TEXT,
                taxon_rank TEXT,
                taxon_status TEXT,
                family TEXT,
                genus_hybrid TEXT,
                genus TEXT,
                species_hybrid TEXT,
                species TEXT,
                infraspecific_rank TEXT,
                infraspecies TEXT,
                parenthetical_author TEXT,
                primary_author TEXT,
                publication_author TEXT,
                place_of_publication TEXT,
                volume_and_page TEXT,
                first_published TEXT,
                nomenclatural_remarks TEXT,
                geographic_area TEXT,
                lifeform_description TEXT,
                climate_description TEXT,
                taxon_name TEXT,
                normalized_taxon_name TEXT,
                taxon_authors TEXT,
                accepted_plant_name_id TEXT,
                basionym_plant_name_id TEXT,
                replaced_synonym_author TEXT,
                homotypic_synonym TEXT,
                parent_plant_name_id TEXT,
                powo_id TEXT,
                hybrid_formula TEXT,
                reviewed TEXT,
                fullname TEXT
            );",
            [],
        )
        .unwrap();

        let temp_csv = std::env::temp_dir().join("test_wcvp_autocomplete.csv");
        let csv_content = "plant_name_id|family|genus|species|taxon_name|reviewed\n\
                           1001|Malvaceae|Adansonia|digitata|Adansonia digitata|1\n\
                           1002|Plantaginaceae|Digitalis|purpurea|Digitalis purpurea|1\n\
                           1003|Poaceae|Digitaria|adscendens|Digitaria adscendens|1\n";
        fs::write(&temp_csv, csv_content).unwrap();

        let mut conn_mut = conn;
        crate::repositories::ReferenceRepository::import_wcvp_csv(
            None,
            &mut conn_mut,
            temp_csv.to_str().unwrap(),
            1,
        )
        .unwrap();

        let _ = fs::remove_file(&temp_csv);

        let results = crate::repositories::TaxonomyRepository::autocomplete_scientific_name(
            &conn_mut, "ad dig",
        )
        .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].scientific_name, "Adansonia digitata");

        let results2 = crate::repositories::TaxonomyRepository::autocomplete_scientific_name(
            &conn_mut, "dig ad",
        )
        .unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].scientific_name, "Digitaria adscendens");
    }

    #[test]
    fn test_field_number_normalization() {
        let mut conn = Connection::open_in_memory().unwrap();

        // 1. Initialize schema via run_migrations
        run_migrations(&mut conn).unwrap();

        // 2. Insert records using SQL directly (testing triggers)
        conn.execute(
            "INSERT INTO gbif (gbifID, fieldNumber) VALUES (1, 's.n.')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO gbif (gbifID, fieldNumber) VALUES (2, '1234a')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO gbif (gbifID, fieldNumber) VALUES (3, NULL)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO gbif (gbifID, fieldNumber) VALUES (4, '')", [])
            .unwrap();

        // Check trigger-populated cleanedFieldNumber
        let cfn1: Option<String> = conn
            .query_row(
                "SELECT cleanedFieldNumber FROM gbif WHERE gbifID = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let cfn2: Option<String> = conn
            .query_row(
                "SELECT cleanedFieldNumber FROM gbif WHERE gbifID = 2",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let cfn3: Option<String> = conn
            .query_row(
                "SELECT cleanedFieldNumber FROM gbif WHERE gbifID = 3",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let cfn4: Option<String> = conn
            .query_row(
                "SELECT cleanedFieldNumber FROM gbif WHERE gbifID = 4",
                [],
                |r| r.get(0),
            )
            .unwrap();

        assert_eq!(cfn1.as_deref(), Some("-"));
        assert_eq!(cfn2.as_deref(), Some("1234"));
        assert_eq!(cfn3, None);
        assert_eq!(cfn4, None);

        // 3. Clear cleanedFieldNumber to simulate old un-normalized data or imported data
        conn.execute("UPDATE gbif SET cleanedFieldNumber = NULL", [])
            .unwrap();

        // Now check how many un-normalized records are detected before normalizing
        let count_cfn: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gbif WHERE fieldNumber IS NOT NULL AND fieldNumber != '' AND (cleanedFieldNumber IS NULL OR cleanedFieldNumber = '')",
            [],
            |r| r.get(0)
        ).unwrap_or(0);
        assert_eq!(count_cfn, 2); // 1 ('s.n.') and 2 ('1234a')

        // Run the manual migration step for cleanedFieldNumber
        let count_cfn: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gbif WHERE fieldNumber IS NOT NULL AND fieldNumber != '' AND (cleanedFieldNumber IS NULL OR cleanedFieldNumber = '')",
            [],
            |r| r.get(0)
        ).unwrap_or(0);

        if count_cfn > 0 {
            conn.execute(
                "UPDATE gbif SET cleanedFieldNumber = (
                    WITH RECURSIVE char_pos(pos, digit_seq, in_digit) AS (
                        SELECT 1, 
                               CASE WHEN substr(fieldNumber, 1, 1) GLOB '[0-9]' THEN substr(fieldNumber, 1, 1) ELSE '' END,
                               CASE WHEN substr(fieldNumber, 1, 1) GLOB '[0-9]' THEN 1 ELSE 0 END
                        UNION ALL
                        SELECT pos + 1,
                               CASE 
                                 WHEN substr(fieldNumber, pos + 1, 1) GLOB '[0-9]' THEN 
                                   CASE WHEN in_digit THEN digit_seq || substr(fieldNumber, pos + 1, 1) ELSE digit_seq || ' ' || substr(fieldNumber, pos + 1, 1) END
                                 ELSE 
                                   digit_seq
                               END,
                               CASE WHEN substr(fieldNumber, pos + 1, 1) GLOB '[0-9]' THEN 1 ELSE 0 END
                        FROM char_pos
                        WHERE pos < length(fieldNumber)
                    )
                    SELECT CASE WHEN trim(digit_seq) IS NULL OR trim(digit_seq) = '' THEN '-' ELSE trim(digit_seq) END FROM char_pos ORDER BY pos DESC LIMIT 1
                )
                WHERE fieldNumber IS NOT NULL AND fieldNumber != '' AND (cleanedFieldNumber IS NULL OR cleanedFieldNumber = '');",
                [],
            ).unwrap();
        }

        // Check if there are any un-normalized records left
        let count_cfn_after: i64 = conn.query_row(
            "SELECT COUNT(*) FROM gbif WHERE fieldNumber IS NOT NULL AND fieldNumber != '' AND (cleanedFieldNumber IS NULL OR cleanedFieldNumber = '')",
            [],
            |r| r.get(0)
        ).unwrap_or(0);
        assert_eq!(count_cfn_after, 0);

        // Verify the values
        let cfn1_after: Option<String> = conn
            .query_row(
                "SELECT cleanedFieldNumber FROM gbif WHERE gbifID = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let cfn2_after: Option<String> = conn
            .query_row(
                "SELECT cleanedFieldNumber FROM gbif WHERE gbifID = 2",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cfn1_after.as_deref(), Some("-"));
        assert_eq!(cfn2_after.as_deref(), Some("1234"));
    }
}
