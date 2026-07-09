use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: i32,
    pub username: String,
    pub given_name: String,
    pub family_name: String,
    pub initials: String,
    pub is_admin: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub id: i32,
    pub name: String,
    pub record_count: i64,
    pub last_record_at: Option<String>,
    pub last_exported_at: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CapturedRecord {
    pub id: Option<i32>,
    pub session_id: i32,
    pub collection_code: Option<String>,
    pub catalog_number: Option<String>,
    pub duplicates: Option<String>,
    pub record_number: Option<String>,
    pub recorded_by: Option<String>,
    pub verbatim_event_date: Option<String>,
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub day: Option<i32>,
    pub country: Option<String>,
    pub state_province: Option<String>,
    pub county: Option<String>,
    pub municipality: Option<String>,
    pub island_group: Option<String>,
    pub island: Option<String>,
    pub locality: Option<String>,
    #[serde(rename = "locationNotes")]
    pub location_remarks: Option<String>,
    pub verbatim_coordinates: Option<String>,
    pub decimal_latitude: Option<f64>,
    pub decimal_longitude: Option<f64>,
    pub verbatim_elevation: Option<String>,
    pub habitat: Option<String>,
    pub occurrence_remarks: Option<String>,
    pub field_notes: Option<String>,
    pub type_status: Option<String>,
    pub identification_qualifier: Option<String>,
    pub scientific_name: Option<String>,
    pub identified_by: Option<String>,
    pub year_identified: Option<i32>,
    pub month_identified: Option<i32>,
    pub day_identified: Option<i32>,
    pub identification_remarks: Option<String>,
    #[serde(rename = "taxonID")]
    pub taxon_id: Option<String>,
    pub cultivated: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettingsDto {
    pub format: String,
    pub collection_code: String,
    pub include_grid_reference: bool,
    pub include_islands: bool,
    pub backup_location: String,
    pub home_country: String,
    pub initials_require_periods: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TaxonAutocompleteResult {
    #[serde(rename = "taxonID")]
    pub taxon_id: String,
    pub scientific_name: String,
    pub family: String,
    pub genus: String,
    pub specific_epithet: String,
    pub authors: String,
    pub rank: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LocalitySearchResult {
    pub locality: String,
    pub country: Option<String>,
    pub state_province: Option<String>,
    pub county: Option<String>,
    #[serde(rename = "locationNotes")]
    pub location_remarks: Option<String>,
    pub verbatim_coordinates: Option<String>,
    pub decimal_latitude: Option<f64>,
    pub decimal_longitude: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceSpecimen {
    pub id: Option<i32>,
    pub recorded_by: String,
    pub record_number: String,
    pub locality: String,
    pub location_notes: String,
    pub verbatim_locality: String,
    pub scientific_name: String,
    pub family: String,
    pub genus: String,
    pub specific_epithet: String,
    pub infra_specific_epithet: String,
    pub country: String,
    pub state_province: String,
    pub island_group: String,
    pub island: String,
    pub year: Option<i32>,
    pub month: Option<i32>,
    pub day: Option<i32>,
    pub identification_qualifier: String,
    pub collection_code: String,
    pub decimal_latitude: Option<f64>,
    pub decimal_longitude: Option<f64>,
    pub verbatim_coordinates: String,
    pub verbatim_elevation: String,
    pub elevation: String,
    pub habitat: String,
    pub occurrence_remarks: String,
    pub field_notes: String,
    pub field_number: String,
}
