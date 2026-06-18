# Rust Backend Development Guidelines

This file governs the development of the Rust backend (`src-tauri`) for the Duplicate Finder application.

---

## Backend Architecture

The backend follows a layered architecture.

```text id="s6g49n"
Tauri Commands
       ↓
Services
       ↓
Repositories
       ↓
SQLite Database
```

### Layer Responsibilities

#### Commands

Commands are IPC endpoints exposed to the frontend.

Responsibilities:

- Receive requests
- Validate inputs
- Call services
- Return structured results

Commands should contain minimal logic.

#### Services

Services contain application business logic.

Examples:

```text id="2zqmxr"
specimen_service.rs
duplicate_service.rs
taxonomy_service.rs
import_service.rs
export_service.rs
```

Responsibilities:

- Validation
- Workflow orchestration
- Duplicate matching
- Import/export operations
- Business rules

Services should not execute SQL directly.

#### Repositories

Repositories are responsible for database access.

Examples:

```text id="7n8hhm"
specimen_repository.rs
session_repository.rs
taxonomy_repository.rs
```

Responsibilities:

- Queries
- Inserts
- Updates
- Deletes
- Transactions

Repositories should not contain business rules.

---

## Backend Directory Structure

Recommended structure:

```text id="pvvtnj"
src-tauri/src/
├── commands/
├── services/
├── repositories/
├── models/
├── parsers/
├── db/
├── lib.rs
└── main.rs
```

### commands/

IPC endpoints exposed to the frontend.

### services/

Business logic.

### repositories/

Database access layer.

### models/

Shared application structs and DTOs.

### parsers/

Parsing and normalization logic.

### db/

Database connection management and migrations.

---

## Tauri Commands & IPC

### Command Registration

All commands must be registered in `lib.rs`.

### Return Types

Commands should return:

```rust id="ifw7ao"
Result<T, String>
```

where the error string is human-readable and suitable for display in the UI.

### Serialization

Use `serde` for all frontend/backend communication.

All request and response types should derive appropriate serialization traits.

### Async Operations

Avoid blocking the UI thread.

Long-running operations should use asynchronous patterns where practical.

Examples:

- Large imports
- Duplicate matching
- Bulk updates
- Data exports

---

## Database Access Rules

### Repository Pattern

All database access must occur through repository modules.

Allowed:

```text id="3vsl94"
Command
   ↓
Service
   ↓
Repository
   ↓
SQLite
```

Not allowed:

```text id="uol3vy"
Command → SQLite
Service → SQLite
Parser → SQLite
```

Only repositories may execute SQL.

### Transactions

Multi-step database operations should use transactions.

Examples:

- Imports
- Batch updates
- Bulk deletes
- Data migrations

Avoid partial updates that can leave the database in an inconsistent state.

### Migrations

Schema changes must be implemented through migrations.

Never modify an existing migration after it has been committed.

Always create a new migration for schema changes.

---

## Models & Types

All major entities should have explicit Rust structs.

Examples:

```text id="uzaklq"
Specimen
Session
Taxon
DuplicateMatch
ReferenceRecord
```

Avoid passing loosely structured JSON objects through the application.

Prefer strongly typed models.

### DTOs

When frontend-facing structures differ from internal structures, create dedicated DTOs.

Examples:

```text id="n7t7m4"
SpecimenDto
SearchResultDto
DuplicateMatchDto
```

---

## Parsing & Normalization

All parsing and normalization logic belongs in parser modules.

Examples:

```text id="hvvrjl"
scientific_name_parser.rs
collector_parser.rs
date_parser.rs
elevation_parser.rs
coordinate_parser.rs
```

Parsing logic should not be embedded in:

- Commands
- Repositories
- Database migrations

### Parser Design

Parsers should:

- Be deterministic
- Be independently testable
- Avoid side effects

Where possible:

```text id="6ffw2g"
input -> output
```

with no external dependencies.

---

## Error Handling

### General Principles

Errors should be:

- Explicit
- Descriptive
- Recoverable where possible

Avoid:

```rust id="pkh48p"
unwrap()
expect()
```

except in startup code where failure is unrecoverable.

Prefer:

```rust id="l2qh93"
?
map_err(...)
```

### User-Facing Errors

Errors returned to the frontend should be understandable by non-technical users.

Avoid exposing:

- SQL errors
- Internal file paths
- Debug-only details

Log technical details separately.

---

## Logging

Use the `log` crate.

Examples:

```rust id="jlu4xv"
info!()
warn!()
error!()
debug!()
```

Do not use:

```rust id="zbxd1q"
println!()
eprintln!()
```

for application logging.

### What to Log

Log:

- Application startup
- Database initialization
- Imports
- Exports
- Unexpected failures
- Migration execution

Avoid excessive logging inside high-frequency loops.

---

## Rust Style & Dependencies

### Dependencies

Keep dependencies minimal.

Before adding a crate:

1. Check existing dependencies.
2. Prefer the standard library where practical.
3. Justify large new dependencies.

### Formatting

Before committing:

```bash id="o56o6v"
cargo fmt
cargo clippy
```

Code should compile without warnings where practical.

### Ownership & Borrowing

Prefer borrowing over cloning.

Avoid unnecessary allocations.

Use `Arc`, `Mutex`, or other synchronization primitives only when genuinely required.

---

## Documentation

### Public APIs

Public structs, enums, and functions should have documentation comments.

Example:

```rust id="59gib5"
/// Finds potential duplicate specimens using
/// collector, collector number, and date criteria.
```

### Complex Logic

Business rules and algorithms should include concise explanations of:

- Purpose
- Assumptions
- Edge cases

### Platform-Specific Behaviour

Document any:

- Windows-specific logic
- macOS-specific logic
- Linux-specific logic
- Filesystem assumptions

---

## Testing Considerations

Business logic should be testable independently from Tauri.

Preferred structure:

```text id="k8yv8r"
Command
   ↓
Service
   ↓
Repository
```

so that services and parsers can be unit-tested without IPC involvement.

### Unit Tests

Prioritize tests for:

- Parsing logic
- Normalization routines
- Duplicate matching
- Taxonomic processing
- Validation rules

---

## AI Agent Guidelines

When implementing backend functionality:

1. Check for an existing service before creating a new one.
2. Place business rules in services.
3. Place SQL in repositories.
4. Place parsing in parsers.
5. Expose functionality through commands only when required by the frontend.
6. Prefer extending existing modules over creating new generic files.

Avoid creating files such as:

```text id="rn17p7"
helpers.rs
misc.rs
utils2.rs
database_stuff.rs
```

Prefer descriptive, domain-specific names.

### General Principle

The backend should remain understandable and maintainable after years of development.

Favour strong typing, clear separation of concerns, and explicit business logic over convenience.
