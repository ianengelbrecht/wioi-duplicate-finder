// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::AppHandle;
use rusqlite::{Connection, params};
use serde_json::json;
use std::fs;

mod parser;
mod db;

use parser::{normalize_taxon_name, normalize_search_recorded_by, normalize_locality};
use db::{get_db_path, init_database, hash_password, shutdown_database};

// -------------------------------------------------------------
// Tauri Command Handlers
// -------------------------------------------------------------

#[tauri::command]
fn initialize_database(app: AppHandle) -> Result<(), String> {
    init_database(&app)
}

#[tauri::command]
fn register_user(app: AppHandle, username: String, password: String) -> Result<String, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let username_clean = username.trim();
    if username_clean.is_empty() || password.is_empty() {
        return Err("Username and password cannot be empty.".to_string());
    }
    
    let hash = hash_password(&password);
    
    match conn.execute(
        "INSERT INTO users (username, password_hash) VALUES (?1, ?2)",
        params![username_clean, hash],
    ) {
        Ok(_) => Ok("User registered successfully!".to_string()),
        Err(rusqlite::Error::SqliteFailure(err, _)) if err.code == rusqlite::ErrorCode::ConstraintViolation => {
            Err("Username already exists. Please choose another.".to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn login_user(app: AppHandle, username: String, password: String) -> Result<Option<serde_json::Value>, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let username_clean = username.trim();
    let hash = hash_password(&password);
    
    let mut stmt = conn
        .prepare("SELECT id, username FROM users WHERE username = ?1 AND password_hash = ?2")
        .map_err(|e| e.to_string())?;
        
    let mut rows = stmt
        .query_map(params![username_clean, hash], |row| {
            let id: i32 = row.get(0)?;
            let uname: String = row.get(1)?;
            Ok(json!({ "id": id, "username": uname }))
        })
        .map_err(|e| e.to_string())?;
        
    if let Some(row) = rows.next() {
        let user = row.map_err(|e| e.to_string())?;
        Ok(Some(user))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn create_session(app: AppHandle, user_id: i32, name: String) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let name_clean = name.trim();
    if name_clean.is_empty() {
        return Err("Session name cannot be empty.".to_string());
    }
    
    conn.execute(
        "INSERT INTO sessions (user_id, name) VALUES (?1, ?2)",
        params![user_id, name_clean],
    ).map_err(|e| e.to_string())?;
    
    let id: i32 = conn.last_insert_rowid() as i32;
    
    Ok(json!({
        "id": id,
        "userId": user_id,
        "name": name_clean,
        "recordCount": 0
    }))
}

#[tauri::command]
fn get_sessions(app: AppHandle, user_id: i32) -> Result<Vec<serde_json::Value>, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.name, COUNT(r.id) as count, MAX(r.modified_at) as last_record, s.last_exported_at 
             FROM sessions s 
             LEFT JOIN captured_records r ON s.id = r.session_id 
             WHERE s.user_id = ?1 
             GROUP BY s.id 
             ORDER BY s.id DESC"
        )
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![user_id], |row| {
        let id: i32 = row.get(0)?;
        let name: String = row.get(1)?;
        let record_count: i64 = row.get(2)?;
        let last_record: Option<String> = row.get(3)?;
        let last_exported_at: Option<String> = row.get(4)?;
        Ok(json!({
            "id": id,
            "name": name,
            "recordCount": record_count,
            "lastRecordAt": last_record,
            "lastExportedAt": last_exported_at
        }))
    }).map_err(|e| e.to_string())?;
    
    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
fn search_reference(app: AppHandle, filters: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    // Parse filters
    let recorded_by = filters.get("recordedBy").and_then(|v| v.as_str()).unwrap_or("").trim();
    let record_number = filters.get("recordNumber").and_then(|v| v.as_str()).unwrap_or("").trim();
    let locality = filters.get("locality").and_then(|v| v.as_str()).unwrap_or("").trim();
    let scientific_name = filters.get("scientificName").and_then(|v| v.as_str()).unwrap_or("").trim();
    let family = filters.get("family").and_then(|v| v.as_str()).unwrap_or("").trim();
    let country = filters.get("country").and_then(|v| v.as_str()).unwrap_or("").trim();
    let state_province = filters.get("stateProvince").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let year = filters.get("year").and_then(|v| v.as_i64());
    let month = filters.get("month").and_then(|v| v.as_i64());
    let day = filters.get("day").and_then(|v| v.as_i64());
    
    // Check search constraints
    let has_recorded_by = !recorded_by.is_empty();
    let has_record_number = !record_number.is_empty();
    let has_locality = !locality.is_empty();
    let has_scientific_name = !scientific_name.is_empty();
    let has_family = !family.is_empty();
    let has_country = !country.is_empty();
    let has_state_province = !state_province.is_empty();
    
    let has_year = year.is_some();
    let has_month = month.is_some();
    let has_day = day.is_some();
    let has_date = has_year || has_month || has_day;
    
    let has_other = has_family || has_scientific_name || has_country || has_state_province || has_locality;
    
    // Count how many non-date fields are filled
    let mut non_date_fields_count = 0;
    if has_recorded_by { non_date_fields_count += 1; }
    if has_record_number { non_date_fields_count += 1; }
    if has_family { non_date_fields_count += 1; }
    if has_scientific_name { non_date_fields_count += 1; }
    if has_country { non_date_fields_count += 1; }
    if has_state_province { non_date_fields_count += 1; }
    if has_locality { non_date_fields_count += 1; }
    
    // Count total filled fields in the form
    let mut total_filled_count = non_date_fields_count;
    if has_year { total_filled_count += 1; }
    if has_month { total_filled_count += 1; }
    if has_day { total_filled_count += 1; }
    
    if total_filled_count == 0 {
        return Err("Please enter at least one query search field.".to_string());
    }
    
    // Rule 1: collector requires at least a collector number, or if just collector and one of the date fields, then also one of the other fields
    if has_recorded_by && !has_record_number && !(has_date && has_other) {
        return Err("Collector search requires a collector number, or if just a collector and a date field, it also requires at least one of (family, scientific name, country, Admin Div 1, or locality).".to_string());
    }
    
    // Rule 2: collector number always requires a collector
    if has_record_number && !has_recorded_by {
        return Err("Collector number always requires a collector name, regardless of other fields.".to_string());
    }
    
    // Rule 3: date searches require at least two other non-date fields
    if has_date && non_date_fields_count < 2 {
        return Err("Searches on year, month, or day require at least two other non-date fields.".to_string());
    }
    
    // Rule 4: family, scientific name, country, stateProvince, or locality requires at least two other fields (total of 3 or more fields)
    if has_other && total_filled_count < 3 {
        return Err("Searching on family, scientific name, country, Admin Div 1, or locality requires at least two other fields (total of 3 or more fields).".to_string());
    }
    
    let mut sql = String::from(
        "SELECT recordedBy, recordNumber, locality, locationRemarks, verbatimLocality, 
                scientificName, family, country, stateProvince, year, month, day,
                identificationQualifier, collectionCode, decimalLatitude, decimalLongitude,
                verbatimCoordinates, verbatimElevation, elevation, habitat, occurrenceRemarks,
                fieldNotes
         FROM gbif WHERE 1=1"
    );
    let mut params_vec: Vec<serde_json::Value> = Vec::new();
    
    if has_recorded_by {
        let normalized = normalize_search_recorded_by(recorded_by);
        sql.push_str(" AND searchRecordedBy LIKE ? COLLATE NOCASE");
        params_vec.push(json!(format!("{}%", normalized)));
    }
    if has_record_number {
        sql.push_str(" AND recordNumber = ?");
        params_vec.push(json!(record_number));
    }
    if has_family {
        sql.push_str(" AND family LIKE ? COLLATE NOCASE");
        params_vec.push(json!(format!("{}%", family)));
    }
    if has_country {
        sql.push_str(" AND country LIKE ? COLLATE NOCASE");
        params_vec.push(json!(format!("{}%", country)));
    }
    if has_state_province {
        sql.push_str(" AND stateProvince LIKE ? COLLATE NOCASE");
        params_vec.push(json!(format!("{}%", state_province)));
    }
    if let Some(y) = year {
        sql.push_str(" AND year = ?");
        params_vec.push(json!(y));
    }
    if let Some(m) = month {
        sql.push_str(" AND month = ?");
        params_vec.push(json!(m));
    }
    if let Some(d) = day {
        sql.push_str(" AND day = ?");
        params_vec.push(json!(d));
    }
    
    // Locality FTS5 Search (multi-term prefix match across normalized_locality)
    if has_locality {
        let normalized = normalize_locality(locality);
        let terms: Vec<&str> = normalized.split_whitespace().collect();
        if !terms.is_empty() {
            let mut match_clauses = Vec::new();
            for term in terms {
                match_clauses.push(format!("normalized_locality:{}*", term));
            }
            if !match_clauses.is_empty() {
                let fts_query = match_clauses.join(" AND ");
                sql.push_str(" AND gbifID IN (SELECT rowid FROM gbif_fts WHERE gbif_fts MATCH ?)");
                params_vec.push(json!(fts_query));
            }
        }
    }
    
    // Taxon FTS5 Search (normalized sequence-exact prefix match)
    if has_scientific_name {
        let normalized = normalize_taxon_name(scientific_name);
        let terms: Vec<&str> = normalized.split_whitespace().collect();
        if !terms.is_empty() {
            let mut fts_query = String::from("^");
            for (i, term) in terms.iter().enumerate() {
                if i > 0 {
                    fts_query.push_str(" + ");
                }
                fts_query.push_str(&format!("{}*", term));
            }
            sql.push_str(" AND gbifID IN (SELECT rowid FROM gbif_fts WHERE gbif_fts MATCH ?)");
            params_vec.push(json!(fts_query));
        }
    }
    
    sql.push_str(" LIMIT 250");
    
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    
    // Map params dynamically
    let rusql_params: Vec<Box<dyn rusqlite::ToSql>> = params_vec.iter().map(|v| {
        if let Some(s) = v.as_str() {
            Box::new(s.to_string()) as Box<dyn rusqlite::ToSql>
        } else if let Some(i) = v.as_i64() {
            Box::new(i) as Box<dyn rusqlite::ToSql>
        } else {
            Box::new("") as Box<dyn rusqlite::ToSql>
        }
    }).collect();
    
    let ref_params: Vec<&dyn rusqlite::ToSql> = rusql_params.iter().map(|b| b.as_ref()).collect();
    
    let rows = stmt.query_map(&ref_params[..], |row| {
        let recorded_by: Option<String> = row.get(0)?;
        let record_number: Option<String> = row.get(1)?;
        let locality: Option<String> = row.get(2)?;
        let location_notes: Option<String> = row.get(3)?;
        let verbatim_locality: Option<String> = row.get(4)?;
        let scientific_name: Option<String> = row.get(5)?;
        let family: Option<String> = row.get(6)?;
        let country: Option<String> = row.get(7)?;
        let state_province: Option<String> = row.get(8)?;
        let year: Option<i32> = row.get(9)?;
        let month: Option<i32> = row.get(10)?;
        let day: Option<i32> = row.get(11)?;
        let id_qualifier: Option<String> = row.get(12)?;
        let collection_code: Option<String> = row.get(13)?;
        let decimal_latitude: Option<f64> = row.get(14)?;
        let decimal_longitude: Option<f64> = row.get(15)?;
        let verbatim_coordinates: Option<String> = row.get(16)?;
        let verbatim_elevation: Option<String> = row.get(17)?;
        let elevation: Option<String> = row.get(18)?;
        let habitat: Option<String> = row.get(19)?;
        let occurrence_remarks: Option<String> = row.get(20)?;
        let field_notes: Option<String> = row.get(21)?;
        
        Ok(json!({
            "id": serde_json::Value::Null,
            "recordedBy": recorded_by.unwrap_or_default(),
            "recordNumber": record_number.unwrap_or_default(),
            "locality": locality.unwrap_or_default(),
            "locationNotes": location_notes.unwrap_or_default(),
            "verbatimLocality": verbatim_locality.unwrap_or_default(),
            "scientificName": scientific_name.unwrap_or_default(),
            "family": family.unwrap_or_default(),
            "genus": "",
            "specificEpithet": "",
            "infraSpecificEpithet": "",
            "country": country.unwrap_or_default(),
            "stateProvince": state_province.unwrap_or_default(),
            "year": year,
            "month": month,
            "day": day,
            "identificationQualifier": id_qualifier.unwrap_or_default(),
            "collectionCode": collection_code.unwrap_or_default(),
            "decimalLatitude": decimal_latitude,
            "decimalLongitude": decimal_longitude,
            "verbatimCoordinates": verbatim_coordinates.unwrap_or_default(),
            "verbatimElevation": verbatim_elevation.unwrap_or_default(),
            "elevation": elevation.unwrap_or_default(),
            "habitat": habitat.unwrap_or_default(),
            "occurrenceRemarks": occurrence_remarks.unwrap_or_default(),
            "fieldNotes": field_notes.unwrap_or_default(),
        }))
    }).map_err(|e| e.to_string())?;
    
    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    
    Ok(list)
}

#[tauri::command]
fn autocomplete_scientific_name(app: AppHandle, query: String) -> Result<Vec<serde_json::Value>, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let q_clean = query.trim();
    if q_clean.is_empty() {
        return Ok(Vec::new());
    }
    
    let terms: Vec<&str> = q_clean.split_whitespace().collect();
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    
    let mut fts_query = String::new();
    for (i, term) in terms.iter().enumerate() {
        let clean = term.trim_matches(|c: char| c.is_ascii_punctuation());
        if !clean.is_empty() {
            if i > 0 {
                fts_query.push_str(" AND ");
            }
            fts_query.push_str(&format!("{}*", clean));
        }
    }
    
    let mut stmt = conn
        .prepare(
            "SELECT plant_name_id, taxon_name, family, genus, species, taxon_authors, taxon_rank 
             FROM wcvp_taxonomy 
             WHERE rowid IN (SELECT rowid FROM wcvp_taxonomy_fts WHERE wcvp_taxonomy_fts MATCH ?1) 
             LIMIT 15"
        )
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![fts_query], |row| {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let family: Option<String> = row.get(2)?;
        let genus: Option<String> = row.get(3)?;
        let species: Option<String> = row.get(4)?;
        let authors: Option<String> = row.get(5)?;
        let rank: Option<String> = row.get(6)?;
        
        let full_name = match &authors {
            Some(a) if !a.trim().is_empty() => format!("{} {}", name, a.trim()),
            _ => name,
        };
        
        Ok(json!({
            "taxonID": id,
            "scientificName": full_name,
            "family": family.unwrap_or_default(),
            "genus": genus.unwrap_or_default(),
            "specificEpithet": species.unwrap_or_default(),
            "authors": authors.unwrap_or_default(),
            "rank": rank.unwrap_or_default()
        }))
    }).map_err(|e| e.to_string())?;
    
    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
fn autocomplete_recorded_by(app: AppHandle, query: String) -> Result<Vec<String>, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let q_clean = query.trim();
    if q_clean.is_empty() {
        return Ok(Vec::new());
    }
    
    let normalized = normalize_search_recorded_by(q_clean);
    
    // We aggregate unique collector names from recordedBy, using searchRecordedBy for lookup.
    // The user must only ever see values from recordedBy.
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT collector FROM (
                SELECT recordedBy AS collector FROM gbif WHERE searchRecordedBy LIKE ?1 COLLATE NOCASE
                UNION
                SELECT recordedBy AS collector FROM captured_records WHERE recordedBy LIKE ?2 COLLATE NOCASE
             ) WHERE collector IS NOT NULL AND collector != '' LIMIT 10"
        )
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![format!("{}%", normalized), format!("{}%", q_clean)], |row| {
        let name: String = row.get(0)?;
        Ok(name)
    }).map_err(|e| e.to_string())?;
    
    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
fn autocomplete_agent(app: AppHandle, query: String) -> Result<Vec<String>, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let q_clean = query.trim();
    if q_clean.is_empty() {
        return Ok(Vec::new());
    }
    
    let normalized = normalize_search_recorded_by(q_clean);
    
    let mut stmt = conn
        .prepare("SELECT agentName FROM agents WHERE searchAgentName LIKE ?1 LIMIT 10")
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![format!("{}%", normalized)], |row| {
        let name: String = row.get(0)?;
        Ok(name)
    }).map_err(|e| e.to_string())?;
    
    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
fn check_agent_exists(app: AppHandle, name: String) -> Result<bool, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let clean = name.trim();
    if clean.is_empty() {
        return Ok(true);
    }
    let normalized = normalize_search_recorded_by(clean);
    let mut stmt = conn
        .prepare("SELECT 1 FROM agents WHERE searchAgentName = ?1")
        .map_err(|e| e.to_string())?;
    let exists = stmt.exists(params![normalized]).map_err(|e| e.to_string())?;
    Ok(exists)
}

#[tauri::command]
fn add_agent(app: AppHandle, name: String) -> Result<(), String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let clean = name.trim();
    if clean.is_empty() {
        return Ok(());
    }
    let search_name = normalize_search_recorded_by(clean);
    conn.execute(
        "INSERT OR IGNORE INTO agents (agentName, searchAgentName) VALUES (?1, ?2)",
        params![clean, search_name],
    ).map_err(|e| e.to_string())?;
    Ok(())
}



#[tauri::command]
fn autocomplete_locality(app: AppHandle, query: String) -> Result<Vec<String>, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let q_clean = query.trim();
    if q_clean.is_empty() {
        return Ok(Vec::new());
    }
    
    let normalized = normalize_locality(q_clean);
    let terms: Vec<&str> = normalized.split_whitespace().collect();
    
    let mut sql = String::from("SELECT MIN(TRIM(locality)) AS uniq_locality FROM (\n");
    let mut params_vec: Vec<String> = Vec::new();
    
    if !terms.is_empty() {
        // Build FTS query for gbif
        let mut match_clauses = Vec::new();
        for term in &terms {
            match_clauses.push(format!("normalized_locality:{}*", term));
        }
        let fts_query = match_clauses.join(" AND ");
        
        sql.push_str("    SELECT locality FROM gbif WHERE gbifID IN (\n");
        sql.push_str("        SELECT rowid FROM gbif_fts WHERE gbif_fts MATCH ?1\n");
        sql.push_str("    )\n");
        params_vec.push(fts_query);
        
        sql.push_str("    UNION ALL\n");
        
        // Build LIKE clauses for captured_records
        sql.push_str("    SELECT locality FROM captured_records WHERE 1=1");
        for (i, term) in terms.iter().enumerate() {
            sql.push_str(&format!(" AND locality LIKE ?{}", i + 2));
            params_vec.push(format!("%{}%", term));
        }
        sql.push_str("\n");
    } else {
        // Fallback for short query or only stopwords
        sql.push_str("    SELECT locality FROM gbif WHERE locality LIKE ?1\n");
        sql.push_str("    UNION ALL\n");
        sql.push_str("    SELECT locality FROM captured_records WHERE locality LIKE ?1\n");
        params_vec.push(format!("%{}%", q_clean));
    }
    
    sql.push_str(") WHERE locality IS NOT NULL AND TRIM(locality) != ''\n");
    sql.push_str("GROUP BY LOWER(TRIM(locality))\n");
    sql.push_str("LIMIT 10");
    
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    
    // Map params dynamically
    let rusql_params: Vec<Box<dyn rusqlite::ToSql>> = params_vec.iter().map(|s| {
        Box::new(s.to_string()) as Box<dyn rusqlite::ToSql>
    }).collect();
    
    let ref_params: Vec<&dyn rusqlite::ToSql> = rusql_params.iter().map(|b| b.as_ref()).collect();
    
    let rows = stmt.query_map(&ref_params[..], |row| {
        let val: String = row.get(0)?;
        Ok(val)
    }).map_err(|e| e.to_string())?;
    
    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
fn save_captured_record(app: AppHandle, record: serde_json::Value) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let id = record.get("id").and_then(|v| v.as_i64());
    let session_id = record.get("sessionId").and_then(|v| v.as_i64()).ok_or("Session ID is required.")?;
    
    let collection_code = record.get("collectionCode").and_then(|v| v.as_str()).unwrap_or("").trim();
    let catalog_number = record.get("catalogNumber").and_then(|v| v.as_str()).unwrap_or("").trim();
    let duplicates = record.get("duplicates").and_then(|v| v.as_i64());
    let record_number = record.get("recordNumber").and_then(|v| v.as_str()).unwrap_or("").trim();
    let recorded_by = record.get("recordedBy").and_then(|v| v.as_str()).unwrap_or("").trim();
    let verbatim_event_date = record.get("verbatimEventDate").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let year = record.get("year").and_then(|v| v.as_i64());
    let month = record.get("month").and_then(|v| v.as_i64());
    let day = record.get("day").and_then(|v| v.as_i64());
    
    let country = record.get("country").and_then(|v| v.as_str()).unwrap_or("").trim();
    let state_province = record.get("stateProvince").and_then(|v| v.as_str()).unwrap_or("").trim();
    let county = record.get("county").and_then(|v| v.as_str()).unwrap_or("").trim();
    let municipality = record.get("municipality").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let locality = record.get("locality").and_then(|v| v.as_str()).unwrap_or("").trim();
    let location_remarks = record.get("locationNotes").and_then(|v| v.as_str()).unwrap_or("").trim(); // UI locationNotes -> locationRemarks
    
    let verbatim_coordinates = record.get("verbatimCoordinates").and_then(|v| v.as_str()).unwrap_or("").trim();
    let decimal_latitude = record.get("decimalLatitude").and_then(|v| {
        if v.is_number() {
            v.as_f64()
        } else if let Some(s) = v.as_str() {
            s.trim().parse::<f64>().ok()
        } else {
            None
        }
    });
    let decimal_longitude = record.get("decimalLongitude").and_then(|v| {
        if v.is_number() {
            v.as_f64()
        } else if let Some(s) = v.as_str() {
            s.trim().parse::<f64>().ok()
        } else {
            None
        }
    });
    let verbatim_elevation = record.get("verbatimElevation").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let habitat = record.get("habitat").and_then(|v| v.as_str()).unwrap_or("").trim();
    let occurrence_remarks = record.get("occurrenceRemarks").and_then(|v| v.as_str()).unwrap_or("").trim();
    let field_notes = record.get("fieldNotes").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let type_status = record.get("typeStatus").and_then(|v| v.as_str()).unwrap_or("").trim();
    let id_qualifier = record.get("identificationQualifier").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let scientific_name = record.get("scientificName").and_then(|v| v.as_str()).unwrap_or("").trim();
    let identified_by = record.get("identifiedBy").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let year_identified = record.get("yearIdentified").and_then(|v| v.as_i64());
    let month_identified = record.get("monthIdentified").and_then(|v| v.as_i64());
    let day_identified = record.get("dayIdentified").and_then(|v| v.as_i64());
    
    let id_remarks = record.get("identificationRemarks").and_then(|v| v.as_str()).unwrap_or("").trim();
    let taxon_id = record.get("taxonID").and_then(|v| v.as_str()).unwrap_or("").trim();
    let cultivated = record.get("cultivated").and_then(|v| {
        if v.is_boolean() {
            Some(if v.as_bool().unwrap_or(false) { 1 } else { 0 })
        } else if v.is_number() {
            v.as_i64()
        } else if let Some(s) = v.as_str() {
            s.trim().parse::<i64>().ok()
        } else {
            None
        }
    }).unwrap_or(0);

    // Insert new unique names into agents table
    let new_recorded_by_agents: Vec<&str> = recorded_by.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    for name in new_recorded_by_agents {
        let search_name = normalize_search_recorded_by(name);
        let _ = conn.execute("INSERT OR IGNORE INTO agents (agentName, searchAgentName) VALUES (?1, ?2)", params![name, search_name]);
    }
    let new_identified_by_agents: Vec<&str> = identified_by.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    for name in new_identified_by_agents {
        let search_name = normalize_search_recorded_by(name);
        let _ = conn.execute("INSERT OR IGNORE INTO agents (agentName, searchAgentName) VALUES (?1, ?2)", params![name, search_name]);
    }
    
    if let Some(existing_id) = id {
        // Update existing record
        conn.execute(
            "UPDATE captured_records SET 
                collectionCode=?1, catalogNumber=?2, duplicates=?3, recordNumber=?4, recordedBy=?5,
                verbatimEventDate=?6, year=?7, month=?8, day=?9, country=?10,
                stateProvince=?11, county=?12, municipality=?13, locality=?14, locationRemarks=?15,
                verbatimCoordinates=?16, decimalLatitude=?17, decimalLongitude=?18, verbatimElevation=?19, habitat=?20, 
                occurrenceRemarks=?21, fieldNotes=?22, typeStatus=?23, identificationQualifier=?24, scientificName=?25, 
                identifiedBy=?26, yearIdentified=?27, monthIdentified=?28, dayIdentified=?29, identificationRemarks=?30, 
                taxonID=?31, cultivated=?32
             WHERE id = ?33 AND session_id = ?34",
            params![
                collection_code, catalog_number, duplicates, record_number, recorded_by,
                verbatim_event_date, year, month, day, country,
                state_province, county, municipality, locality, location_remarks,
                verbatim_coordinates, decimal_latitude, decimal_longitude, verbatim_elevation, habitat, 
                occurrence_remarks, field_notes, type_status, id_qualifier, scientific_name, 
                identified_by, year_identified, month_identified, day_identified, id_remarks, 
                taxon_id, cultivated, existing_id, session_id
            ]
        ).map_err(|e| e.to_string())?;
        
        Ok(json!({ "id": existing_id, "success": true }))
    } else {
        // Insert new record
        conn.execute(
            "INSERT INTO captured_records (
                session_id, collectionCode, catalogNumber, duplicates, recordNumber, recordedBy,
                verbatimEventDate, year, month, day, country, stateProvince, county, municipality,
                locality, locationRemarks, verbatimCoordinates, decimalLatitude, decimalLongitude, verbatimElevation, habitat, occurrenceRemarks, fieldNotes,
                typeStatus, identificationQualifier, scientificName, identifiedBy, yearIdentified,
                monthIdentified, dayIdentified, identificationRemarks, taxonID, cultivated
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33)",
            params![
                session_id, collection_code, catalog_number, duplicates, record_number, recorded_by,
                verbatim_event_date, year, month, day, country, state_province, county, municipality,
                locality, location_remarks, verbatim_coordinates, decimal_latitude, decimal_longitude, verbatim_elevation, habitat, occurrence_remarks, field_notes,
                type_status, id_qualifier, scientific_name, identified_by, year_identified,
                month_identified, day_identified, id_remarks, taxon_id, cultivated
            ]
        ).map_err(|e| e.to_string())?;
        
        let new_id = conn.last_insert_rowid() as i32;
        Ok(json!({ "id": new_id, "success": true }))
    }
}

#[tauri::command]
fn get_captured_records(app: AppHandle, session_id: i32) -> Result<Vec<serde_json::Value>, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn
        .prepare(
            "SELECT id, collectionCode, catalogNumber, duplicates, recordNumber, recordedBy, 
                    verbatimEventDate, year, month, day, country, stateProvince, county, municipality, 
                    locality, locationRemarks, verbatimCoordinates, decimalLatitude, decimalLongitude, verbatimElevation, habitat, occurrenceRemarks, fieldNotes,
                    typeStatus, identificationQualifier, scientificName, identifiedBy, yearIdentified, 
                    monthIdentified, dayIdentified, identificationRemarks, taxonID, cultivated 
             FROM captured_records 
             WHERE session_id = ?1 
             ORDER BY id DESC"
        )
        .map_err(|e| e.to_string())?;
        
     let rows = stmt.query_map(params![session_id], |row| {
        let id: i32 = row.get(0)?;
        let collection_code: Option<String> = row.get(1)?;
        let catalog_number: Option<String> = row.get(2)?;
        let duplicates: Option<i32> = row.get(3)?;
        let record_number: Option<String> = row.get(4)?;
        let recorded_by: Option<String> = row.get(5)?;
        let verbatim_event_date: Option<String> = row.get(6)?;
        let year: Option<i32> = row.get(7)?;
        let month: Option<i32> = row.get(8)?;
        let day: Option<i32> = row.get(9)?;
        let country: Option<String> = row.get(10)?;
        let state_province: Option<String> = row.get(11)?;
        let county: Option<String> = row.get(12)?;
        let municipality: Option<String> = row.get(13)?;
        let locality: Option<String> = row.get(14)?;
        let location_notes: Option<String> = row.get(15)?;
        let verbatim_coordinates: Option<String> = row.get(16)?;
        let decimal_latitude: Option<f64> = row.get(17)?;
        let decimal_longitude: Option<f64> = row.get(18)?;
        let verbatim_elevation: Option<String> = row.get(19)?;
        let habitat: Option<String> = row.get(20)?;
        let occurrence_remarks: Option<String> = row.get(21)?;
        let field_notes: Option<String> = row.get(22)?;
        let type_status: Option<String> = row.get(23)?;
        let id_qualifier: Option<String> = row.get(24)?;
        let scientific_name: Option<String> = row.get(25)?;
        let identified_by: Option<String> = row.get(26)?;
        let year_identified: Option<i32> = row.get(27)?;
        let month_identified: Option<i32> = row.get(28)?;
        let day_identified: Option<i32> = row.get(29)?;
        let id_remarks: Option<String> = row.get(30)?;
        let taxon_id: Option<String> = row.get(31)?;
        let cultivated: Option<i32> = row.get(32)?;
        
        Ok(json!({
            "id": id,
            "sessionId": session_id,
            "collectionCode": collection_code.unwrap_or_default(),
            "catalogNumber": catalog_number.unwrap_or_default(),
            "duplicates": duplicates,
            "recordNumber": record_number.unwrap_or_default(),
            "recordedBy": recorded_by.unwrap_or_default(),
            "verbatimEventDate": verbatim_event_date.unwrap_or_default(),
            "year": year,
            "month": month,
            "day": day,
            "country": country.unwrap_or_default(),
            "stateProvince": state_province.unwrap_or_default(),
            "county": county.unwrap_or_default(),
            "municipality": municipality.unwrap_or_default(),
            "locality": locality.unwrap_or_default(),
            "locationNotes": location_notes.unwrap_or_default(), // locationRemarks mapped to locationNotes
            "verbatimCoordinates": verbatim_coordinates.unwrap_or_default(),
            "decimalLatitude": decimal_latitude,
            "decimalLongitude": decimal_longitude,
            "verbatimElevation": verbatim_elevation.unwrap_or_default(),
            "habitat": habitat.unwrap_or_default(),
            "occurrenceRemarks": occurrence_remarks.unwrap_or_default(),
            "fieldNotes": field_notes.unwrap_or_default(),
            "typeStatus": type_status.unwrap_or_default(),
            "identificationQualifier": id_qualifier.unwrap_or_default(),
            "scientificName": scientific_name.unwrap_or_default(),
            "identifiedBy": identified_by.unwrap_or_default(),
            "yearIdentified": year_identified,
            "monthIdentified": month_identified,
            "dayIdentified": day_identified,
            "identificationRemarks": id_remarks.unwrap_or_default(),
            "taxonID": taxon_id.unwrap_or_default(),
            "cultivated": cultivated.unwrap_or(0) == 1,
        }))
    }).map_err(|e| e.to_string())?;
    
    let mut list = Vec::new();
    for r in rows {
        list.push(r.map_err(|e| e.to_string())?);
    }
    Ok(list)
}

#[tauri::command]
fn delete_captured_record(app: AppHandle, id: i32) -> Result<(), String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    conn.execute("DELETE FROM captured_records WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
        
    Ok(())
}

#[tauri::command]
fn delete_session(app: AppHandle, id: i32) -> Result<(), String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    // Explicitly delete all captured records belonging to this session first
    conn.execute("DELETE FROM captured_records WHERE session_id = ?1", params![id])
        .map_err(|e| e.to_string())?;
        
    // Delete the session itself
    conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
        
    Ok(())
}

#[tauri::command]
fn rename_session(app: AppHandle, id: i32, name: String) -> Result<(), String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let name_clean = name.trim();
    if name_clean.is_empty() {
        return Err("Session name cannot be empty.".to_string());
    }
    
    conn.execute(
        "UPDATE sessions SET name = ?1 WHERE id = ?2",
        params![name_clean, id],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}


#[tauri::command]
fn save_export_settings(app: AppHandle, user_id: i32, format: String, mappings: String) -> Result<(), String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    conn.execute(
        "INSERT INTO export_settings (user_id, format, mappings) 
         VALUES (?1, ?2, ?3) 
         ON CONFLICT(user_id) DO UPDATE SET format=?2, mappings=?3",
        params![user_id, format, mappings],
    ).map_err(|e| e.to_string())?;
    
    Ok(())
}

#[tauri::command]
fn get_export_settings(app: AppHandle, user_id: i32) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let mut stmt = conn
        .prepare("SELECT format, mappings FROM export_settings WHERE user_id = ?1")
        .map_err(|e| e.to_string())?;
        
    let mut rows = stmt
        .query_map(params![user_id], |row| {
            let format: String = row.get(0)?;
            let mappings: String = row.get(1)?;
            Ok(json!({ "format": format, "mappings": mappings }))
        })
        .map_err(|e| e.to_string())?;
        
    if let Some(row) = rows.next() {
        let settings = row.map_err(|e| e.to_string())?;
        Ok(settings)
    } else {
        // Return default empty mappings
        Ok(json!({ "format": "DwC", "mappings": "{}" }))
    }
}

#[tauri::command]
fn get_table_counts(app: AppHandle) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let gbif_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM gbif", [], |r| r.get(0))
        .unwrap_or(0);
        
    let wcvp_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM wcvp_taxonomy", [], |r| r.get(0))
        .unwrap_or(0);
        
    Ok(json!({
        "gbif": gbif_count,
        "wcvp": wcvp_count
    }))
}

#[tauri::command]
fn select_export_path(default_name: String) -> Option<String> {
    let file = rfd::FileDialog::new()
        .set_file_name(&default_name)
        .add_filter("CSV File", &["csv"])
        .save_file();
        
    file.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn export_session_csv(app: AppHandle, session_id: i32, filepath: String, csv_content: String) -> Result<String, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    fs::write(&filepath, csv_content).map_err(|e| e.to_string())?;
    
    conn.execute(
        "UPDATE sessions SET last_exported_at = CURRENT_TIMESTAMP WHERE id = ?1",
        params![session_id],
    ).map_err(|e| e.to_string())?;
    
    Ok(format!("Successfully exported records to {}", filepath))
}

fn find_family_recursive(conn: &Connection, start_id: &str) -> Result<Option<String>, rusqlite::Error> {
    let mut current_id = start_id.to_string();
    let mut depth = 0;
    
    while depth < 30 {
        let mut stmt = conn.prepare(
            "SELECT plant_name_id, parent_plant_name_id, taxon_rank, taxon_name 
             FROM wcvp_taxonomy 
             WHERE plant_name_id = ?1"
        )?;
        
        let row = stmt.query_row(params![current_id], |r| {
            let pid: Option<String> = r.get(0)?;
            let parent_id: Option<String> = r.get(1)?;
            let rank: Option<String> = r.get(2)?;
            let name: Option<String> = r.get(3)?;
            Ok((pid, parent_id, rank, name))
        });
        
        match row {
            Ok((_, parent_id, rank, name)) => {
                if let Some(r) = rank {
                    if r.eq_ignore_ascii_case("Family") {
                        return Ok(name);
                    }
                }
                if let Some(p) = parent_id {
                    if p.is_empty() || p == current_id {
                        break;
                    }
                    current_id = p;
                } else {
                    break;
                }
            }
            Err(_) => break,
        }
        depth += 1;
    }
    
    Ok(None)
}

#[tauri::command]
fn resolve_wcvp_families(app: AppHandle, queries: Vec<serde_json::Value>) -> Result<serde_json::Value, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let mut results = serde_json::Map::new();
    let mut cache: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    
    for query in queries {
        let rec_id = query.get("id").and_then(|v| v.as_i64()).unwrap_or(0).to_string();
        let taxon_id = query.get("taxonID").and_then(|v| v.as_str()).map(|s| s.trim());
        let scientific_name = query.get("scientificName").and_then(|v| v.as_str()).unwrap_or("").trim();
        
        let mut resolved_family = None;
        
        // 1. If taxonID is present
        if let Some(tid) = taxon_id {
            if !tid.is_empty() {
                if let Some(cached) = cache.get(tid) {
                    resolved_family = Some(cached.clone());
                } else {
                    if let Ok(Some(fam)) = find_family_recursive(&conn, tid) {
                        cache.insert(tid.to_string(), fam.clone());
                        resolved_family = Some(fam);
                    }
                }
            }
        }
        
        // 2. If taxonID is absent or recursive search failed
        if resolved_family.is_none() && !scientific_name.is_empty() {
            if let Some(first_word) = scientific_name.split_whitespace().next() {
                let first_word_clean = first_word.trim_matches(|c: char| c.is_ascii_punctuation());
                if !first_word_clean.is_empty() {
                    if first_word_clean.to_ascii_lowercase().ends_with("ceae") {
                        resolved_family = Some(first_word_clean.to_string());
                    } else {
                        // Treat as genus
                        if let Some(cached) = cache.get(first_word_clean) {
                            resolved_family = Some(cached.clone());
                        } else {
                            let genus_pattern = format!("{} %", first_word_clean);
                            let mut stmt = conn.prepare(
                                "SELECT plant_name_id FROM wcvp_taxonomy WHERE taxon_name LIKE ?1 LIMIT 1"
                            ).map_err(|e| e.to_string())?;
                            
                            let start_id: Option<String> = stmt.query_row(params![genus_pattern], |r| r.get(0)).ok();
                            if let Some(sid) = start_id {
                                if let Ok(Some(fam)) = find_family_recursive(&conn, &sid) {
                                    cache.insert(first_word_clean.to_string(), fam.clone());
                                    resolved_family = Some(fam);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        results.insert(rec_id, serde_json::Value::String(resolved_family.unwrap_or_default()));
    }
    
    Ok(serde_json::Value::Object(results))
}


#[tauri::command]
fn autocomplete_geography(
    app: AppHandle,
    field: String,
    query: String,
    country: String,
    state_province: String,
    county: String,
) -> Result<Vec<String>, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    let q = query.trim();
    let c = country.trim();
    let sp = state_province.trim();
    let co = county.trim();

    let mut sql = String::new();
    let mut params_vec: Vec<String> = vec![format!("{}%", q)];

    match field.as_str() {
        "country" => {
            sql.push_str(
                "SELECT DISTINCT country \
                 FROM gbif \
                 WHERE country LIKE ?1 COLLATE NOCASE \
                   AND country != '' \
                   AND country IS NOT NULL \
                 ORDER BY country ASC \
                 LIMIT 15",
            );
        }
        "stateProvince" => {
            sql.push_str(
                "SELECT DISTINCT stateProvince \
                 FROM gbif \
                 WHERE stateProvince LIKE ?1 COLLATE NOCASE \
                   AND stateProvince != '' \
                   AND stateProvince IS NOT NULL \
                   AND (?2 = '' OR country = ?2 COLLATE NOCASE) \
                 ORDER BY stateProvince ASC \
                 LIMIT 15",
            );
            params_vec.push(c.to_string());
        }
        "county" => {
            sql.push_str(
                "SELECT DISTINCT county \
                 FROM gbif \
                 WHERE county LIKE ?1 COLLATE NOCASE \
                   AND county != '' \
                   AND county IS NOT NULL \
                   AND (?2 = '' OR country = ?2 COLLATE NOCASE) \
                   AND (?3 = '' OR stateProvince = ?3 COLLATE NOCASE) \
                 ORDER BY county ASC \
                 LIMIT 15",
            );
            params_vec.push(c.to_string());
            params_vec.push(sp.to_string());
        }
        "municipality" => {
            sql.push_str(
                "SELECT DISTINCT municipality \
                 FROM gbif \
                 WHERE municipality LIKE ?1 COLLATE NOCASE \
                   AND municipality != '' \
                   AND municipality IS NOT NULL \
                   AND (?2 = '' OR country = ?2 COLLATE NOCASE) \
                   AND (?3 = '' OR stateProvince = ?3 COLLATE NOCASE) \
                   AND (?4 = '' OR county = ?4 COLLATE NOCASE) \
                 ORDER BY municipality ASC \
                 LIMIT 15",
            );
            params_vec.push(c.to_string());
            params_vec.push(sp.to_string());
            params_vec.push(co.to_string());
        }
        _ => return Err(format!("Invalid geography field: {}", field)),
    }

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    
    // Map params dynamically
    let rusql_params: Vec<Box<dyn rusqlite::ToSql>> = params_vec.iter().map(|v| {
        Box::new(v.clone()) as Box<dyn rusqlite::ToSql>
    }).collect();
    let ref_params: Vec<&dyn rusqlite::ToSql> = rusql_params.iter().map(|b| b.as_ref()).collect();

    let rows = stmt.query_map(&ref_params[..], |row| {
        let val: Option<String> = row.get(0)?;
        Ok(val.unwrap_or_default())
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        let s = r.map_err(|e| e.to_string())?;
        if !s.is_empty() {
            list.push(s);
        }
    }

    Ok(list)
}

// -------------------------------------------------------------
// Tauri Application Runner
// -------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_log::Builder::new()
            .targets([
                tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                    file_name: Some("app".to_string()),
                }),
            ])
            .build())
        .setup(|_app| {
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            initialize_database,
            register_user,
            login_user,
            create_session,
            get_sessions,
            search_reference,
            autocomplete_scientific_name,
            autocomplete_recorded_by,
            autocomplete_agent,
            check_agent_exists,
            add_agent,
            autocomplete_locality,
            save_captured_record,
            get_captured_records,
            delete_captured_record,
            delete_session,
            rename_session,
            select_export_path,
            save_export_settings,
            get_export_settings,
            export_session_csv,
            autocomplete_geography,
            get_table_counts,
            resolve_wcvp_families
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            shutdown_database(app_handle);
        }
    });
}
