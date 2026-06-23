use crate::models::{
    CapturedRecord, ExportSettingsDto, ReferenceSpecimen, SessionDto, TaxonAutocompleteResult,
    UserDto,
};
use crate::parsers::{
    extract_digits, normalize_locality, normalize_search_recorded_by, normalize_taxon_name,
};
use rusqlite::{params, Connection, Error};
use serde_json::json;

pub struct UserRepository;

impl UserRepository {
    pub fn insert_user(
        conn: &Connection,
        username: &str,
        password_hash: &str,
    ) -> Result<(), Error> {
        conn.execute(
            "INSERT INTO users (username, password_hash) VALUES (?1, ?2)",
            params![username, password_hash],
        )?;
        Ok(())
    }

    pub fn get_user_by_credentials(
        conn: &Connection,
        username: &str,
        password_hash: &str,
    ) -> Result<Option<UserDto>, Error> {
        let mut stmt = conn
            .prepare("SELECT id, username FROM users WHERE username = ?1 AND password_hash = ?2")?;
        let mut rows = stmt.query_map(params![username, password_hash], |row| {
            Ok(UserDto {
                id: row.get(0)?,
                username: row.get(1)?,
            })
        })?;

        if let Some(row) = rows.next() {
            let user = row?;
            Ok(Some(user))
        } else {
            Ok(None)
        }
    }
}

pub struct SessionRepository;

impl SessionRepository {
    pub fn create_session(conn: &Connection, user_id: i32, name: &str) -> Result<i32, Error> {
        conn.execute(
            "INSERT INTO sessions (user_id, name) VALUES (?1, ?2)",
            params![user_id, name],
        )?;
        let id = conn.last_insert_rowid() as i32;
        Ok(id)
    }

    pub fn get_sessions(conn: &Connection, user_id: i32) -> Result<Vec<SessionDto>, Error> {
        let mut stmt = conn.prepare(
            "SELECT s.id, s.name, COUNT(r.id) as count, MAX(r.modified_at) as last_record, s.last_exported_at 
             FROM sessions s 
             LEFT JOIN captured_records r ON s.id = r.session_id 
             WHERE s.user_id = ?1 
             GROUP BY s.id 
             ORDER BY s.id DESC"
        )?;
        let rows = stmt.query_map(params![user_id], |row| {
            Ok(SessionDto {
                id: row.get(0)?,
                name: row.get(1)?,
                record_count: row.get(2)?,
                last_record_at: row.get(3)?,
                last_exported_at: row.get(4)?,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn rename_session(conn: &Connection, id: i32, name: &str) -> Result<(), Error> {
        conn.execute(
            "UPDATE sessions SET name = ?1 WHERE id = ?2",
            params![name, id],
        )?;
        Ok(())
    }

    pub fn delete_session(conn: &Connection, id: i32) -> Result<(), Error> {
        conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn update_last_exported(conn: &Connection, id: i32) -> Result<(), Error> {
        conn.execute(
            "UPDATE sessions SET last_exported_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }
}

pub struct SpecimenRepository;

impl SpecimenRepository {
    pub fn get_captured_records(
        conn: &Connection,
        session_id: i32,
    ) -> Result<Vec<CapturedRecord>, Error> {
        let mut stmt = conn.prepare(
            "SELECT id, collectionCode, catalogNumber, duplicates, recordNumber, recordedBy, 
                    verbatimEventDate, year, month, day, country, stateProvince, county, municipality, 
                    locality, locationRemarks, verbatimCoordinates, decimalLatitude, decimalLongitude, verbatimElevation, habitat, occurrenceRemarks, fieldNotes,
                    typeStatus, identificationQualifier, scientificName, identifiedBy, yearIdentified, 
                    monthIdentified, dayIdentified, identificationRemarks, taxonID, cultivated 
             FROM captured_records 
             WHERE session_id = ?1 
             ORDER BY id DESC"
        )?;

        let rows = stmt.query_map(params![session_id], |row| {
            let cultivated: Option<i32> = row.get(32)?;
            Ok(CapturedRecord {
                id: Some(row.get(0)?),
                session_id,
                collection_code: row.get(1)?,
                catalog_number: row.get(2)?,
                duplicates: row.get(3)?,
                record_number: row.get(4)?,
                recorded_by: row.get(5)?,
                verbatim_event_date: row.get(6)?,
                year: row.get(7)?,
                month: row.get(8)?,
                day: row.get(9)?,
                country: row.get(10)?,
                state_province: row.get(11)?,
                county: row.get(12)?,
                municipality: row.get(13)?,
                locality: row.get(14)?,
                location_remarks: row.get(15)?,
                verbatim_coordinates: row.get(16)?,
                decimal_latitude: row.get(17)?,
                decimal_longitude: row.get(18)?,
                verbatim_elevation: row.get(19)?,
                habitat: row.get(20)?,
                occurrence_remarks: row.get(21)?,
                field_notes: row.get(22)?,
                type_status: row.get(23)?,
                identification_qualifier: row.get(24)?,
                scientific_name: row.get(25)?,
                identified_by: row.get(26)?,
                year_identified: row.get(27)?,
                month_identified: row.get(28)?,
                day_identified: row.get(29)?,
                identification_remarks: row.get(30)?,
                taxon_id: row.get(31)?,
                cultivated: cultivated.unwrap_or(0) == 1,
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn save_captured_record(conn: &Connection, record: &CapturedRecord) -> Result<i32, Error> {
        let cultivated_int = if record.cultivated { 1 } else { 0 };

        if let Some(existing_id) = record.id {
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
                    record.collection_code, record.catalog_number, record.duplicates, record.record_number, record.recorded_by,
                    record.verbatim_event_date, record.year, record.month, record.day, record.country,
                    record.state_province, record.county, record.municipality, record.locality, record.location_remarks,
                    record.verbatim_coordinates, record.decimal_latitude, record.decimal_longitude, record.verbatim_elevation, record.habitat,
                    record.occurrence_remarks, record.field_notes, record.type_status, record.identification_qualifier, record.scientific_name,
                    record.identified_by, record.year_identified, record.month_identified, record.day_identified, record.identification_remarks,
                    record.taxon_id, cultivated_int, existing_id, record.session_id
                ]
            )?;
            Ok(existing_id)
        } else {
            conn.execute(
                "INSERT INTO captured_records (
                    session_id, collectionCode, catalogNumber, duplicates, recordNumber, recordedBy,
                    verbatimEventDate, year, month, day, country, stateProvince, county, municipality,
                    locality, locationRemarks, verbatimCoordinates, decimalLatitude, decimalLongitude, verbatimElevation, habitat, occurrenceRemarks, fieldNotes,
                    typeStatus, identificationQualifier, scientificName, identifiedBy, yearIdentified,
                    monthIdentified, dayIdentified, identificationRemarks, taxonID, cultivated
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33)",
                params![
                    record.session_id, record.collection_code, record.catalog_number, record.duplicates, record.record_number, record.recorded_by,
                    record.verbatim_event_date, record.year, record.month, record.day, record.country, record.state_province, record.county, record.municipality,
                    record.locality, record.location_remarks, record.verbatim_coordinates, record.decimal_latitude, record.decimal_longitude, record.verbatim_elevation, record.habitat, record.occurrence_remarks, record.field_notes,
                    record.type_status, record.identification_qualifier, record.scientific_name, record.identified_by, record.year_identified,
                    record.month_identified, record.day_identified, record.identification_remarks, record.taxon_id, cultivated_int
                ]
            )?;
            let new_id = conn.last_insert_rowid() as i32;
            Ok(new_id)
        }
    }

    pub fn delete_captured_record(conn: &Connection, id: i32) -> Result<(), Error> {
        conn.execute("DELETE FROM captured_records WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn delete_captured_records_by_session(
        conn: &Connection,
        session_id: i32,
    ) -> Result<(), Error> {
        conn.execute(
            "DELETE FROM captured_records WHERE session_id = ?1",
            params![session_id],
        )?;
        Ok(())
    }
}

pub struct TaxonomyRepository;

impl TaxonomyRepository {
    pub fn search_reference(
        conn: &Connection,
        filters: &serde_json::Value,
    ) -> Result<Vec<ReferenceSpecimen>, String> {
        let recorded_by = filters
            .get("recordedBy")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let record_number = filters
            .get("recordNumber")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let locality = filters
            .get("locality")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let scientific_name = filters
            .get("scientificName")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let family = filters
            .get("family")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let country = filters
            .get("country")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let state_province = filters
            .get("stateProvince")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();

        let year = filters.get("year").and_then(|v| v.as_i64());
        let month = filters.get("month").and_then(|v| v.as_i64());
        let day = filters.get("day").and_then(|v| v.as_i64());

        let has_recorded_by = !recorded_by.is_empty();
        let has_record_number = !record_number.is_empty();
        let has_locality = !locality.is_empty();
        let has_scientific_name = !scientific_name.is_empty();
        let has_family = !family.is_empty();
        let has_country = !country.is_empty();
        let has_state_province = !state_province.is_empty();

        let mut sql = String::from(
            "SELECT recordedBy, recordNumber, locality, locationRemarks, verbatimLocality, 
                    scientificName, family, country, stateProvince, year, month, day,
                    identificationQualifier, collectionCode, decimalLatitude, decimalLongitude,
                    verbatimCoordinates, verbatimElevation, elevation, habitat, occurrenceRemarks,
                    fieldNotes, fieldNumber
             FROM gbif WHERE 1=1",
        );
        let mut params_vec: Vec<serde_json::Value> = Vec::new();

        if has_recorded_by {
            let normalized = normalize_search_recorded_by(recorded_by);
            sql.push_str(" AND searchRecordedBy LIKE ? COLLATE NOCASE");
            params_vec.push(json!(format!("{}%", normalized)));
        }
        if has_record_number {
            let digits = extract_digits(record_number);
            if !digits.is_empty() {
                let terms: Vec<&str> = digits.split_whitespace().collect();

                let mut fts_clauses = Vec::new();
                for term in &terms {
                    fts_clauses.push(format!("cleanedFieldNumber:{}", term));
                }
                let fts_query = fts_clauses.join(" AND ");

                let mut fn_clauses = Vec::new();
                for _ in &terms {
                    fn_clauses.push("fieldNotes LIKE ?");
                }
                let fn_clause_str = fn_clauses.join(" AND ");

                sql.push_str(" AND (recordNumber = ? OR gbifID IN (SELECT rowid FROM gbif_fts WHERE gbif_fts MATCH ?)");
                if !fn_clause_str.is_empty() {
                    sql.push_str(" OR (");
                    sql.push_str(&fn_clause_str);
                    sql.push_str(")");
                }
                sql.push_str(")");

                params_vec.push(json!(record_number));
                params_vec.push(json!(fts_query));
                for term in &terms {
                    params_vec.push(json!(format!("%{}%", term)));
                }
            } else {
                sql.push_str(" AND recordNumber = ?");
                params_vec.push(json!(record_number));
            }
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
                    sql.push_str(
                        " AND gbifID IN (SELECT rowid FROM gbif_fts WHERE gbif_fts MATCH ?)",
                    );
                    params_vec.push(json!(fts_query));
                }
            }
        }

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

        let rusql_params: Vec<Box<dyn rusqlite::ToSql>> = params_vec
            .iter()
            .map(|v| {
                if let Some(s) = v.as_str() {
                    Box::new(s.to_string()) as Box<dyn rusqlite::ToSql>
                } else if let Some(i) = v.as_i64() {
                    Box::new(i) as Box<dyn rusqlite::ToSql>
                } else {
                    Box::new("") as Box<dyn rusqlite::ToSql>
                }
            })
            .collect();

        let ref_params: Vec<&dyn rusqlite::ToSql> =
            rusql_params.iter().map(|b| b.as_ref()).collect();

        let rows = stmt
            .query_map(&ref_params[..], |row| {
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
                let field_number: Option<String> = row.get(22)?;

                Ok(ReferenceSpecimen {
                    id: None,
                    recorded_by: recorded_by.unwrap_or_default(),
                    record_number: record_number.unwrap_or_default(),
                    locality: locality.unwrap_or_default(),
                    location_notes: location_notes.unwrap_or_default(),
                    verbatim_locality: verbatim_locality.unwrap_or_default(),
                    scientific_name: scientific_name.unwrap_or_default(),
                    family: family.unwrap_or_default(),
                    genus: "".to_string(),
                    specific_epithet: "".to_string(),
                    infra_specific_epithet: "".to_string(),
                    country: country.unwrap_or_default(),
                    state_province: state_province.unwrap_or_default(),
                    year,
                    month,
                    day,
                    identification_qualifier: id_qualifier.unwrap_or_default(),
                    collection_code: collection_code.unwrap_or_default(),
                    decimal_latitude,
                    decimal_longitude,
                    verbatim_coordinates: verbatim_coordinates.unwrap_or_default(),
                    verbatim_elevation: verbatim_elevation.unwrap_or_default(),
                    elevation: elevation.unwrap_or_default(),
                    habitat: habitat.unwrap_or_default(),
                    occurrence_remarks: occurrence_remarks.unwrap_or_default(),
                    field_notes: field_notes.unwrap_or_default(),
                    field_number: field_number.unwrap_or_default(),
                })
            })
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn autocomplete_scientific_name(
        conn: &Connection,
        query: &str,
    ) -> Result<Vec<TaxonAutocompleteResult>, Error> {
        let terms: Vec<&str> = query.split_whitespace().collect();
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

        let mut stmt = conn.prepare(
            "SELECT plant_name_id, taxon_name, family, genus, species, taxon_authors, taxon_rank 
             FROM wcvp_taxonomy 
             WHERE rowid IN (SELECT rowid FROM wcvp_taxonomy_fts WHERE wcvp_taxonomy_fts MATCH ?1) 
             LIMIT 15",
        )?;

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

            Ok(TaxonAutocompleteResult {
                taxon_id: id,
                scientific_name: full_name,
                family: family.unwrap_or_default(),
                genus: genus.unwrap_or_default(),
                specific_epithet: species.unwrap_or_default(),
                authors: authors.unwrap_or_default(),
                rank: rank.unwrap_or_default(),
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn find_family_recursive(
        conn: &Connection,
        start_id: &str,
    ) -> Result<Option<String>, Error> {
        let mut stmt = conn.prepare("SELECT family FROM wcvp_taxonomy WHERE plant_name_id = ?1")?;
        if let Ok(Some(fam)) = stmt.query_row(params![start_id], |r| r.get::<_, Option<String>>(0))
        {
            if !fam.trim().is_empty() {
                return Ok(Some(fam));
            }
        }

        let mut current_id = start_id.to_string();
        let mut depth = 0;

        while depth < 30 {
            let mut stmt = conn.prepare(
                "SELECT plant_name_id, parent_plant_name_id, taxon_rank, taxon_name, family 
                 FROM wcvp_taxonomy 
                 WHERE plant_name_id = ?1",
            )?;

            let row = stmt.query_row(params![current_id], |r| {
                let pid: Option<String> = r.get(0)?;
                let parent_id: Option<String> = r.get(1)?;
                let rank: Option<String> = r.get(2)?;
                let name: Option<String> = r.get(3)?;
                let family: Option<String> = r.get(4)?;
                Ok((pid, parent_id, rank, name, family))
            });

            match row {
                Ok((_, parent_id, rank, name, family)) => {
                    if let Some(r) = rank {
                        if r.eq_ignore_ascii_case("Family") {
                            return Ok(name);
                        }
                    }
                    if let Some(fam) = family {
                        if !fam.trim().is_empty() {
                            return Ok(Some(fam));
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
}

pub struct AgentRepository;

impl AgentRepository {
    pub fn autocomplete_recorded_by(conn: &Connection, query: &str) -> Result<Vec<String>, Error> {
        let normalized = normalize_search_recorded_by(query);
        let mut stmt = conn.prepare(
            "SELECT DISTINCT collector FROM (
                SELECT recordedBy AS collector FROM gbif WHERE searchRecordedBy LIKE ?1 COLLATE NOCASE
                UNION
                SELECT recordedBy AS collector FROM captured_records WHERE recordedBy LIKE ?2 COLLATE NOCASE
             ) WHERE collector IS NOT NULL AND collector != '' LIMIT 10"
        )?;

        let rows = stmt.query_map(
            params![format!("{}%", normalized), format!("{}%", query)],
            |row| row.get(0),
        )?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn autocomplete_agent(conn: &Connection, query: &str) -> Result<Vec<String>, Error> {
        let normalized = normalize_search_recorded_by(query);
        let mut stmt =
            conn.prepare("SELECT agentName FROM agents WHERE searchAgentName LIKE ?1 LIMIT 10")?;
        let rows = stmt.query_map(params![format!("{}%", normalized)], |row| row.get(0))?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn check_agent_exists(conn: &Connection, name: &str) -> Result<bool, Error> {
        let normalized = normalize_search_recorded_by(name);
        let mut stmt = conn.prepare("SELECT 1 FROM agents WHERE searchAgentName = ?1")?;
        let exists = stmt.exists(params![normalized])?;
        Ok(exists)
    }

    pub fn add_agent(conn: &Connection, name: &str) -> Result<(), Error> {
        let search_name = normalize_search_recorded_by(name);
        conn.execute(
            "INSERT OR IGNORE INTO agents (agentName, searchAgentName) VALUES (?1, ?2)",
            params![name, search_name],
        )?;
        Ok(())
    }
}

pub struct ExportRepository;

impl ExportRepository {
    pub fn save_export_settings(
        conn: &Connection,
        user_id: i32,
        format: &str,
        mappings: &str,
    ) -> Result<(), Error> {
        conn.execute(
            "INSERT INTO export_settings (user_id, format, mappings) 
             VALUES (?1, ?2, ?3) 
             ON CONFLICT(user_id) DO UPDATE SET format=?2, mappings=?3",
            params![user_id, format, mappings],
        )?;
        Ok(())
    }

    pub fn get_export_settings(
        conn: &Connection,
        user_id: i32,
    ) -> Result<Option<ExportSettingsDto>, Error> {
        let mut stmt =
            conn.prepare("SELECT format, mappings FROM export_settings WHERE user_id = ?1")?;
        let mut rows = stmt.query_map(params![user_id], |row| {
            Ok(ExportSettingsDto {
                format: row.get(0)?,
                mappings: row.get(1)?,
            })
        })?;

        if let Some(row) = rows.next() {
            let settings = row?;
            Ok(Some(settings))
        } else {
            Ok(None)
        }
    }
}

pub struct GeographyRepository;

impl GeographyRepository {
    pub fn get_table_counts(conn: &Connection) -> Result<serde_json::Value, Error> {
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

    pub fn autocomplete_locality(conn: &Connection, query: &str) -> Result<Vec<String>, String> {
        let normalized = normalize_locality(query);
        let terms: Vec<&str> = normalized.split_whitespace().collect();

        let mut sql = String::from("SELECT MIN(TRIM(locality)) AS uniq_locality FROM (\n");
        let mut params_vec: Vec<String> = Vec::new();

        if !terms.is_empty() {
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

            sql.push_str("    SELECT locality FROM captured_records WHERE 1=1");
            for (i, term) in terms.iter().enumerate() {
                sql.push_str(&format!(" AND locality LIKE ?{}", i + 2));
                params_vec.push(format!("%{}%", term));
            }
            sql.push_str("\n");
        } else {
            sql.push_str("    SELECT locality FROM gbif WHERE locality LIKE ?1\n");
            sql.push_str("    UNION ALL\n");
            sql.push_str("    SELECT locality FROM captured_records WHERE locality LIKE ?1\n");
            params_vec.push(format!("%{}%", query));
        }

        sql.push_str(") WHERE locality IS NOT NULL AND TRIM(locality) != ''\n");
        sql.push_str("GROUP BY LOWER(TRIM(locality))\n");
        sql.push_str("LIMIT 10");

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        let rusql_params: Vec<Box<dyn rusqlite::ToSql>> = params_vec
            .iter()
            .map(|s| Box::new(s.to_string()) as Box<dyn rusqlite::ToSql>)
            .collect();

        let ref_params: Vec<&dyn rusqlite::ToSql> =
            rusql_params.iter().map(|b| b.as_ref()).collect();

        let rows = stmt
            .query_map(&ref_params[..], |row| {
                let val: String = row.get(0)?;
                Ok(val)
            })
            .map_err(|e| e.to_string())?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(|e| e.to_string())?);
        }
        Ok(list)
    }

    pub fn autocomplete_geography(
        conn: &Connection,
        field: &str,
        query: &str,
        country: &str,
        state_province: &str,
        county: &str,
    ) -> Result<Vec<String>, Error> {
        let mut sql = String::new();
        let mut params_vec: Vec<String> = vec![format!("{}%", query)];

        match field {
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
                params_vec.push(country.to_string());
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
                params_vec.push(country.to_string());
                params_vec.push(state_province.to_string());
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
                params_vec.push(country.to_string());
                params_vec.push(state_province.to_string());
                params_vec.push(county.to_string());
            }
            _ => return Err(Error::QueryReturnedNoRows),
        }

        let mut stmt = conn.prepare(&sql)?;

        let rusql_params: Vec<Box<dyn rusqlite::ToSql>> = params_vec
            .iter()
            .map(|v| Box::new(v.clone()) as Box<dyn rusqlite::ToSql>)
            .collect();
        let ref_params: Vec<&dyn rusqlite::ToSql> =
            rusql_params.iter().map(|b| b.as_ref()).collect();

        let rows = stmt.query_map(&ref_params[..], |row| {
            let val: Option<String> = row.get(0)?;
            Ok(val.unwrap_or_default())
        })?;

        let mut list = Vec::new();
        for r in rows {
            let s = r?;
            if !s.is_empty() {
                list.push(s);
            }
        }

        Ok(list)
    }
}

pub struct ReferenceRepository;

impl ReferenceRepository {
    pub fn get_metadata(conn: &Connection) -> Result<serde_json::Value, String> {
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM gbif", [], |r| r.get(0))
            .unwrap_or(0);

        let mut stmt = conn.prepare("SELECT DISTINCT country FROM gbif WHERE country IS NOT NULL AND country != '' ORDER BY country")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut countries = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let country: String = row.get(0).map_err(|e| e.to_string())?;
            countries.push(country);
        }

        let mut stmt = conn.prepare("SELECT DISTINCT collectionCode FROM gbif WHERE collectionCode IS NOT NULL AND collectionCode != '' ORDER BY collectionCode")
            .map_err(|e| e.to_string())?;
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut collection_codes = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let code: String = row.get(0).map_err(|e| e.to_string())?;
            collection_codes.push(code);
        }

        Ok(json!({
            "recordCount": count,
            "countries": countries,
            "collectionCodes": collection_codes
        }))
    }

    pub fn import_csv(conn: &mut Connection, filepath: &str) -> Result<(), String> {
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_path(filepath)
            .map_err(|e| format!("Failed to open CSV file: {}", e))?;

        let headers = rdr
            .headers()
            .map_err(|e| format!("Failed to read CSV headers: {}", e))?;
        let mut header_map = std::collections::HashMap::new();
        for (i, h) in headers.iter().enumerate() {
            header_map.insert(h.to_lowercase(), i);
        }

        let get_idx =
            |name: &str| -> Option<usize> { header_map.get(&name.to_lowercase()).copied() };

        // Fallbacks mapping
        let mut col_indices = std::collections::HashMap::new();
        let target_fields = vec![
            "gbifID",
            "collectionCode",
            "catalogNumber",
            "recordNumber",
            "recordedBy",
            "year",
            "month",
            "day",
            "verbatimEventDate",
            "country",
            "stateProvince",
            "county",
            "municipality",
            "locality",
            "verbatimLocality",
            "locationRemarks",
            "verbatimCoordinates",
            "decimalLatitude",
            "decimalLongitude",
            "habitat",
            "verbatimElevation",
            "elevation",
            "occurrenceRemarks",
            "fieldNotes",
            "typeStatus",
            "identificationQualifier",
            "family",
            "scientificName",
            "identifiedBy",
            "yearIdentified",
            "monthIdentified",
            "dayIdentified",
            "identificationRemarks",
            "fieldNumber",
        ];

        for field in &target_fields {
            let mut idx = get_idx(field);
            if idx.is_none() {
                if *field == "gbifID" {
                    idx = get_idx("id");
                } else if *field == "locationRemarks" {
                    idx = get_idx("locationnotes");
                }
            }
            if let Some(i) = idx {
                col_indices.insert(*field, i);
            }
        }

        let get_val = |record: &csv::StringRecord, name: &str| -> Option<String> {
            col_indices
                .get(name)
                .and_then(|&i| record.get(i))
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        let tx = conn.transaction().map_err(|e| e.to_string())?;

        // 1. Delete all existing records
        tx.execute("DELETE FROM gbif", [])
            .map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM agents", [])
            .map_err(|e| e.to_string())?;

        // 2. Drop triggers temporarily for import performance
        let _ = tx.execute("DROP TRIGGER IF EXISTS gbif_ai", []);
        let _ = tx.execute("DROP TRIGGER IF EXISTS gbif_ad", []);
        let _ = tx.execute("DROP TRIGGER IF EXISTS gbif_au", []);
        let _ = tx.execute("DROP TRIGGER IF EXISTS gbif_cfn_insert", []);
        let _ = tx.execute("DROP TRIGGER IF EXISTS gbif_cfn_update", []);

        // 3. Perform inserts
        {
            let mut stmt = tx.prepare_cached(
                "INSERT INTO gbif (
                    gbifID, collectionCode, catalogNumber, recordNumber, recordedBy,
                    year, month, day, verbatimEventDate, country,
                    stateProvince, county, municipality, locality, verbatimLocality,
                    locationRemarks, verbatimCoordinates, decimalLatitude, decimalLongitude, habitat,
                    verbatimElevation, elevation, occurrenceRemarks, fieldNotes, typeStatus,
                    identificationQualifier, family, scientificName, identifiedBy, yearIdentified,
                    monthIdentified, dayIdentified, identificationRemarks, fieldNumber,
                    searchRecordedBy, normalizedRecordedBy, normalized_scientific_name, normalized_locality, cleanedFieldNumber
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5,
                    ?6, ?7, ?8, ?9, ?10,
                    ?11, ?12, ?13, ?14, ?15,
                    ?16, ?17, ?18, ?19, ?20,
                    ?21, ?22, ?23, ?24, ?25,
                    ?26, ?27, ?28, ?29, ?30,
                    ?31, ?32, ?33, ?34, ?35,
                    ?36, ?37, ?38, ?39
                )"
            ).map_err(|e| e.to_string())?;

            for result in rdr.records() {
                let record = result.map_err(|e| format!("CSV parse error: {}", e))?;

                let gbif_id: Option<i64> = get_val(&record, "gbifID").and_then(|s| s.parse().ok());
                let collection_code = get_val(&record, "collectionCode");
                let catalog_number = get_val(&record, "catalogNumber");
                let record_number = get_val(&record, "recordNumber");
                let recorded_by = get_val(&record, "recordedBy");

                let year: Option<i32> = get_val(&record, "year").and_then(|s| s.parse().ok());
                let month: Option<i32> = get_val(&record, "month").and_then(|s| s.parse().ok());
                let day: Option<i32> = get_val(&record, "day").and_then(|s| s.parse().ok());

                let verbatim_event_date = get_val(&record, "verbatimEventDate");
                let country = get_val(&record, "country");
                let state_province = get_val(&record, "stateProvince");
                let county = get_val(&record, "county");
                let municipality = get_val(&record, "municipality");
                let locality = get_val(&record, "locality");
                let verbatim_locality = get_val(&record, "verbatimLocality");
                let location_remarks = get_val(&record, "locationRemarks");
                let verbatim_coordinates = get_val(&record, "verbatimCoordinates");

                let decimal_latitude: Option<f64> =
                    get_val(&record, "decimalLatitude").and_then(|s| s.parse().ok());
                let decimal_longitude: Option<f64> =
                    get_val(&record, "decimalLongitude").and_then(|s| s.parse().ok());

                let habitat = get_val(&record, "habitat");
                let verbatim_elevation = get_val(&record, "verbatimElevation");
                let elevation = get_val(&record, "elevation");
                let occurrence_remarks = get_val(&record, "occurrenceRemarks");
                let field_notes = get_val(&record, "fieldNotes");
                let type_status = get_val(&record, "typeStatus");
                let identification_qualifier = get_val(&record, "identificationQualifier");
                let family = get_val(&record, "family");
                let scientific_name = get_val(&record, "scientificName");
                let identified_by = get_val(&record, "identifiedBy");

                let year_identified: Option<i32> =
                    get_val(&record, "yearIdentified").and_then(|s| s.parse().ok());
                let month_identified: Option<i32> =
                    get_val(&record, "monthIdentified").and_then(|s| s.parse().ok());
                let day_identified: Option<i32> =
                    get_val(&record, "dayIdentified").and_then(|s| s.parse().ok());

                let identification_remarks = get_val(&record, "identificationRemarks");
                let field_number = get_val(&record, "fieldNumber");

                // Normalizations
                let search_recorded_by = recorded_by
                    .as_ref()
                    .map(|s| normalize_search_recorded_by(s));
                let normalized_recorded_by = search_recorded_by.clone();

                let normalized_scientific_name =
                    scientific_name.as_ref().map(|s| normalize_taxon_name(s));

                let combined_locality = format!(
                    "{} {} {}",
                    locality.as_deref().unwrap_or(""),
                    location_remarks.as_deref().unwrap_or(""),
                    verbatim_locality.as_deref().unwrap_or("")
                );
                let normalized_locality = if combined_locality.trim().is_empty() {
                    None
                } else {
                    let norm = normalize_locality(&combined_locality);
                    if norm.trim().is_empty() {
                        Some("-".to_string())
                    } else {
                        Some(norm)
                    }
                };

                let cleaned_field_number = field_number.as_ref().map(|s| extract_digits(s));

                stmt.execute(params![
                    gbif_id,
                    collection_code,
                    catalog_number,
                    record_number,
                    recorded_by,
                    year,
                    month,
                    day,
                    verbatim_event_date,
                    country,
                    state_province,
                    county,
                    municipality,
                    locality,
                    verbatim_locality,
                    location_remarks,
                    verbatim_coordinates,
                    decimal_latitude,
                    decimal_longitude,
                    habitat,
                    verbatim_elevation,
                    elevation,
                    occurrence_remarks,
                    field_notes,
                    type_status,
                    identification_qualifier,
                    family,
                    scientific_name,
                    identified_by,
                    year_identified,
                    month_identified,
                    day_identified,
                    identification_remarks,
                    field_number,
                    search_recorded_by,
                    normalized_recorded_by,
                    normalized_scientific_name,
                    normalized_locality,
                    cleaned_field_number
                ])
                .map_err(|e| format!("Failed to insert record: {}", e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;
        Ok(())
    }
}
