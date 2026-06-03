use rusqlite::{Connection, Result, params};
use tauri::{AppHandle, Manager};
use std::path::PathBuf;
use std::fs;
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha256;

use crate::parser::normalize_taxon_name;

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
    
    // Setup tables
    run_migrations(&conn).map_err(|e| e.to_string())?;
    
    // Seed sample reference data if the reference tables are empty (helps in dev and testing)
    seed_sample_data(&conn).map_err(|e| e.to_string())?;
    
    // Auto-normalize imported CSV reference records if they exist and are empty
    auto_normalize_reference_data(&conn).map_err(|e| e.to_string())?;
    
    Ok(())
}

fn run_migrations(conn: &Connection) -> Result<()> {
    // 0. Drop old tables, triggers, and virtual tables if they exist
    let _ = conn.execute("DROP TRIGGER IF EXISTS parsed_gbif_ai", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS parsed_gbif_ad", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS parsed_gbif_au", []);
    let _ = conn.execute("DROP TABLE IF EXISTS parsed_gbif_fts", []);
    let _ = conn.execute("DROP TABLE IF EXISTS parsed_gbif", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ai", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_ad", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS gbif_au", []);
    let _ = conn.execute("DROP TABLE IF EXISTS gbif_fts", []);
    let _ = conn.execute("DROP TABLE IF EXISTS gbif", []);

    // 1. Reference Data Tables
    conn.execute(
        "CREATE TABLE IF NOT EXISTS gbif (
            gbifID INTEGER PRIMARY KEY,
            collectionCode TEXT,
            catalogNumber TEXT,
            recordNumber TEXT,
            recordedBy TEXT,
            normalizedRecordedBy TEXT,
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
            normalized_scientific_name TEXT
        );",
        [],
    )?;

    // Drop old wcvp_taxonomy trigger and table if it exists to align with new schema
    let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_ai", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_ad", []);
    let _ = conn.execute("DROP TRIGGER IF EXISTS wcvp_taxonomy_au", []);
    let _ = conn.execute("DROP TABLE IF EXISTS wcvp_taxonomy_fts", []);
    let _ = conn.execute("DROP TABLE IF EXISTS wcvp_taxonomy", []);

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
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_country ON gbif(country COLLATE NOCASE);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_family ON gbif(family COLLATE NOCASE);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_gbif_normalized_sci_name ON gbif(normalized_scientific_name);", [])?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_plant_name_id ON wcvp_taxonomy(plant_name_id);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_accepted_plant_name_id ON wcvp_taxonomy(accepted_plant_name_id);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_basionym_plant_name_id ON wcvp_taxonomy(basionym_plant_name_id);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_wcvp_taxonomy_parent_plant_name_id ON wcvp_taxonomy(parent_plant_name_id);", [])?;

    // 3. FTS5 Virtual Tables setup for gbif table (external content content-rowid mapped for maximum index efficiency)
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS gbif_fts USING fts5(
            locality,
            locationRemarks,
            verbatimLocality,
            scientificName,
            normalized_scientific_name,
            content='gbif',
            content_rowid='gbifID'
        );",
        [],
    )?;

    // Create FTS triggers to automatically index inserts/deletes/updates to gbif
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS gbif_ai AFTER INSERT ON gbif BEGIN
            INSERT INTO gbif_fts(
                rowid,
                locality,
                locationRemarks,
                verbatimLocality,
                scientificName,
                normalized_scientific_name
            )
            VALUES (
                new.gbifID,
                new.locality,
                new.locationRemarks,
                new.verbatimLocality,
                new.scientificName,
                new.normalized_scientific_name
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
                normalized_scientific_name
            )
            VALUES (
                'delete',
                old.gbifID,
                old.locality,
                old.locationRemarks,
                old.verbatimLocality,
                old.scientificName,
                old.normalized_scientific_name
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
                normalized_scientific_name
            )
            VALUES (
                'delete',
                old.gbifID,
                old.locality,
                old.locationRemarks,
                old.verbatimLocality,
                old.scientificName,
                old.normalized_scientific_name
            );

            INSERT INTO gbif_fts(
                rowid,
                locality,
                locationRemarks,
                verbatimLocality,
                scientificName,
                normalized_scientific_name
            )
            VALUES (
                new.gbifID,
                new.locality,
                new.locationRemarks,
                new.verbatimLocality,
                new.scientificName,
                new.normalized_scientific_name
            );
        END;",
        [],
    )?;

    // 4. FTS5 Virtual Table for wcvp_taxonomy table
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS wcvp_taxonomy_fts USING fts5(
            taxon_name,
            content='wcvp_taxonomy'
        );",
        [],
    )?;

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

fn seed_sample_data(conn: &Connection) -> Result<()> {
    // Check if gbif has any data
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM gbif", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    
    println!("Reference table empty. Seeding realistic sample herbarium records...");

    // Seed 1. gbif reference dataset (small subset for testing)
    let sample_records = vec![
        (
            1, "K", "K000123456", "1042", "John Smith", 2024, 1, 15,
            "South Africa", "Free State", "Kestell", "Kestell", "Kestell district near farm", "Kestell district",
            "Wetland zone, moist soil", "", -28.25, 28.65, "Wetland", "1500m", "Occurs with grasses", "Field note 1",
            "Type", "", "Abelmoschus manihot (L.) Medik.", "John Smith", 2024, 1, 15, "Confirmed"
        ),
        (
            2, "PRE", "PRE0078901", "89", "Alice Johnson", 2023, 5, 20,
            "South Africa", "Free State", "Mangaung", "Bloemfontein", "Bloemfontein Botanical Garden, north facing slope", "Bloemfontein",
            "Grassland, red sand", "", -29.1, 26.2, "Grassland", "1400m", "", "Field note 2",
            "", "", "Abelmoschus esculentus (L.) Moench", "Alice Johnson", 2023, 5, 20, ""
        ),
        (
            3, "BOL", "BOL0054321", "5512", "Peter Williams", 2025, 10, 5,
            "South Africa", "Western Cape", "Cape Town", "Cape Town", "Table Mountain, Nursery Ravine", "Table Mountain, Cape Town",
            "Fynbos scrubland, rocky soil", "", -33.95, 18.45, "Fynbos", "800m", "", "Field note 3",
            "", "", "Protea cynaroides (L.) L.", "Peter Williams", 2025, 10, 5, ""
        ),
        (
            4, "K", "K000123457", "1043", "John Smith", 2024, 1, 17,
            "South Africa", "Free State", "Kestell", "Kestell", "Kestell mountain pass", "Kestell pass",
            "Alpine rock crevice", "", -28.28, 28.67, "Alpine", "1700m", "", "Field note 4",
            "", "", "Protea cynaroides (L.) L.", "John Smith", 2024, 1, 17, ""
        ),
    ];

    for r in sample_records {
        conn.execute(
            "INSERT INTO gbif (
                gbifID, collectionCode, catalogNumber, recordNumber, recordedBy, year, month, day,
                country, stateProvince, county, municipality, locality, verbatimLocality,
                locationRemarks, verbatimCoordinates, decimalLatitude, decimalLongitude, habitat, verbatimElevation,
                occurrenceRemarks, fieldNotes, typeStatus, identificationQualifier, scientificName,
                identifiedBy, yearIdentified, monthIdentified, dayIdentified, identificationRemarks, normalized_scientific_name
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31)",
            params![
                r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10, r.11, r.12, r.13, r.14, r.15, r.16, r.17, r.18, r.19, r.20, r.21, r.22, r.23, r.24, r.25, r.26, r.27, r.28, r.29,
                normalize_taxon_name(r.24)
            ]
        )?;
    }

    // Explicitly rebuild FTS index to sync seeded contents
    conn.execute("INSERT INTO gbif_fts(gbif_fts) VALUES('rebuild');", [])?;

    // Seed 2. WCVP taxonomy references
    let sample_taxa = vec![
        ("Abelmoschus manihot (L.) Medik.", "Malvaceae", "Abelmoschus", "manihot", "(L.) Medik.", "species", "abelmoschus manihot"),
        ("Abelmoschus esculentus (L.) Moench", "Malvaceae", "Abelmoschus", "esculentus", "(L.) Moench", "species", "abelmoschus esculentus"),
        ("Protea cynaroides (L.) L.", "Proteaceae", "Protea", "cynaroides", "(L.) L.", "species", "protea cynaroides"),
    ];

    for t in sample_taxa {
        conn.execute(
            "INSERT INTO wcvp_taxonomy (
                taxon_name, family, genus, species, taxon_authors, taxon_rank, normalized_taxon_name, plant_name_id
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                t.0,
                t.1,
                t.2,
                t.3,
                t.4,
                t.5,
                t.6,
                format!("seed_id_{}", t.3)
            ]
        )?;
    }

    println!("Seeding completed successfully!");
    Ok(())
}

pub fn auto_normalize_reference_data(conn: &Connection) -> Result<()> {
    // 1. Check if gbif has un-normalized records
    let count_gbif: i64 = conn.query_row(
        "SELECT COUNT(*) FROM gbif WHERE scientificName IS NOT NULL AND (normalized_scientific_name IS NULL OR normalized_scientific_name = '')",
        [],
        |r| r.get(0)
    )?;
    
    if count_gbif > 0 {
        println!("Detected {} un-normalized scientific names in gbif. Normalizing...", count_gbif);
        let mut stmt = conn.prepare(
            "SELECT gbifID, scientificName FROM gbif WHERE scientificName IS NOT NULL AND (normalized_scientific_name IS NULL OR normalized_scientific_name = '')"
        )?;
        let mut rows = stmt.query([])?;
        let mut updates = Vec::new();
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let normalized = normalize_taxon_name(&name);
            updates.push((id, normalized));
        }
        
        let mut stmt_update = conn.prepare("UPDATE gbif SET normalized_scientific_name = ?1 WHERE gbifID = ?2")?;
        conn.execute("BEGIN TRANSACTION", [])?;
        for (id, normalized) in updates {
            stmt_update.execute(params![normalized, id])?;
        }
        conn.execute("COMMIT", [])?;
        
        println!("Rebuilding FTS5 full-text index...");
        conn.execute("INSERT INTO gbif_fts(gbif_fts) VALUES('rebuild');", [])?;
        println!("Rebuilt FTS5 index successfully!");
    }
    
    // 2. Check if wcvp_taxonomy has un-normalized records
    let count_wcvp: i64 = conn.query_row(
        "SELECT COUNT(*) FROM wcvp_taxonomy WHERE taxon_name IS NOT NULL AND (normalized_taxon_name IS NULL OR normalized_taxon_name = '')",
        [],
        |r| r.get(0)
    )?;
    
    if count_wcvp > 0 {
        println!("Detected {} un-normalized taxa in wcvp_taxonomy. Normalizing...", count_wcvp);
        let mut stmt = conn.prepare(
            "SELECT plant_name_id, taxon_name FROM wcvp_taxonomy WHERE taxon_name IS NOT NULL AND (normalized_taxon_name IS NULL OR normalized_taxon_name = '')"
        )?;
        let mut rows = stmt.query([])?;
        let mut updates_wcvp = Vec::new();
        while let Some(row) = rows.next()? {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let normalized = normalize_taxon_name(&name);
            updates_wcvp.push((id, normalized));
        }
        
        let mut stmt_update = conn.prepare("UPDATE wcvp_taxonomy SET normalized_taxon_name = ?1 WHERE plant_name_id = ?2")?;
        conn.execute("BEGIN TRANSACTION", [])?;
        for (id, normalized) in updates_wcvp {
            stmt_update.execute(params![normalized, id])?;
        }
        conn.execute("COMMIT", [])?;
        println!("WCVP normalization completed!");
    }
    
    Ok(())
}
