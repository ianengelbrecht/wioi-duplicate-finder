# Database & SQLite Guidelines

This file governs database schema design, migrations, indexing, and SQLite query usage within the application.

---

## Database Overview

The application uses SQLite via `rusqlite`.

The primary database file is:

```text
reference.db
```

It is stored in the user's local application data directory.

The database contains two main categories of data:

1. **Reference data**
   Imported/preloaded data used for duplicate searching and lookup.

2. **Local capture data**
   User-created specimen records, sessions, and application data.

Reference data should generally be treated as read-only after import.

Local capture data is editable application data.

---

## SQLite Configuration

Always enable Write-Ahead Logging and normal synchronous mode:

```sql
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
PRAGMA foreign_keys=ON;
```

Foreign key enforcement must always be enabled when opening a database connection.

---

## Data Domain Rules

### Reference Data

Reference tables include:

- `gbif`
- `wcvp_taxonomy`
- `agents`

Reference data is used for:

- Duplicate detection
- Autocomplete
- Taxonomic lookup
- Data normalization support

Reference records should not be edited directly through normal capture workflows.

If reference data must be updated, it should be handled through an explicit import/update process.

### Local Capture Data

Local editable tables include:

- `users`
- `sessions`
- `captured_records`

These tables store user-created and user-edited application data.

Local capture data must never be overwritten by reference-data imports.

---

## Schema Structure

The database consists of reference data tables and local capture tables.

---

## Reference Data Tables

### `gbif` Table

Stores pre-loaded reference specimens downloaded from GBIF.

```sql
CREATE TABLE gbif (
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
    normalized_locality TEXT,
    fieldNumber TEXT,
    cleanedFieldNumber TEXT
);
```

### `wcvp_taxonomy` Table

Stores checklist taxonomic data from Kew WCVP.

```sql
CREATE TABLE wcvp_taxonomy (
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
);
```

### `agents` Table

Stores unique normalized collector or author names for autocomplete and matching.

```sql
CREATE TABLE agents (
    agentName TEXT PRIMARY KEY,
    searchAgentName TEXT NOT NULL
);
```

---

## Local Specimen Capture & User Tables

### `users` Table

```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### `sessions` Table

```sql
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_exported_at TEXT,
    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
);
```

### `captured_records` Table

Stores locally captured specimens per session.

Keep fields synchronized with standard `gbif`/Darwin Core-style fields where practical.

```sql
CREATE TABLE captured_records (
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
);
```

---

## Schema & Migration Rules

Database migrations are applied programmatically inside `run_migrations()` in `src-tauri/src/db.rs`.

### Migration Rules

- Never modify an existing migration after it has been committed.
- Always create a new migration for schema changes.
- Migrations must be idempotent where practical.
- Use explicit version tracking for schema upgrades.
- Log migration execution.
- Test migrations against an existing populated database.

### Destructive Changes

Destructive schema changes require extra care.

Avoid:

```sql
DROP TABLE captured_records;
```

unless there is a backup, migration path, or explicit user-facing reset operation.

For local capture data, preserve user records wherever possible.

---

## Database Access Rules

All SQL must be executed through backend repository modules.

Allowed:

```text
Command → Service → Repository → SQLite
```

Not allowed:

```text
Command → SQLite
Service → SQLite
Frontend → SQLite
```

Repositories are responsible for:

- Queries
- Inserts
- Updates
- Deletes
- Transactions

Services are responsible for business rules.

---

## Normalized & Derived Fields

Normalized fields exist to support matching, searching, and sorting.

Examples:

- `normalizedRecordedBy`
- `searchRecordedBy`
- `normalized_scientific_name`
- `normalized_locality`
- `cleanedFieldNumber`

Rules:

- Derived fields must be populated consistently.
- The same normalization logic must be used during import and capture.
- Normalization logic belongs in parser/normalizer modules, not directly in SQL unless there is a strong reason.
- If the source field changes, the derived field must be updated too.

---

## Indexing Rules

Columns frequently queried, joined, filtered, or sorted must be indexed.

Common index candidates:

- `gbif.recordNumber`
- `gbif.normalizedRecordedBy`
- `gbif.searchRecordedBy`
- `gbif.cleanedFieldNumber`
- `gbif.normalized_scientific_name`
- `gbif.normalized_locality`
- `gbif.country`
- `gbif.stateProvince`
- `captured_records.session_id`
- `captured_records.recordNumber`
- `captured_records.scientificName`
- `wcvp_taxonomy.ipni_id`
- `wcvp_taxonomy.plant_name_id`
- `wcvp_taxonomy.normalized_taxon_name`
- `agents.searchAgentName`

Index creation should be part of migrations.

Avoid adding indexes blindly. Indexes improve reads but slow writes and increase database size.

---

## FTS5 Virtual Search Indexes

The database uses FTS5 virtual tables for high-performance full-text searches.

```sql
CREATE VIRTUAL TABLE gbif_fts USING fts5(
    locality,
    locationRemarks,
    verbatimLocality,
    scientificName,
    normalized_scientific_name,
    normalized_locality,
    cleanedFieldNumber,
    content='gbif',
    content_rowid='gbifID'
);
```

### FTS Rules

- Keep FTS indexes synchronized with source tables.
- Rebuild FTS indexes after bulk imports when necessary.
- Do not rely on FTS for exact identifier matching where a normal indexed column is better.
- Prefer B-tree indexes for exact matches.
- Prefer FTS for text search across locality, remarks, and taxon fields.

---

## Database Triggers

Triggers may be used for automatic derived-field updates or FTS synchronization.

Known trigger categories:

- `gbif_ai`: after insert FTS sync
- `gbif_ad`: after delete FTS sync
- `gbif_au`: after update FTS sync
- `gbif_cfn_insert`: cleaned field number update
- `gbif_cfn_update`: cleaned field number update

### Trigger Rules

- Triggers must be documented.
- Triggers must be recreated carefully during migrations.
- Avoid recursive trigger behaviour unless intentional and tested.
- Prefer application-level normalization where logic is complex.

---

## Query Performance

Use `EXPLAIN QUERY PLAN` when optimizing slow queries.

Guidelines:

- Use exact matches where possible.
- Avoid leading-wildcard `LIKE` queries on large tables.
- Prefer FTS5 for partial text search.
- Use `LIMIT` for autocomplete and search previews.
- Use pagination for large result sets.
- Avoid loading entire reference tables into memory.

For example, this is usually slow on large indexed tables:

```sql
WHERE normalized_scientific_name LIKE '%acacia%'
```

Prefer FTS or prefix search strategies where appropriate.

---

## Transactions

Use transactions for multi-step operations.

Examples:

- Bulk imports
- Batch updates
- Session deletion
- Export status updates
- Derived-field recalculation

Avoid leaving the database in a partially updated state.

---

## Backups & Data Safety

Local capture data is user-created and must be protected.

Rules:

- Do not overwrite local capture data during reference imports.
- Provide a backup/export path before destructive operations.
- Clearly separate reset operations for reference data and captured data.
- Use transactions for destructive or bulk changes.
- Prefer soft failure over data loss.

---

## Import Rules

Bulk imports should:

- Use transactions.
- Use prepared statements.
- Disable or rebuild FTS indexes if appropriate for performance.
- Recreate indexes after large imports where appropriate.
- Log progress at useful intervals.
- Validate required columns before importing.
- Avoid corrupting existing local capture data.

Reference imports should affect only reference tables.

---

## Date & Boolean Storage

SQLite does not enforce strict date or boolean types.

Application rules:

- Store dates as ISO-like text where exact timestamps are needed.
- Store partial collection dates as separate `year`, `month`, and `day` integer fields.
- Store booleans as integers:
  - `0` = false
  - `1` = true

Do not store incomplete specimen event dates as invalid date strings.

---

## Naming Conventions

Prefer consistent field names that align with Darwin Core where practical.

Examples:

```text
recordedBy
recordNumber
scientificName
decimalLatitude
decimalLongitude
verbatimEventDate
```

Avoid introducing alternate names for the same concept unless required for export compatibility.

---

## Integrity Checks

Use SQLite integrity checks when database corruption is suspected.

```sql
PRAGMA integrity_check;
PRAGMA foreign_key_check;
```

Run integrity checks after unusual failures, interrupted imports, or suspected disk/database issues.

---

## AI Agent Guidelines

When changing database behaviour:

1. Check existing schema and migrations first.
2. Do not edit old migrations.
3. Add a new migration for schema changes.
4. Keep reference data and capture data separate.
5. Add indexes intentionally.
6. Keep SQL in repositories.
7. Preserve user-created data.
8. Test against an existing populated database where possible.

Avoid quick fixes that mutate production/user data without a clear migration or backup path.
