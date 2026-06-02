// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::AppHandle;
use rusqlite::{Connection, params};
use serde_json::json;
use std::fs;

mod parser;
mod db;

use parser::{normalize_taxon_name, normalize_collector_search};
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
    
    // Constraint: family search must be accompanied by collector
    if has_family && !has_recorded_by {
        return Err("Family search must be accompanied by a Collector name.".to_string());
    }

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
        "SELECT recordedBy, recordNumber, locality, locationRemarks, verbatimLocality, 
                scientificName, family, country, stateProvince, year, month, day 
         FROM gbif WHERE 1=1"
    );
    let mut params_vec: Vec<serde_json::Value> = Vec::new();
    
    if has_recorded_by {
        let normalized = normalize_collector_search(recorded_by);
        sql.push_str(" AND normalizedRecordedBy LIKE ? COLLATE NOCASE");
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
    
    // Locality FTS5 Search (multi-term prefix match across locality, locationRemarks, verbatimLocality)
    if has_locality {
        let terms: Vec<&str> = locality.split_whitespace().collect();
        if !terms.is_empty() {
            let mut match_clauses = Vec::new();
            for term in terms {
                let clean_term = term.trim_matches(|c: char| c.is_ascii_punctuation());
                if !clean_term.is_empty() {
                    match_clauses.push(format!(
                        "(locality:{term}* OR locationRemarks:{term}* OR verbatimLocality:{term}*)",
                        term = clean_term
                    ));
                }
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
        
        Ok(json!({
            "taxonID": id,
            "scientificName": name,
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
    
    let normalized = normalize_collector_search(q_clean);
    
    // We aggregate unique collector names from recordedBy, using normalizedRecordedBy for lookup.
    // The user must only ever see values from recordedBy.
    let mut stmt = conn
        .prepare(
            "SELECT DISTINCT collector FROM (
                SELECT recordedBy AS collector FROM gbif WHERE normalizedRecordedBy LIKE ?1 COLLATE NOCASE
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
                SELECT locality FROM gbif WHERE locality LIKE ?1
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
    let verbatim_elevation = record.get("verbatimElevation").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let habitat = record.get("habitat").and_then(|v| v.as_str()).unwrap_or("").trim();
    let occurrence_remarks = record.get("occurrenceRemarks").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let type_status = record.get("typeStatus").and_then(|v| v.as_str()).unwrap_or("").trim();
    let id_qualifier = record.get("identificationQualifier").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let scientific_name = record.get("scientificName").and_then(|v| v.as_str()).unwrap_or("").trim();
    let identified_by = record.get("identifiedBy").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let year_identified = record.get("yearIdentified").and_then(|v| v.as_i64());
    let month_identified = record.get("monthIdentified").and_then(|v| v.as_i64());
    let day_identified = record.get("dayIdentified").and_then(|v| v.as_i64());
    
    let id_remarks = record.get("identificationRemarks").and_then(|v| v.as_str()).unwrap_or("").trim();
    let taxon_id = record.get("taxonID").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    if let Some(existing_id) = id {
        // Update existing record
        conn.execute(
            "UPDATE captured_records SET 
                collectionCode=?1, catalogNumber=?2, duplicates=?3, recordNumber=?4, recordedBy=?5,
                verbatimEventDate=?6, year=?7, month=?8, day=?9, country=?10,
                stateProvince=?11, county=?12, municipality=?13, locality=?14, locationRemarks=?15,
                verbatimCoordinates=?16, verbatimElevation=?17, habitat=?18, occurrenceRemarks=?19, typeStatus=?20,
                identificationQualifier=?21, scientificName=?22, identifiedBy=?23, yearIdentified=?24, monthIdentified=?25,
                dayIdentified=?26, identificationRemarks=?27, taxonID=?28
             WHERE id = ?29 AND session_id = ?30",
            params![
                collection_code, catalog_number, duplicates, record_number, recorded_by,
                verbatim_event_date, year, month, day, country,
                state_province, county, municipality, locality, location_remarks,
                verbatim_coordinates, verbatim_elevation, habitat, occurrence_remarks, type_status,
                id_qualifier, scientific_name, identified_by, year_identified, month_identified,
                day_identified, id_remarks, taxon_id, existing_id, session_id
            ]
        ).map_err(|e| e.to_string())?;
        
        Ok(json!({ "id": existing_id, "success": true }))
    } else {
        // Insert new record
        conn.execute(
            "INSERT INTO captured_records (
                session_id, collectionCode, catalogNumber, duplicates, recordNumber, recordedBy,
                verbatimEventDate, year, month, day, country, stateProvince, county, municipality,
                locality, locationRemarks, verbatimCoordinates, verbatimElevation, habitat, occurrenceRemarks,
                typeStatus, identificationQualifier, scientificName, identifiedBy, yearIdentified,
                monthIdentified, dayIdentified, identificationRemarks, taxonID
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29)",
            params![
                session_id, collection_code, catalog_number, duplicates, record_number, recorded_by,
                verbatim_event_date, year, month, day, country, state_province, county, municipality,
                locality, location_remarks, verbatim_coordinates, verbatim_elevation, habitat, occurrence_remarks,
                type_status, id_qualifier, scientific_name, identified_by, year_identified,
                month_identified, day_identified, id_remarks, taxon_id
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
                    locality, locationRemarks, verbatimCoordinates, verbatimElevation, habitat, occurrenceRemarks, 
                    typeStatus, identificationQualifier, scientificName, identifiedBy, yearIdentified, 
                    monthIdentified, dayIdentified, identificationRemarks, taxonID 
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
        let verbatim_elevation: Option<String> = row.get(17)?;
        let habitat: Option<String> = row.get(18)?;
        let occurrence_remarks: Option<String> = row.get(19)?;
        let type_status: Option<String> = row.get(20)?;
        let id_qualifier: Option<String> = row.get(21)?;
        let scientific_name: Option<String> = row.get(22)?;
        let identified_by: Option<String> = row.get(23)?;
        let year_identified: Option<i32> = row.get(24)?;
        let month_identified: Option<i32> = row.get(25)?;
        let day_identified: Option<i32> = row.get(26)?;
        let id_remarks: Option<String> = row.get(27)?;
        let taxon_id: Option<String> = row.get(28)?;
        
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
            "verbatimElevation": verbatim_elevation.unwrap_or_default(),
            "habitat": habitat.unwrap_or_default(),
            "occurrenceRemarks": occurrence_remarks.unwrap_or_default(),
            "typeStatus": type_status.unwrap_or_default(),
            "identificationQualifier": id_qualifier.unwrap_or_default(),
            "scientificName": scientific_name.unwrap_or_default(),
            "identifiedBy": identified_by.unwrap_or_default(),
            "yearIdentified": year_identified,
            "monthIdentified": month_identified,
            "dayIdentified": day_identified,
            "identificationRemarks": id_remarks.unwrap_or_default(),
            "taxonID": taxon_id.unwrap_or_default(),
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
fn select_export_path(default_name: String) -> Option<String> {
    let file = rfd::FileDialog::new()
        .set_file_name(&default_name)
        .add_filter("CSV File", &["csv"])
        .save_file();
        
    file.map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn export_session_csv(app: AppHandle, session_id: i32, filepath: String) -> Result<String, String> {
    let db_path = get_db_path(&app);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    
    // We select every column exactly as defined in the database
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, collectionCode, catalogNumber, duplicates, recordNumber, 
                    recordedBy, verbatimEventDate, year, month, day, country, stateProvince, 
                    county, municipality, locality, locationRemarks, verbatimCoordinates, 
                    verbatimElevation, habitat, occurrenceRemarks, typeStatus, identificationQualifier, 
                    scientificName, identifiedBy, yearIdentified, monthIdentified, dayIdentified, 
                    identificationRemarks, taxonID, created_at, modified_at 
             FROM captured_records 
             WHERE session_id = ?1 
             ORDER BY id ASC"
        )
        .map_err(|e| e.to_string())?;
        
    let rows = stmt.query_map(params![session_id], |row| {
        let id: Option<i32> = row.get(0)?;
        let session_id: Option<i32> = row.get(1)?;
        let collection_code: Option<String> = row.get(2)?;
        let catalog_number: Option<String> = row.get(3)?;
        let duplicates: Option<i32> = row.get(4)?;
        let record_number: Option<String> = row.get(5)?;
        let recorded_by: Option<String> = row.get(6)?;
        let verbatim_event_date: Option<String> = row.get(7)?;
        let year: Option<i32> = row.get(8)?;
        let month: Option<i32> = row.get(9)?;
        let day: Option<i32> = row.get(10)?;
        let country: Option<String> = row.get(11)?;
        let state_province: Option<String> = row.get(12)?;
        let county: Option<String> = row.get(13)?;
        let municipality: Option<String> = row.get(14)?;
        let locality: Option<String> = row.get(15)?;
        let location_remarks: Option<String> = row.get(16)?;
        let verbatim_coordinates: Option<String> = row.get(17)?;
        let verbatim_elevation: Option<String> = row.get(18)?;
        let habitat: Option<String> = row.get(19)?;
        let occurrence_remarks: Option<String> = row.get(20)?;
        let type_status: Option<String> = row.get(21)?;
        let id_qualifier: Option<String> = row.get(22)?;
        let scientific_name: Option<String> = row.get(23)?;
        let identified_by: Option<String> = row.get(24)?;
        let year_identified: Option<i32> = row.get(25)?;
        let month_identified: Option<i32> = row.get(26)?;
        let day_identified: Option<i32> = row.get(27)?;
        let id_remarks: Option<String> = row.get(28)?;
        let taxon_id: Option<String> = row.get(29)?;
        let created_at: Option<String> = row.get(30)?;
        let modified_at: Option<String> = row.get(31)?;
        
        Ok(vec![
            id.map(|x| x.to_string()).unwrap_or_default(),
            session_id.map(|x| x.to_string()).unwrap_or_default(),
            collection_code.unwrap_or_default(),
            catalog_number.unwrap_or_default(),
            duplicates.map(|x| x.to_string()).unwrap_or_default(),
            record_number.unwrap_or_default(),
            recorded_by.unwrap_or_default(),
            verbatim_event_date.unwrap_or_default(),
            year.map(|x| x.to_string()).unwrap_or_default(),
            month.map(|x| x.to_string()).unwrap_or_default(),
            day.map(|x| x.to_string()).unwrap_or_default(),
            country.unwrap_or_default(),
            state_province.unwrap_or_default(),
            county.unwrap_or_default(),
            municipality.unwrap_or_default(),
            locality.unwrap_or_default(),
            location_remarks.unwrap_or_default(),
            verbatim_coordinates.unwrap_or_default(),
            verbatim_elevation.unwrap_or_default(),
            habitat.unwrap_or_default(),
            occurrence_remarks.unwrap_or_default(),
            type_status.unwrap_or_default(),
            id_qualifier.unwrap_or_default(),
            scientific_name.unwrap_or_default(),
            identified_by.unwrap_or_default(),
            year_identified.map(|x| x.to_string()).unwrap_or_default(),
            month_identified.map(|x| x.to_string()).unwrap_or_default(),
            day_identified.map(|x| x.to_string()).unwrap_or_default(),
            id_remarks.unwrap_or_default(),
            taxon_id.unwrap_or_default(),
            created_at.unwrap_or_default(),
            modified_at.unwrap_or_default(),
        ])
    }).map_err(|e| e.to_string())?;
    
    let mut records = Vec::new();
    for r in rows {
        records.push(r.map_err(|e| e.to_string())?);
    }
    
    if records.is_empty() {
        return Err("No records to export in this session.".to_string());
    }
    
    let headers = vec![
        "id", "session_id", "collectionCode", "catalogNumber", "duplicates", "recordNumber",
        "recordedBy", "verbatimEventDate", "year", "month", "day", "country", "stateProvince",
        "county", "municipality", "locality", "locationRemarks", "verbatimCoordinates",
        "verbatimElevation", "habitat", "occurrenceRemarks", "typeStatus", "identificationQualifier",
        "scientificName", "identifiedBy", "yearIdentified", "monthIdentified", "dayIdentified",
        "identificationRemarks", "taxonID", "created_at", "modified_at"
    ];
    
    let mut csv_content = String::new();
    
    // Header
    let escaped_headers: Vec<String> = headers.iter().map(|h| format!("\"{}\"", h.replace("\"", "\"\""))).collect();
    csv_content.push_str(&escaped_headers.join(","));
    csv_content.push('\n');
    
    // Rows
    for rec in records {
        let escaped_fields: Vec<String> = rec.iter().map(|f| format!("\"{}\"", f.replace("\"", "\"\""))).collect();
        csv_content.push_str(&escaped_fields.join(","));
        csv_content.push('\n');
    }
    
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
            delete_session,
            select_export_path,
            save_export_settings,
            get_export_settings,
            export_session_csv
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
