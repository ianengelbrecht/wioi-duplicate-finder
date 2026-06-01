use rusqlite::{Connection, Result, params};
use tauri::{AppHandle, Manager};
use std::path::PathBuf;
use std::fs;
use pbkdf2::pbkdf2_hmac_array;
use sha2::Sha256;

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
    
    Ok(())
}

fn run_migrations(conn: &Connection) -> Result<()> {
    // 1. Reference Data Tables (fallback empty structures if resource not bundled)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS parsed_gbif (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            recordedBy TEXT,
            recordNumber TEXT,
            locality TEXT,
            locationNotes TEXT,
            verbatimLocality TEXT,
            scientificName TEXT,
            normalized_scientific_name TEXT,
            family TEXT,
            genus TEXT,
            specificEpithet TEXT,
            infraSpecificEpithet TEXT,
            country TEXT,
            stateProvince TEXT,
            year INTEGER,
            month INTEGER,
            day INTEGER
        );",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS wcvp_taxonomy (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scientific_name TEXT,
            family TEXT,
            genus TEXT,
            species TEXT,
            authors TEXT,
            rank TEXT,
            normalized_name TEXT
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

    conn.execute(
        "CREATE TABLE IF NOT EXISTS captured_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER NOT NULL,
            recordedBy TEXT,
            recordNumber TEXT,
            locality TEXT,
            locationNotes TEXT,
            verbatimLocality TEXT,
            scientificName TEXT,
            family TEXT,
            genus TEXT,
            specificEpithet TEXT,
            infraSpecificEpithet TEXT,
            country TEXT,
            stateProvince TEXT,
            year INTEGER,
            month INTEGER,
            day INTEGER,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );",
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

    // 3. FTS5 Virtual Tables setup (external content content-rowid mapped for maximum index efficiency)
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS parsed_gbif_fts USING fts5(
            recordedBy,
            locality,
            locationNotes,
            verbatimLocality,
            scientificName,
            normalized_scientific_name,
            content='parsed_gbif',
            content_rowid='id'
        );",
        [],
    )?;

    // Create FTS triggers to automatically index inserts/deletes/updates to parsed_gbif
    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS parsed_gbif_ai AFTER INSERT ON parsed_gbif BEGIN
            INSERT INTO parsed_gbif_fts(rowid, recordedBy, locality, locationNotes, verbatimLocality, scientificName, normalized_scientific_name)
            VALUES (new.id, new.recordedBy, new.locality, new.locationNotes, new.verbatimLocality, new.scientificName, new.normalized_scientific_name);
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS parsed_gbif_ad AFTER DELETE ON parsed_gbif BEGIN
            INSERT INTO parsed_gbif_fts(parsed_gbif_fts, rowid, recordedBy, locality, locationNotes, verbatimLocality, scientificName, normalized_scientific_name)
            VALUES('delete', old.id, old.recordedBy, old.locality, old.locationNotes, old.verbatimLocality, old.scientificName, old.normalized_scientific_name);
        END;",
        [],
    )?;

    conn.execute(
        "CREATE TRIGGER IF NOT EXISTS parsed_gbif_au AFTER UPDATE ON parsed_gbif BEGIN
            INSERT INTO parsed_gbif_fts(parsed_gbif_fts, rowid, recordedBy, locality, locationNotes, verbatimLocality, scientificName, normalized_scientific_name)
            VALUES('delete', old.id, old.recordedBy, old.locality, old.locationNotes, old.verbatimLocality, old.scientificName, old.normalized_scientific_name);
            INSERT INTO parsed_gbif_fts(rowid, recordedBy, locality, locationNotes, verbatimLocality, scientificName, normalized_scientific_name)
            VALUES (new.id, new.recordedBy, new.locality, new.locationNotes, new.verbatimLocality, new.scientificName, new.normalized_scientific_name);
        END;",
        [],
    )?;

    Ok(())
}

fn seed_sample_data(conn: &Connection) -> Result<()> {
    // Check if parsed_gbif has any data
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM parsed_gbif", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    
    println!("Reference table empty. Seeding realistic sample herbarium records...");

    // Seed 1. parseGBIF reference dataset
    let sample_records = vec![
        (
            "John Smith", "1042", "Kestell district near farm", "Wetland zone, moist soil", "Kestell district",
            "Abelmoschus manihot (L.) Medik.", "abelmoschus manihot", "Malvaceae", "Abelmoschus", "manihot", "",
            "South Africa", "Free State", 2024, 1, 15
        ),
        (
            "Alice Johnson", "89", "Bloemfontein Botanical Garden, north facing slope", "Grassland, red sand", "Bloemfontein",
            "Abelmoschus esculentus (L.) Moench", "abelmoschus esculentus", "Malvaceae", "Abelmoschus", "esculentus", "",
            "South Africa", "Free State", 2023, 5, 20
        ),
        (
            "Peter Williams", "5512", "Table Mountain, Nursery Ravine", "Fynbos scrubland, rocky soil", "Table Mountain, Cape Town",
            "Protea cynaroides (L.) L.", "protea cynaroides", "Proteaceae", "Protea", "cynaroides", "",
            "South Africa", "Western Cape", 2025, 10, 5
        ),
        (
            "John Smith", "1043", "Kestell mountain pass", "Alpine rock crevice", "Kestell pass",
            "Protea cynaroides (L.) L.", "protea cynaroides", "Proteaceae", "Protea", "cynaroides", "",
            "South Africa", "Free State", 2024, 1, 17
        ),
    ];

    for r in sample_records {
        conn.execute(
            "INSERT INTO parsed_gbif (
                recordedBy, recordNumber, locality, locationNotes, verbatimLocality,
                scientificName, normalized_scientific_name, family, genus, specificEpithet,
                infraSpecificEpithet, country, stateProvince, year, month, day
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9, r.10, r.11, r.12, r.13, r.14, r.15
            ]
        )?;
    }

    // Explicitly rebuild FTS index to sync seeded contents
    conn.execute("INSERT INTO parsed_gbif_fts(parsed_gbif_fts) VALUES('rebuild');", [])?;

    // Seed 2. WCVP taxonomy references
    let sample_taxa = vec![
        ("Abelmoschus manihot (L.) Medik.", "Malvaceae", "Abelmoschus", "manihot", "(L.) Medik.", "species", "abelmoschus manihot"),
        ("Abelmoschus esculentus (L.) Moench", "Malvaceae", "Abelmoschus", "esculentus", "(L.) Moench", "species", "abelmoschus esculentus"),
        ("Protea cynaroides (L.) L.", "Proteaceae", "Protea", "cynaroides", "(L.) L.", "species", "protea cynaroides"),
    ];

    for t in sample_taxa {
        conn.execute(
            "INSERT INTO wcvp_taxonomy (
                scientific_name, family, genus, species, authors, rank, normalized_name
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![t.0, t.1, t.2, t.3, t.4, t.5, t.6]
        )?;
    }

    println!("Seeding completed successfully!");
    Ok(())
}
