// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::AppHandle;
use rusqlite::{Connection, params};
use serde_json::json;
use std::fs;

mod parser;
mod db;

use parser::normalize_taxon_name;
use db::{get_db_path, init_database, hash_password};

// -------------------------------------------------------------
// Tauri Command Handlers
// -------------------------------------------------------------

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
            "SELECT s.id, s.name, COUNT(r.id) as count 
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
        Ok(json!({
            "id": id,
            "name": name,
            "recordCount": record_count
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
    let has_date = year.is_some() || month.is_some() || day.is_some();
    
    let other_fields_present = has_record_number || has_locality || has_scientific_name || has_family || has_country || has_state_province;
    
    // Constraint: collector must be accompanied by at least one other field
    if has_recorded_by && !other_fields_present && !has_date {
        return Err("Collector search must be accompanied by at least one other field (e.g. locality, country, taxon, or date).".to_string());
    }
    
    // Constraint: date searches must be accompanied by taxon name, family, collector, or locality
    if has_date && !(has_scientific_name || has_family || has_locality || has_recorded_by) {
        return Err("Date queries require a Taxon Name, Family, Collector, or Locality to prevent excessive database hits.".to_string());
    }
    
    if !has_recorded_by && !other_fields_present && !has_date {
        return Err("Please enter at least one query search field.".to_string());
    }
    
    let mut sql = String::from(
        "SELECT id, recordedBy, recordNumber, locality, locationNotes, verbatimLocality, 
                scientificName, family, genus, specificEpithet, infraSpecificEpithet, 
                country, stateProvince, year, month, day 
         FROM parsed_gbif WHERE 1=1"
    );
    let mut params_vec: Vec<serde_json::Value> = Vec::new();
    
    if has_recorded_by {
        sql.push_str(" AND recordedBy LIKE ?");
        params_vec.push(json!(format!("%{}%", recorded_by)));
    }
    if has_record_number {
        sql.push_str(" AND recordNumber LIKE ?");
        params_vec.push(json!(format!("{}%", record_number)));
    }
    if has_family {
        sql.push_str(" AND family LIKE ?");
        params_vec.push(json!(format!("{}%", family)));
    }
    if has_country {
        sql.push_str(" AND country LIKE ?");
        params_vec.push(json!(format!("{}%", country)));
    }
    if has_state_province {
        sql.push_str(" AND stateProvince LIKE ?");
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
    
    // Locality FTS5 Search (multi-term prefix match across locality, locationNotes, verbatimLocality)
    if has_locality {
        let terms: Vec<&str> = locality.split_whitespace().collect();
        if !terms.is_empty() {
            let mut match_clauses = Vec::new();
            for term in terms {
                let clean_term = term.trim_matches(|c: char| c.is_ascii_punctuation());
                if !clean_term.is_empty() {
                    match_clauses.push(format!(
                        "(locality:{term}* OR locationNotes:{term}* OR verbatimLocality:{term}*)",
                        term = clean_term
                    ));
                }
            }
            if !match_clauses.is_empty() {
                let fts_query = match_clauses.join(" AND ");
                sql.push_str(" AND id IN (SELECT rowid FROM parsed_gbif_fts WHERE parsed_gbif_fts MATCH ?)");
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
            sql.push_str(" AND id IN (SELECT rowid FROM parsed_gbif_fts WHERE parsed_gbif_fts MATCH ?)");
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
        let id: i32 = row.get(0)?;
        let recorded_by: Option<String> = row.get(1)?;
        let record_number: Option<String> = row.get(2)?;
        let locality: Option<String> = row.get(3)?;
        let location_notes: Option<String> = row.get(4)?;
        let verbatim_locality: Option<String> = row.get(5)?;
        let scientific_name: Option<String> = row.get(6)?;
        let family: Option<String> = row.get(7)?;
        let genus: Option<String> = row.get(8)?;
        let specific_epithet: Option<String> = row.get(9)?;
        let infra_specific_epithet: Option<String> = row.get(10)?;
        let country: Option<String> = row.get(11)?;
        let state_province: Option<String> = row.get(12)?;
        let year: Option<i32> = row.get(13)?;
        let month: Option<i32> = row.get(14)?;
        let day: Option<i32> = row.get(15)?;
        
        Ok(json!({
            "id": id,
            "recordedBy": recorded_by.unwrap_or_default(),
            "recordNumber": record_number.unwrap_or_default(),
            "locality": locality.unwrap_or_default(),
            "locationNotes": location_notes.unwrap_or_default(),
            "verbatimLocality": verbatim_locality.unwrap_or_default(),
            "scientificName": scientific_name.unwrap_or_default(),
            "family": family.unwrap_or_default(),
            "genus": genus.unwrap_or_default(),
            "specificEpithet": specific_epithet.unwrap_or_default(),
            "infraSpecificEpithet": infra_specific_epithet.unwrap_or_default(),
            "country": country.unwrap_or_default(),
            "stateProvince": state_province.unwrap_or_default(),
            "year": year,
            "month": month,
            "day": day,
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
    
    let normalized = normalize_taxon_name(q_clean);
    let mut stmt = conn
        .prepare(
            "SELECT scientific_name, family, genus, species, authors, rank 
             FROM wcvp_taxonomy 
             WHERE normalized_name LIKE ?1 OR scientific_name LIKE ?2 
             LIMIT 15"
        )
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![format!("{}%", normalized), format!("{}%", q_clean)], |row| {
        let name: String = row.get(0)?;
        let family: String = row.get(1)?;
        let genus: String = row.get(2)?;
        let species: String = row.get(3)?;
        let authors: String = row.get(4)?;
        let rank: String = row.get(5)?;
        Ok(json!({
            "scientificName": name,
            "family": family,
            "genus": genus,
            "specificEpithet": species,
            "authors": authors,
            "rank": rank
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
    
    // Search both parsed_gbif and captured_records for unique collector names starting with query
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT recordedBy FROM (
                SELECT recordedBy FROM parsed_gbif WHERE recordedBy LIKE ?1
                UNION
                SELECT recordedBy FROM captured_records WHERE recordedBy LIKE ?1
             ) WHERE recordedBy IS NOT NULL AND recordedBy != '' LIMIT 10"
        )
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![format!("%{}%", q_clean)], |row| {
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
fn autocomplete_locality(app: AppHandle, query: String) -> Result<Vec<String>, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    let q_clean = query.trim();
    if q_clean.is_empty() {
        return Ok(Vec::new());
    }
    
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT locality FROM (
                SELECT locality FROM parsed_gbif WHERE locality LIKE ?1
                UNION
                SELECT locality FROM captured_records WHERE locality LIKE ?1
             ) WHERE locality IS NOT NULL AND locality != '' LIMIT 10"
        )
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![format!("%{}%", q_clean)], |row| {
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
    
    let recorded_by = record.get("recordedBy").and_then(|v| v.as_str()).unwrap_or("").trim();
    let record_number = record.get("recordNumber").and_then(|v| v.as_str()).unwrap_or("").trim();
    let locality = record.get("locality").and_then(|v| v.as_str()).unwrap_or("").trim();
    let location_notes = record.get("locationNotes").and_then(|v| v.as_str()).unwrap_or("").trim();
    let verbatim_locality = record.get("verbatimLocality").and_then(|v| v.as_str()).unwrap_or("").trim();
    let scientific_name = record.get("scientificName").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let family = record.get("family").and_then(|v| v.as_str()).unwrap_or("").trim();
    let genus = record.get("genus").and_then(|v| v.as_str()).unwrap_or("").trim();
    let specific_epithet = record.get("specificEpithet").and_then(|v| v.as_str()).unwrap_or("").trim();
    let infra_specific_epithet = record.get("infraSpecificEpithet").and_then(|v| v.as_str()).unwrap_or("").trim();
    let country = record.get("country").and_then(|v| v.as_str()).unwrap_or("").trim();
    let state_province = record.get("stateProvince").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let year = record.get("year").and_then(|v| v.as_i64());
    let month = record.get("month").and_then(|v| v.as_i64());
    let day = record.get("day").and_then(|v| v.as_i64());
    
    if let Some(existing_id) = id {
        // Update existing record
        conn.execute(
            "UPDATE captured_records SET 
                recordedBy=?1, recordNumber=?2, locality=?3, locationNotes=?4, verbatimLocality=?5,
                scientificName=?6, family=?7, genus=?8, specificEpithet=?9, infraSpecificEpithet=?10,
                country=?11, stateProvince=?12, year=?13, month=?14, day=?15, updated_at=CURRENT_TIMESTAMP
             WHERE id = ?16 AND session_id = ?17",
            params![
                recorded_by, record_number, locality, location_notes, verbatim_locality,
                scientific_name, family, genus, specific_epithet, infra_specific_epithet,
                country, state_province, year, month, day, existing_id, session_id
            ]
        ).map_err(|e| e.to_string())?;
        
        Ok(json!({ "id": existing_id, "success": true }))
    } else {
        // Insert new record
        conn.execute(
            "INSERT INTO captured_records (
                session_id, recordedBy, recordNumber, locality, locationNotes, verbatimLocality,
                scientificName, family, genus, specificEpithet, infraSpecificEpithet,
                country, stateProvince, year, month, day
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                session_id, recorded_by, record_number, locality, location_notes, verbatim_locality,
                scientific_name, family, genus, specific_epithet, infra_specific_epithet,
                country, state_province, year, month, day
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
            "SELECT id, recordedBy, recordNumber, locality, locationNotes, verbatimLocality, 
                    scientificName, family, genus, specificEpithet, infraSpecificEpithet, 
                    country, stateProvince, year, month, day 
             FROM captured_records 
             WHERE session_id = ?1 
             ORDER BY id DESC"
        )
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![session_id], |row| {
        let id: i32 = row.get(0)?;
        let recorded_by: Option<String> = row.get(1)?;
        let record_number: Option<String> = row.get(2)?;
        let locality: Option<String> = row.get(3)?;
        let location_notes: Option<String> = row.get(4)?;
        let verbatim_locality: Option<String> = row.get(5)?;
        let scientific_name: Option<String> = row.get(6)?;
        let family: Option<String> = row.get(7)?;
        let genus: Option<String> = row.get(8)?;
        let specific_epithet: Option<String> = row.get(9)?;
        let infra_specific_epithet: Option<String> = row.get(10)?;
        let country: Option<String> = row.get(11)?;
        let state_province: Option<String> = row.get(12)?;
        let year: Option<i32> = row.get(13)?;
        let month: Option<i32> = row.get(14)?;
        let day: Option<i32> = row.get(15)?;
        
        Ok(json!({
            "id": id,
            "sessionId": session_id,
            "recordedBy": recorded_by.unwrap_or_default(),
            "recordNumber": record_number.unwrap_or_default(),
            "locality": locality.unwrap_or_default(),
            "locationNotes": location_notes.unwrap_or_default(),
            "verbatimLocality": verbatim_locality.unwrap_or_default(),
            "scientificName": scientific_name.unwrap_or_default(),
            "family": family.unwrap_or_default(),
            "genus": genus.unwrap_or_default(),
            "specificEpithet": specific_epithet.unwrap_or_default(),
            "infraSpecificEpithet": infra_specific_epithet.unwrap_or_default(),
            "country": country.unwrap_or_default(),
            "stateProvince": state_province.unwrap_or_default(),
            "year": year,
            "month": month,
            "day": day,
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
fn export_session_csv(app: AppHandle, session_id: i32, filepath: String) -> Result<String, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    // 1. Fetch captured records
    let mut stmt = conn
        .prepare(
            "SELECT recordedBy, recordNumber, locality, locationNotes, verbatimLocality, 
                    scientificName, family, genus, specificEpithet, infraSpecificEpithet, 
                    country, stateProvince, year, month, day 
             FROM captured_records 
             WHERE session_id = ?1 
             ORDER BY id ASC"
        )
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![session_id], |row| {
        let recorded_by: Option<String> = row.get(0)?;
        let record_number: Option<String> = row.get(1)?;
        let locality: Option<String> = row.get(2)?;
        let location_notes: Option<String> = row.get(3)?;
        let verbatim_locality: Option<String> = row.get(4)?;
        let scientific_name: Option<String> = row.get(5)?;
        let family: Option<String> = row.get(6)?;
        let genus: Option<String> = row.get(7)?;
        let specific_epithet: Option<String> = row.get(8)?;
        let infra_specific_epithet: Option<String> = row.get(9)?;
        let country: Option<String> = row.get(10)?;
        let state_province: Option<String> = row.get(11)?;
        let year: Option<i32> = row.get(12)?;
        let month: Option<i32> = row.get(13)?;
        let day: Option<i32> = row.get(14)?;
        
        Ok(vec![
            recorded_by.unwrap_or_default(),
            record_number.unwrap_or_default(),
            locality.unwrap_or_default(),
            location_notes.unwrap_or_default(),
            verbatim_locality.unwrap_or_default(),
            scientific_name.unwrap_or_default(),
            family.unwrap_or_default(),
            genus.unwrap_or_default(),
            specific_epithet.unwrap_or_default(),
            infra_specific_epithet.unwrap_or_default(),
            country.unwrap_or_default(),
            state_province.unwrap_or_default(),
            year.map(|y| y.to_string()).unwrap_or_default(),
            month.map(|m| m.to_string()).unwrap_or_default(),
            day.map(|d| d.to_string()).unwrap_or_default(),
        ])
    }).map_err(|e| e.to_string())?;
    
    let mut records = Vec::new();
    for r in rows {
        records.push(r.map_err(|e| e.to_string())?);
    }
    
    if records.is_empty() {
        return Err("No records to export in this session.".to_string());
    }
    
    // Standard headers
    let dwc_headers = vec![
        "recordedBy", "recordNumber", "locality", "locationNotes", "verbatimLocality",
        "scientificName", "family", "genus", "specificEpithet", "infraSpecificEpithet",
        "country", "stateProvince", "year", "month", "day"
    ];
    
    // We'll read export settings to check if headers need mapping (e.g. BRAHMS format)
    // Fetch session's user_id first
    let user_id: i32 = conn.query_row(
        "SELECT user_id FROM sessions WHERE id = ?1",
        params![session_id],
        |r| r.get(0)
    ).map_err(|e| e.to_string())?;
    
    let mut mapped_headers: Vec<String> = dwc_headers.iter().map(|h| h.to_string()).collect();
    
    let mut stmt_settings = conn
        .prepare("SELECT format, mappings FROM export_settings WHERE user_id = ?1")
        .map_err(|e| e.to_string())?;
        
    let mut rows_settings = stmt_settings
        .query_map(params![user_id], |row| {
            let format: String = row.get(0)?;
            let mappings_str: String = row.get(1)?;
            Ok((format, mappings_str))
        })
        .map_err(|e| e.to_string())?;
        
    if let Some(r_set) = rows_settings.next() {
        let (format, mappings_str) = r_set.map_err(|e| e.to_string())?;
        if format == "BRAHMS" {
            // Apply a pre-defined BRAHMS mapping example
            mapped_headers = vec![
                "COLLECTOR", "NUMBER", "LOCALITY", "LOC_NOTES", "VERB_LOC",
                "TAXON", "FAMILY", "GENUS", "SPECIES", "INFRA_SP",
                "COUNTRY", "PROVINCE", "YEAR", "MONTH", "DAY"
            ].iter().map(|h| h.to_string()).collect();
        } else if let Ok(custom_maps) = serde_json::from_str::<serde_json::Value>(&mappings_str) {
            // If custom header mappings exist in JSON (e.g., {"recordedBy": "COLLECTOR_NAME"})
            for (i, header) in dwc_headers.iter().enumerate() {
                if let Some(mapped) = custom_maps.get(*header).and_then(|v| v.as_str()) {
                    if !mapped.trim().is_empty() {
                        mapped_headers[i] = mapped.to_string();
                    }
                }
            }
        }
    }
    
    // Build CSV content
    let mut csv_content = String::new();
    
    // Header
    let escaped_headers: Vec<String> = mapped_headers.iter().map(|h| format!("\"{}\"", h.replace("\"", "\"\""))).collect();
    csv_content.push_str(&escaped_headers.join(","));
    csv_content.push('\n');
    
    // Rows
    for rec in records {
        let escaped_fields: Vec<String> = rec.iter().map(|f| format!("\"{}\"", f.replace("\"", "\"\""))).collect();
        csv_content.push_str(&escaped_fields.join(","));
        csv_content.push('\n');
    }
    
    // Save to file
    fs::write(&filepath, csv_content).map_err(|e| e.to_string())?;
    
    Ok(format!("Successfully exported {} records to {}", escaped_headers.len(), filepath))
}

// -------------------------------------------------------------
// Tauri Application Runner
// -------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize database on startup
            if let Err(e) = init_database(app.handle()) {
                println!("Error initializing SQLite database: {}", e);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            register_user,
            login_user,
            create_session,
            get_sessions,
            search_reference,
            autocomplete_scientific_name,
            autocomplete_recorded_by,
            autocomplete_locality,
            save_captured_record,
            get_captured_records,
            delete_captured_record,
            save_export_settings,
            get_export_settings,
            export_session_csv
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
