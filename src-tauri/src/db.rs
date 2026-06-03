use rusqlite::{Connection, Result, params};
use tauri::{AppHandle, Manager};
use std::path::PathBuf;
use std::fs;
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha256;

use crate::parser::{normalize_taxon_name, normalize_locality};

/// Encodes binary data to standard hex string.
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Hashing function for securing user credentials.
pub fn hash_password(password: &str) -> String {
    let salt = "herbarium_duplicate_finder_salt_2026";
    let password_bytes = pbkdf2_hmac_array::<Sha256, 32>(password.as_bytes(), salt.as_bytes(), 10_000);
    to_hex(&password_bytes)
}

/// Resolves the writeable SQLite database path.
pub fn get_db_path(app: &AppHandle) -> PathBuf {
    let app_dir = app.path().app_data_dir().unwrap_or_else(|_| {
        let mut path = std::env::current_dir().unwrap_or_default();
        path.push("duplicate-finder-data");
        path
    });
    
    // Ensure the app directory exists
    let _ = fs::create_dir_all(&app_dir);
    app_dir.join("reference.db")
}

/// Initializes the database on startup.
/// Copies the bundled reference database if not present, runs migrations, and seeds fallback test data.
pub fn init_database(app: &AppHandle) -> std::result::Result<(), String> {
    let db_path = get_db_path(app);
    
    if !db_path.exists() {
        // Option 1: Try copying the bundled resource database
        if let Ok(resource_path) = app.path().resource_dir().map(|p| p.join("resources/reference.db")) {
            if resource_path.exists() {
                if let Err(err) = fs::copy(&resource_path, &db_path) {
                    println!("Failed to copy reference.db resource: {}", err);
                } else {
                    println!("Successfully copied pre-bundled reference.db resource!");
                }
            } else {
                println!("Reference DB resource not found at {:?}, initializing empty DB.", resource_path);
            }
        } else {
            println!("Could not resolve resource path for reference.db, initializing empty DB.");
        }
    }
    
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    // Enable WAL journal mode & normal synchronous for performance and resilience
    let _ = conn.execute("PRAGMA journal_mode=WAL;", []);
    let _ = conn.execute("PRAGMA synchronous=NORMAL;", []);
    
    // Run quick integrity check on startup
    if let Ok(res) = conn.query_row("PRAGMA quick_check;", [], |r| r.get::<_, String>(0)) {
        if res != "ok" {
            println!("Warning: Database quick_check failed on startup: {}", res);
        } else {
            println!("Database quick_check passed on startup.");
        }
    }
    
    // Setup tables
    run_migrations(&conn).map_err(|e| e.to_string())?;
    
    // Auto-normalize imported CSV reference records if they exist and are empty
    auto_normalize_reference_data(&conn).map_err(|e| e.to_string())?;
    
    Ok(())
}

/// Runs optimizations and integrity checks on shutdown.
pub fn shutdown_database(app: &AppHandle) {
    let db_path = get_db_path(app);
    if !db_path.exists() {
        return;
    }
    if let Ok(conn) = Connection::open(&db_path) {
        println!("Running database optimization and integrity checks on shutdown...");
        let _ = conn.execute("PRAGMA optimize;", []);
        
        match conn.query_row("PRAGMA quick_check;", [], |r| r.get::<_, String>(0)) {
            Ok(res) => {
                if res != "ok" {
                    println!("Database quick_check failed on shutdown: {}", res);
                } else {
                    println!("Database quick_check passed on shutdown.");
                }
            }
            Err(e) => {
                println!("Failed to run database quick_check on shutdown: {}", e);
            }
        }
    }
}

fn run_migrations(conn: &Connection) -> Result<()> {
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
            normalized_locality TEXT
        );",
        [],
    )?;

    // Migrations for existing databases: check and add normalized_locality if missing
    let col_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('gbif') WHERE name='normalized_locality'",
        [],
        |r| r.get::<_, i32>(0).map(|c| c > 0)
    ).unwrap_or(false);
    
    if !col_exists {
        let _ = conn.execute("ALTER TABLE gbif ADD COLUMN normalized_locality TEXT", []);
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
            reviewed TEXT
        );",
        [],
    )?;

    // 2. Local capturing schema tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
        );",
        [],
    )?;

    // Drop old captured_records trigger and table if it exists to align with new schema
    let _ = conn.execute("DROP TRIGGER IF EXISTS captured_records_modified_at", []);
    let _ = conn.execute("DROP TABLE IF EXISTS captured_records", []);

    conn.execute(
        "CREATE TABLE IF NOT EXISTS captured_records (
            id INTEGER PRIMARY KEY,
            session_id INTEGER NOT NULL,
            collectionCode TEXT,
            catalogNumber TEXT,
            duplicates INTEGER,
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
            locality TEXT,
            locationRemarks TEXT,
            verbatimCoordinates TEXT,
            verbatimElevation TEXT,
            habitat TEXT,
            occurrenceRemarks TEXT,
            typeStatus TEXT,
            identificationQualifier TEXT,
            scientificName TEXT,
            identifiedBy TEXT,
            yearIdentified INTEGER,
            monthIdentified INTEGER,
            dayIdentified INTEGER,
            identificationRemarks TEXT,
            taxonID TEXT,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            modified_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );",
        [],
    )?;

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
            mappings TEXT NOT NULL,
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
        );",
        [],
    )?;

    // Create standard indexes for query optimization
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_recordNumber ON gbif(recordNumber);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_year ON gbif(year);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_month ON gbif(month);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_day ON gbif(day);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_date ON gbif(year, month, day);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_normalizedRecordedBy ON gbif(normalizedRecordedBy COLLATE NOCASE);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_searchRecordedBy ON gbif(searchRecordedBy);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_country ON gbif(country COLLATE NOCASE);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_stateProvince ON gbif(stateProvince COLLATE NOCASE);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_county ON gbif(county COLLATE NOCASE);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_municipality ON gbif(municipality COLLATE NOCASE);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_geo_hierarchy ON gbif(country COLLATE NOCASE, stateProvince COLLATE NOCASE, county COLLATE NOCASE, municipality COLLATE NOCASE);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_family ON gbif(family COLLATE NOCASE);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_normalized_sci_name ON gbif(normalized_scientific_name);", [])?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_plant_name_id ON wcvp_taxonomy(plant_name_id);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_accepted_plant_name_id ON wcvp_taxonomy(accepted_plant_name_id);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_basionym_plant_name_id ON wcvp_taxonomy(basionym_plant_name_id);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_parent_plant_name_id ON wcvp_taxonomy(parent_plant_name_id);", [])?;

    // Drop gbif_fts and recreate if it does not have the normalized_locality column
    let fts_col_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('gbif_fts') WHERE name='normalized_locality'",
        [],
        |r| r.get::<_, i32>(0).map(|c| c > 0)
    ).unwrap_or(false);
    
    if !fts_col_exists {
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ai", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ad", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_au", []);
        let _ = conn.execute("DROP TABLE IF EXISTS gbif_fts", []);
    }

    // 3. FTS5 Virtual Tables setup for gbif table (external content content-rowid mapped for maximum index efficiency)
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS gbif_fts USING fts5(
            locality,
            locationRemarks,
            verbatimLocality,
            scientificName,
            normalized_scientific_name,
            normalized_locality,
            content='gbif',
            content_rowid='gbifID'
        );",
        [],
    )?;

    // Create FTS triggers to automatically index inserts/deletes/updates to gbif
    recreate_gbif_triggers(conn)?;

    // 4. FTS5 Virtual Table for wcvp_taxonomy table
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS wcvp_taxonomy_fts USING fts5(
            taxon_name,
            content='wcvp_taxonomy'
        );",
        [],
    )?;

    recreate_wcvp_triggers(conn)?;

    Ok(())
}

fn recreate_gbif_triggers(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS gbif_ai AFTER INSERT ON gbif BEGIN
            INSERT INTO gbif_fts(
                rowid,
                locality,
                locationRemarks,
                verbatimLocality,
                scientificName,
                normalized_scientific_name,
                normalized_locality
            )
            VALUES (
                new.gbifID,
                new.locality,
                new.locationRemarks,
                new.verbatimLocality,
                new.scientificName,
                new.normalized_scientific_name,
                new.normalized_locality
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
                normalized_locality
            )
            VALUES (
                'delete',
                old.gbifID,
                old.locality,
                old.locationRemarks,
                old.verbatimLocality,
                old.scientificName,
                old.normalized_scientific_name,
                old.normalized_locality
            );
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS gbif_au AFTER UPDATE ON gbif BEGIN
            INSERT INTO gbif_fts(
                gbif_fts,
                rowid,
                locality,
                locationRemarks,
                verbatimLocality,
                scientificName,
                normalized_scientific_name,
                normalized_locality
            )
            VALUES (
                'delete',
                old.gbifID,
                old.locality,
                old.locationRemarks,
                old.verbatimLocality,
                old.scientificName,
                old.normalized_scientific_name,
                old.normalized_locality
            );

            INSERT INTO gbif_fts(
                rowid,
                locality,
                locationRemarks,
                verbatimLocality,
                scientificName,
                normalized_scientific_name,
                normalized_locality
            )
            VALUES (
                new.gbifID,
                new.locality,
                new.locationRemarks,
                new.verbatimLocality,
                new.scientificName,
                new.normalized_scientific_name,
                new.normalized_locality
            );
        END;",
        [],
    )?;
    Ok(())
}

fn recreate_wcvp_triggers(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS wcvp_taxonomy_ai
        AFTER INSERT ON wcvp_taxonomy
        BEGIN
            INSERT INTO wcvp_taxonomy_fts(rowid, taxon_name)
            VALUES (new.rowid, new.taxon_name);
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS wcvp_taxonomy_ad
        AFTER DELETE ON wcvp_taxonomy
        BEGIN
            INSERT INTO wcvp_taxonomy_fts(wcvp_taxonomy_fts, rowid, taxon_name)
            VALUES ('delete', old.rowid, old.taxon_name);
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS wcvp_taxonomy_au
        AFTER UPDATE ON wcvp_taxonomy
        BEGIN
            INSERT INTO wcvp_taxonomy_fts(wcvp_taxonomy_fts, rowid, taxon_name)
            VALUES ('delete', old.rowid, old.taxon_name);

            INSERT INTO wcvp_taxonomy_fts(rowid, taxon_name)
            VALUES (new.rowid, new.taxon_name);
        END;",
        [],
    )?;
    Ok(())
}

pub fn auto_normalize_reference_data(conn: &Connection) -> Result<()> {
    // 1. Check if gbif has un-normalized records (ignoring nulls and empty strings)
    let count_gbif: i64 = conn.query_row(
        "SELECT COUNT(*) FROM gbif WHERE scientificName IS NOT NULL AND scientificName != '' AND (normalized_scientific_name IS NULL OR normalized_scientific_name = '')",
        [],
        |r| r.get(0)
    )?;
    
    // 1b. Check if gbif has un-normalized localities (ignoring nulls and empty strings)
    let count_locality: i64 = conn.query_row(
        "SELECT COUNT(*) FROM gbif WHERE (normalized_locality IS NULL OR normalized_locality = '') AND (locality IS NOT NULL AND locality != '' OR locationRemarks IS NOT NULL AND locationRemarks != '' OR verbatimLocality IS NOT NULL AND verbatimLocality != '')",
        [],
        |r| r.get(0)
    )?;

    if count_gbif > 0 || count_locality > 0 {
        println!("Detected un-normalized data in gbif ({} names, {} localities). Normalizing...", count_gbif, count_locality);
        
        // Temporarily drop FTS triggers to make updates lightning fast
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ai", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ad", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_au", []);
        
        // Gather scientific names updates
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
                updates_names.push((id, normalized));
            }
        }
        
        // Gather locality updates
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
                updates_locality.push((id, normalized));
            }
        }
        
        // Apply updates in a single transaction
        conn.execute("BEGIN TRANSACTION", [])?;
        
        if !updates_names.is_empty() {
            let mut stmt_update = conn.prepare("UPDATE gbif SET normalized_scientific_name = ?1 WHERE gbifID = ?2")?;
            for (id, normalized) in updates_names {
                stmt_update.execute(params![normalized, id])?;
            }
        }
        
        if !updates_locality.is_empty() {
            let mut stmt_update = conn.prepare("UPDATE gbif SET normalized_locality = ?1 WHERE gbifID = ?2")?;
            for (id, normalized) in updates_locality {
                stmt_update.execute(params![normalized, id])?;
            }
        }
        
        conn.execute("COMMIT", [])?;
        
        // Recreate the triggers
        recreate_gbif_triggers(conn)?;
        
        println!("Rebuilding FTS5 full-text index for gbif...");
        conn.execute("INSERT INTO gbif_fts(gbif_fts) VALUES('rebuild');", [])?;
        println!("Rebuilt FTS5 index successfully!");
    }
    
    // 2. Check if wcvp_taxonomy has un-normalized records
    let count_wcvp: i64 = conn.query_row(
        "SELECT COUNT(*) FROM wcvp_taxonomy WHERE taxon_name IS NOT NULL AND taxon_name != '' AND (normalized_taxon_name IS NULL OR normalized_taxon_name = '')",
        [],
        |r| r.get(0)
    )?;
    
    if count_wcvp > 0 {
        println!("Detected {} un-normalized taxa in wcvp_taxonomy. Normalizing...", count_wcvp);
        
        // Drop triggers to speed up bulk updates
        let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_ai", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_ad", []);
        let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_au", []);
        
        let mut stmt = conn.prepare(
            "SELECT plant_name_id, taxon_name FROM wcvp_taxonomy WHERE taxon_name IS NOT NULL AND taxon_name != '' AND (normalized_taxon_name IS NULL OR normalized_taxon_name = '')"
        )?;
        let mut rows = stmt.query([])?;
        let mut updates_wcvp = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let normalized = normalize_taxon_name(&name);
            updates_wcvp.push((id, normalized));
        }
        
        conn.execute("BEGIN TRANSACTION", [])?;
        let mut stmt_update = conn.prepare("UPDATE wcvp_taxonomy SET normalized_taxon_name = ?1 WHERE plant_name_id = ?2")?;
        for (id, normalized) in updates_wcvp {
            stmt_update.execute(params![normalized, id])?;
        }
        conn.execute("COMMIT", [])?;
        
        // Recreate the triggers
        recreate_wcvp_triggers(conn)?;
        
        println!("Rebuilding FTS5 full-text index for wcvp_taxonomy...");
        conn.execute("INSERT INTO wcvp_taxonomy_fts(wcvp_taxonomy_fts) VALUES('rebuild');", [])?;
        println!("WCVP normalization and index rebuild completed!");
    }

    Ok(())
}
