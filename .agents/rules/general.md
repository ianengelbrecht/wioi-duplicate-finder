# General Project Guidelines

This file contains general guidelines for the **Herbarium Specimen Duplicate Finder and Capture Tool** project. All developers and AI agents must adhere to these guidelines.

## Project Overview

This is a cross-platform desktop application built with:

- **Frontend**: SvelteKit (Svelte v5) + Vite + TailwindCSS (v4)
- **Backend**: Rust + Tauri (v2)
- **Database**: SQLite (managed with `rusqlite` bundled)

The application helps botanical researchers digitise herbarium specimens by finding duplicates from reference databases (GBIF & Kew Checklist) and capturing/editing records locally.

---

## Architectural Principles

The application follows a layered architecture with clearly separated responsibilities.

```text
UI Components
      ↓
Routes / Pages
      ↓
Frontend Services
      ↓
Tauri Commands
      ↓
Backend Services
      ↓
Repositories
      ↓
SQLite Database
```

Each layer has a specific responsibility and must not bypass intermediate layers.

### Allowed Dependencies

```text
Component → Service → Tauri Command → Backend Service → Repository → Database
```

### Not Allowed

```text
Component → Database
Component → Repository
Component → Tauri Command
Service → Database
Repository → UI
```

Business logic must never be embedded directly in UI components.

Database access must always occur through repository modules.

---

## Workspace Structure

### Frontend

```text
src/
├── routes/
├── lib/
│   ├── components/
│   ├── services/
│   ├── stores/
│   ├── types/
│   ├── utils/
│   └── constants/
├── app.css
└── app.html
```

### Backend

```text
src-tauri/
├── src/
│   ├── commands/
│   ├── services/
│   ├── repositories/
│   ├── models/
│   ├── db/
│   ├── parsers/
│   ├── lib.rs
│   └── main.rs
└── tauri.conf.json
```

As the application grows, code should be organised by domain where appropriate.

Example:

```text
specimens/
    specimen_service.rs
    specimen_repository.rs
    specimen_types.rs

taxonomy/
    taxonomy_service.rs
    taxonomy_repository.rs
    taxonomy_types.rs
```

Prefer domain-oriented organisation over large generic files.

---

## Frontend Responsibilities

### Components

Components are responsible for:

- Rendering UI
- Handling user interaction
- Displaying state
- Emitting events

Components should not:

- Execute database queries
- Perform business logic
- Call SQLite directly
- Contain complex data transformation logic

### Routes

Routes are responsible for:

- Page composition
- Wiring components together
- Coordinating data loading

Routes should delegate business logic to services.

### Services

Frontend services are responsible for:

- Calling Tauri commands
- Validation
- Data transformation
- Workflow orchestration

Examples:

```text
specimenService.ts
duplicateFinderService.ts
taxonomyService.ts
labelExportService.ts
```

### Stores

Stores are responsible only for application state.

Stores should not:

- Execute database queries
- Perform business logic
- Contain parsing logic

### Utilities

Utility functions should be pure, reusable helper functions.

Examples:

```text
dateUtils.ts
stringUtils.ts
coordinateUtils.ts
```

Avoid placing application business rules in utility modules.

---

## Backend Responsibilities

### Commands

Tauri commands act as IPC endpoints between the frontend and backend.

Responsibilities:

- Receive requests
- Validate input
- Call backend services
- Return structured results

Commands should contain minimal logic.

### Services

Services contain application business logic.

Examples:

```text
specimen_service.rs
duplicate_service.rs
taxonomy_service.rs
import_service.rs
```

Responsibilities:

- Validation
- Duplicate matching
- Import workflows
- Export workflows
- Record processing

### Repositories

Repositories are responsible for all database access.

Examples:

```text
specimen_repository.rs
session_repository.rs
taxonomy_repository.rs
```

Responsibilities:

- SQL queries
- Inserts
- Updates
- Deletes

Repositories should not contain business rules.

### Parsers

All parsing and normalisation logic belongs in dedicated parser modules.

Examples:

```text
date_parser.rs
elevation_parser.rs
collector_parser.rs
scientific_name_parser.rs
```

Do not embed parsing logic inside:

- Components
- Routes
- Commands
- Repositories

---

## Database Rules

### Repository Pattern

All database access must occur through repository modules.

Only repositories may execute SQL.

Avoid direct database access from:

- Components
- Frontend services
- Tauri commands
- Backend services

### Migrations

Schema changes must be implemented through migrations.

Never modify an existing migration after it has been committed.

Always create a new migration for schema changes.

---

## Data Models

Frontend and backend models should remain aligned wherever practical.

Primary entities include:

```text
Specimen
Session
Taxon
DuplicateMatch
ReferenceRecord
```

Prefer explicit interfaces and structs over anonymous objects.

All API responses should be strongly typed.

---

## Coding Style & Documentation

### Comments & Documentation

- Do not remove existing comments or docstrings unless explicitly requested.
- Public functions should include concise documentation where appropriate.
- Complex business rules should be documented.

### Typing

- Use strong typing throughout the application.
- Prefer explicit interfaces and Rust structs.
- Avoid use of `any` except where unavoidable.

### Error Handling

Backend:

- All Tauri commands should return `Result<T, String>`.
- Errors should be descriptive and actionable.

Frontend:

- Handle all command failures gracefully.
- Display user-friendly error messages.
- Never silently swallow errors.

---

## Design & Aesthetics

### Visual Style

The application should maintain a modern, professional appearance suitable for scientific data capture and collection management.

Use the Slate colour system consistently:

- `bg-slate-50` for page backgrounds
- `bg-white` for primary containers
- `text-slate-900` for headings
- `border-slate-300` for form controls

### Layout

- Prefer structured grid-based layouts.
- Use `rounded-none` throughout the application.
- Maintain consistent spacing and alignment.

### Interactive Elements

Interactive controls should provide clear feedback through hover and focus states.

Examples:

```text
hover:bg-slate-900
focus:border-slate-500
focus:ring-slate-500
```

### Typography

- Use **Outfit** for headings and major titles.
- Use **Inter** for body text and metadata.

### Component Integrity

Keep components:

- Modular
- Focused
- Reusable
- Self-contained

Avoid large monolithic components.

---

## AI Agent Guidelines

When implementing new functionality:

1. Create or update a service.
2. Create or update a repository if database access is required.
3. Expose functionality through a Tauri command.
4. Consume functionality through a frontend service.
5. Keep UI components presentation-focused.

Before creating a new file:

- Check for an existing domain module.
- Extend existing functionality where appropriate.
- Avoid creating generic files such as:
  - `helpers.js`
  - `misc.js`
  - `utils2.js`
  - `dbService.js`

Prefer descriptive, domain-specific naming.

### Svelte Requirements

- Use Svelte 5 syntax exclusively.
- Follow existing project patterns.
- Prefer small, focused components.
- Avoid unnecessary reactive complexity.

### General Principle

The codebase should remain understandable to a new developer after years of maintenance.

Favour clarity, explicit structure, and separation of concerns over convenience.
