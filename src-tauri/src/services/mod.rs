use crate::db::{get_connection, hash_password};
use crate::models::{
    CapturedRecord, ExportSettingsDto, LocalitySearchResult, ReferenceSpecimen, SessionDto,
    TaxonAutocompleteResult, UserDto,
};
use crate::parsers::split_names;
use crate::repositories::{
    AgentRepository, ExportRepository, GeographyRepository, ReferenceRepository, SessionRepository,
    SpecimenRepository, TaxonomyRepository, UserRepository,
};
use rusqlite::params;
use serde_json::json;
use std::fs;
use tauri::{AppHandle, Emitter};

pub struct AuthService;

impl AuthService {
    pub fn register_user(
        app: &AppHandle,
        username: &str,
        password: &str,
        given_name: &str,
        family_name: &str,
        initials: &str,
    ) -> Result<String, String> {
        let username_clean = username.trim();
        let given_name_clean = given_name.trim();
        let family_name_clean = family_name.trim();
        let initials_clean = initials.trim();

        if username_clean.is_empty() || password.is_empty() {
            return Err("Username and password cannot be empty.".to_string());
        }
        if given_name_clean.is_empty() || family_name_clean.is_empty() || initials_clean.is_empty()
        {
            return Err("Given name, family name, and initials cannot be empty.".to_string());
        }

        let hash = hash_password(password);
        let conn = get_connection(app)?;

        match UserRepository::insert_user(
            &conn,
            username_clean,
            &hash,
            given_name_clean,
            family_name_clean,
            initials_clean,
        ) {
            Ok(_) => Ok("User registered successfully!".to_string()),
            Err(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err("Username already exists. Please choose another.".to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn login_user(
        app: &AppHandle,
        username: &str,
        password: &str,
    ) -> Result<Option<UserDto>, String> {
        let username_clean = username.trim();
        let hash = hash_password(password);
        let conn = get_connection(app)?;

        UserRepository::get_user_by_credentials(&conn, username_clean, &hash)
            .map_err(|e| e.to_string())
    }
}

pub struct UserService;

impl UserService {
    pub fn get_user_by_id(app: &AppHandle, id: i32) -> Result<Option<UserDto>, String> {
        let conn = get_connection(app)?;
        UserRepository::get_user_by_id(&conn, id).map_err(|e| e.to_string())
    }

    pub fn get_all_users(app: &AppHandle, caller_id: i32) -> Result<Vec<UserDto>, String> {
        let conn = get_connection(app)?;
        let caller_is_admin =
            UserRepository::is_admin(&conn, caller_id).map_err(|e| e.to_string())?;
        if !caller_is_admin {
            return Err("Access denied: only admins can manage users.".to_string());
        }
        UserRepository::get_all_users(&conn).map_err(|e| e.to_string())
    }

    pub fn update_user_profile(
        app: &AppHandle,
        user_id: i32,
        given_name: &str,
        family_name: &str,
        initials: &str,
    ) -> Result<(), String> {
        let given_name_clean = given_name.trim();
        let family_name_clean = family_name.trim();
        let initials_clean = initials.trim();
        if given_name_clean.is_empty() || family_name_clean.is_empty() || initials_clean.is_empty()
        {
            return Err("Given name, family name, and initials cannot be empty.".to_string());
        }

        let conn = get_connection(app)?;
        UserRepository::update_user_profile(
            &conn,
            user_id,
            given_name_clean,
            family_name_clean,
            initials_clean,
        )
        .map_err(|e| e.to_string())
    }

    pub fn update_user_by_admin(
        app: &AppHandle,
        caller_id: i32,
        target_user_id: i32,
        given_name: &str,
        family_name: &str,
        initials: &str,
        is_admin: bool,
    ) -> Result<(), String> {
        let given_name_clean = given_name.trim();
        let family_name_clean = family_name.trim();
        let initials_clean = initials.trim();
        if given_name_clean.is_empty() || family_name_clean.is_empty() || initials_clean.is_empty()
        {
            return Err("Given name, family name, and initials cannot be empty.".to_string());
        }

        let conn = get_connection(app)?;
        let caller_is_admin =
            UserRepository::is_admin(&conn, caller_id).map_err(|e| e.to_string())?;
        if !caller_is_admin {
            return Err("Access denied: only admins can manage users.".to_string());
        }
        UserRepository::update_user_by_admin(
            &conn,
            target_user_id,
            given_name_clean,
            family_name_clean,
            initials_clean,
            is_admin,
        )
        .map_err(|e| e.to_string())
    }
}

pub struct SessionService;

impl SessionService {
    pub fn create_session(app: &AppHandle, user_id: i32, name: &str) -> Result<SessionDto, String> {
        let name_clean = name.trim();
        if name_clean.is_empty() {
            return Err("Session name cannot be empty.".to_string());
        }

        let conn = get_connection(app)?;
        let id = SessionRepository::create_session(&conn, user_id, name_clean)
            .map_err(|e| e.to_string())?;

        let initials =
            UserRepository::get_user_initials(&conn, user_id).map_err(|e| e.to_string())?;

        Ok(SessionDto {
            id,
            name: name_clean.to_string(),
            record_count: 0,
            last_record_at: None,
            last_exported_at: None,
            created_by: Some(initials),
        })
    }

    pub fn get_sessions(app: &AppHandle, user_id: i32) -> Result<Vec<SessionDto>, String> {
        let conn = get_connection(app)?;
        SessionRepository::get_sessions(&conn, user_id).map_err(|e| e.to_string())
    }

    pub fn rename_session(app: &AppHandle, id: i32, name: &str) -> Result<(), String> {
        let name_clean = name.trim();
        if name_clean.is_empty() {
            return Err("Session name cannot be empty.".to_string());
        }

        let conn = get_connection(app)?;
        SessionRepository::rename_session(&conn, id, name_clean).map_err(|e| e.to_string())
    }

    pub fn delete_session(app: &AppHandle, id: i32) -> Result<(), String> {
        let mut conn = get_connection(app)?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        SpecimenRepository::delete_captured_records_by_session(&tx, id)
            .map_err(|e| e.to_string())?;

        SessionRepository::delete_session(&tx, id).map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub struct SpecimenService;

impl SpecimenService {
    pub fn get_captured_records(
        app: &AppHandle,
        session_id: i32,
    ) -> Result<Vec<CapturedRecord>, String> {
        let conn = get_connection(app)?;
        SpecimenRepository::get_captured_records(&conn, session_id).map_err(|e| e.to_string())
    }

    pub fn save_captured_record(
        app: &AppHandle,
        record: CapturedRecord,
    ) -> Result<serde_json::Value, String> {
        let mut conn = get_connection(app)?;
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        let user_id: Option<i32> = tx
            .query_row(
                "SELECT user_id FROM sessions WHERE id = ?1",
                params![record.session_id],
                |r| r.get(0),
            )
            .ok();
        let now = chrono::Local::now().to_rfc3339();

        // Seed/update agents table with new/existing collector/determiner names
        if let Some(ref recorded_by) = record.recorded_by {
            let new_recorded_by_agents = split_names(recorded_by);
            for name in new_recorded_by_agents {
                let _ = AgentRepository::add_or_update_agent(&tx, &name, user_id, Some(&now));
            }
        }
        if let Some(ref identified_by) = record.identified_by {
            let new_identified_by_agents = split_names(identified_by);
            for name in new_identified_by_agents {
                let _ = AgentRepository::add_or_update_agent(&tx, &name, user_id, Some(&now));
            }
        }

        let id =
            SpecimenRepository::save_captured_record(&tx, &record).map_err(|e| e.to_string())?;

        tx.commit().map_err(|e| e.to_string())?;

        Ok(json!({ "id": id, "success": true }))
    }

    pub fn delete_captured_record(app: &AppHandle, id: i32) -> Result<(), String> {
        let conn = get_connection(app)?;
        SpecimenRepository::delete_captured_record(&conn, id).map_err(|e| e.to_string())
    }
}

pub struct TaxonomyService;

impl TaxonomyService {
    pub fn search_reference(
        app: &AppHandle,
        filters: serde_json::Value,
    ) -> Result<Vec<ReferenceSpecimen>, String> {
        // let recorded_by = filters
        //     .get("recordedBy")
        //     .and_then(|v| v.as_str())
        //     .unwrap_or("")
        //     .trim();
        // let record_number = filters
        //     .get("recordNumber")
        //     .and_then(|v| v.as_str())
        //     .unwrap_or("")
        //     .trim();
        // let locality = filters
        //     .get("locality")
        //     .and_then(|v| v.as_str())
        //     .unwrap_or("")
        //     .trim();
        // let scientific_name = filters
        //     .get("scientificName")
        //     .and_then(|v| v.as_str())
        //     .unwrap_or("")
        //     .trim();
        // let family = filters
        //     .get("family")
        //     .and_then(|v| v.as_str())
        //     .unwrap_or("")
        //     .trim();
        // let country = filters
        //     .get("country")
        //     .and_then(|v| v.as_str())
        //     .unwrap_or("")
        //     .trim();
        // let state_province = filters
        //     .get("stateProvince")
        //     .and_then(|v| v.as_str())
        //     .unwrap_or("")
        //     .trim();

        // let year = filters.get("year").and_then(|v| v.as_i64());
        // let month = filters.get("month").and_then(|v| v.as_i64());
        // let day = filters.get("day").and_then(|v| v.as_i64());

        // let has_recorded_by = !recorded_by.is_empty();
        // let has_record_number = !record_number.is_empty();
        // let has_locality = !locality.is_empty();
        // let has_scientific_name = !scientific_name.is_empty();
        // let has_family = !family.is_empty();
        // let has_country = !country.is_empty();
        // let has_state_province = !state_province.is_empty();

        // let has_year = year.is_some();
        // let has_month = month.is_some();
        // let has_day = day.is_some();
        // let has_date = has_year || has_month || has_day;

        // let has_other =
        //     has_family || has_scientific_name || has_country || has_state_province || has_locality;

        // let mut non_date_fields_count = 0;
        // if has_recorded_by {
        //     non_date_fields_count += 1;
        // }
        // if has_record_number {
        //     non_date_fields_count += 1;
        // }
        // if has_family {
        //     non_date_fields_count += 1;
        // }
        // if has_scientific_name {
        //     non_date_fields_count += 1;
        // }
        // if has_country {
        //     non_date_fields_count += 1;
        // }
        // if has_state_province {
        //     non_date_fields_count += 1;
        // }
        // if has_locality {
        //     non_date_fields_count += 1;
        // }

        // let mut total_filled_count = non_date_fields_count;
        // if has_year {
        //     total_filled_count += 1;
        // }
        // if has_month {
        //     total_filled_count += 1;
        // }
        // if has_day {
        //     total_filled_count += 1;
        // }

        // if total_filled_count == 0 {
        //     return Err("Please enter at least one query search field.".to_string());
        // }

        // // Rule 1: collector requires at least a collector number, or if just collector and one of the date fields, then also one of the other fields
        // if has_recorded_by && !has_record_number && !(has_date && has_other) {
        //     return Err("Collector search requires a collector number, or if just a collector and a date field, it also requires at least one of (family, scientific name, country, Admin Div 1, or locality).".to_string());
        // }

        // // Rule 2: collector number always requires a collector
        // if has_record_number && !has_recorded_by {
        //     return Err(
        //         "Collector number always requires a collector name, regardless of other fields."
        //             .to_string(),
        //     );
        // }

        // // Rule 3: date searches require at least two other non-date fields
        // if has_date && non_date_fields_count < 2 {
        //     return Err(
        //         "Searches on year, month, or day require at least two other non-date fields."
        //             .to_string(),
        //     );
        // }

        // // Rule 4: family, scientific name, country, stateProvince, or locality requires at least two other fields (total of 3 or more fields)
        // if has_other && total_filled_count < 3 {
        //     return Err("Searching on family, scientific name, country, Admin Div 1, or locality requires at least two other fields (total of 3 or more fields).".to_string());
        // }

        let conn = get_connection(app)?;
        TaxonomyRepository::search_reference(&conn, &filters)
    }

    pub fn autocomplete_scientific_name(
        app: &AppHandle,
        query: &str,
    ) -> Result<Vec<TaxonAutocompleteResult>, String> {
        let conn = get_connection(app)?;
        TaxonomyRepository::autocomplete_scientific_name(&conn, query).map_err(|e| e.to_string())
    }

    pub fn lookup_taxon_by_name(app: &AppHandle, name: &str) -> Result<Option<String>, String> {
        let conn = get_connection(app)?;
        TaxonomyRepository::lookup_taxon_by_name(&conn, name).map_err(|e| e.to_string())
    }

    pub fn resolve_wcvp_families(
        app: &AppHandle,
        queries: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let conn = get_connection(app)?;
        let mut results = serde_json::Map::new();
        let mut cache: std::collections::HashMap<String, String> = std::collections::HashMap::new();

        for query in queries {
            let rec_id = query
                .get("id")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                .to_string();
            let taxon_id = query
                .get("taxonID")
                .and_then(|v| v.as_str())
                .map(|s| s.trim());
            let scientific_name = query
                .get("scientificName")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();

            let mut resolved_family = None;

            // 1. If taxonID is present
            if let Some(tid) = taxon_id {
                if !tid.is_empty() {
                    if let Some(cached) = cache.get(tid) {
                        resolved_family = Some(cached.clone());
                    } else {
                        if let Ok(Some(fam)) = TaxonomyRepository::find_family_recursive(&conn, tid)
                        {
                            cache.insert(tid.to_string(), fam.clone());
                            resolved_family = Some(fam);
                        }
                    }
                }
            }

            // 2. If taxonID is absent or recursive search failed
            if resolved_family.is_none() && !scientific_name.is_empty() {
                if let Some(first_word) = scientific_name.split_whitespace().next() {
                    let first_word_clean =
                        first_word.trim_matches(|c: char| c.is_ascii_punctuation());
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

                                let start_id: Option<String> =
                                    stmt.query_row(params![genus_pattern], |r| r.get(0)).ok();
                                if let Some(sid) = start_id {
                                    if let Ok(Some(fam)) =
                                        TaxonomyRepository::find_family_recursive(&conn, &sid)
                                    {
                                        cache.insert(first_word_clean.to_string(), fam.clone());
                                        resolved_family = Some(fam);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            results.insert(
                rec_id,
                serde_json::Value::String(resolved_family.unwrap_or_default()),
            );
        }

        Ok(serde_json::Value::Object(results))
    }
}

pub struct AgentService;

impl AgentService {
    pub fn autocomplete_recorded_by(app: &AppHandle, query: &str) -> Result<Vec<String>, String> {
        let conn = get_connection(app)?;
        AgentRepository::autocomplete_recorded_by(&conn, query).map_err(|e| e.to_string())
    }

    pub fn autocomplete_agent(app: &AppHandle, query: &str) -> Result<Vec<String>, String> {
        let conn = get_connection(app)?;
        AgentRepository::autocomplete_agent(&conn, query).map_err(|e| e.to_string())
    }

    pub fn check_agent_exists(app: &AppHandle, name: &str) -> Result<bool, String> {
        let conn = get_connection(app)?;
        AgentRepository::check_agent_exists(&conn, name).map_err(|e| e.to_string())
    }

    pub fn add_agent(app: &AppHandle, name: &str, created_by: Option<i32>) -> Result<(), String> {
        let conn = get_connection(app)?;
        let now = chrono::Local::now().to_rfc3339();
        AgentRepository::add_or_update_agent(&conn, name, created_by, Some(&now))
            .map_err(|e| e.to_string())
    }
}

pub struct GeographyService;

impl GeographyService {
    pub fn get_table_counts(app: &AppHandle) -> Result<serde_json::Value, String> {
        let conn = get_connection(app)?;
        GeographyRepository::get_table_counts(&conn).map_err(|e| e.to_string())
    }

    pub fn autocomplete_locality(
        app: &AppHandle,
        query: &str,
    ) -> Result<Vec<LocalitySearchResult>, String> {
        let conn = get_connection(app)?;
        GeographyRepository::autocomplete_locality(&conn, query)
    }

    pub fn autocomplete_geography(
        app: &AppHandle,
        field: &str,
        query: &str,
        country: &str,
        state_province: &str,
        county: &str,
    ) -> Result<Vec<String>, String> {
        let conn = get_connection(app)?;
        GeographyRepository::autocomplete_geography(
            &conn,
            field,
            query,
            country,
            state_province,
            county,
        )
        .map_err(|e| e.to_string())
    }
}

pub struct ExportService;

impl ExportService {
    #[allow(clippy::too_many_arguments)]
    pub fn save_export_settings(
        app: &AppHandle,
        user_id: i32,
        format: &str,
        collection_code: &str,
        include_grid_reference: bool,
        include_islands: bool,
        backup_location: &str,
        home_country: &str,
        initials_require_periods: bool,
        preferred_date_format: &str,
    ) -> Result<(), String> {
        let conn = get_connection(app)?;
        ExportRepository::save_export_settings(
            &conn,
            user_id,
            format,
            collection_code,
            include_grid_reference,
            include_islands,
            backup_location,
            home_country,
            initials_require_periods,
            preferred_date_format,
        )
        .map_err(|e| e.to_string())
    }

    pub fn get_export_settings(app: &AppHandle, user_id: i32) -> Result<ExportSettingsDto, String> {
        let conn = get_connection(app)?;
        match ExportRepository::get_export_settings(&conn, user_id) {
            Ok(Some(settings)) => Ok(settings),
            Ok(None) => Ok(ExportSettingsDto {
                format: "DwC".to_string(),
                collection_code: "RHOIO".to_string(),
                include_grid_reference: false,
                include_islands: false,
                backup_location: "".to_string(),
                home_country: "".to_string(),
                initials_require_periods: true,
                preferred_date_format: "en-US".to_string(),
            }),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn export_session_csv(
        app: &AppHandle,
        session_id: i32,
        filepath: &str,
        csv_content: &str,
    ) -> Result<String, String> {
        fs::write(filepath, csv_content).map_err(|e| e.to_string())?;

        let conn = get_connection(app)?;
        SessionRepository::update_last_exported(&conn, session_id).map_err(|e| e.to_string())?;

        Ok(format!("Successfully exported records to {}", filepath))
    }
}

pub struct ReferenceService;

impl ReferenceService {
    pub fn get_metadata(app: &AppHandle) -> Result<serde_json::Value, String> {
        let conn = get_connection(app)?;
        ReferenceRepository::get_metadata(&conn)
    }

    pub fn import_reference_dataset(
        app: &AppHandle,
        filepath: &str,
        append: bool,
    ) -> Result<(), String> {
        let mut conn = get_connection(app)?;
        ReferenceRepository::import_csv(Some(app), &mut conn, filepath, append)?;
        let _ = app.emit(
            "import-progress",
            "Rebuilding search index and collector catalog...",
        );
        crate::db::finalize_reference_import(&mut conn)?;
        Ok(())
    }

    pub fn get_wcvp_metadata(app: &AppHandle) -> Result<serde_json::Value, String> {
        let conn = get_connection(app)?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM wcvp_taxonomy", [], |r| r.get(0))
            .unwrap_or(0);
        let version = crate::db::get_wcvp_version(&conn)?;
        Ok(serde_json::json!({
            "recordCount": count,
            "version": version
        }))
    }

    pub fn import_wcvp_dataset(
        app: &AppHandle,
        filepath: &str,
        version: i32,
    ) -> Result<(), String> {
        let mut conn = get_connection(app)?;
        ReferenceRepository::import_wcvp_csv(Some(app), &mut conn, filepath, version)?;
        Ok(())
    }
}
