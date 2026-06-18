# Herbarium Specimen Duplicate Finder - Agent Rules & Conventions

This workspace contains rules, behavior guidelines, and coding conventions for AI agents and developers working on the **Herbarium Specimen Duplicate Finder and Capture Tool** project.

## Technology Stack & Core Architecture

The project is structured as a cross-platform desktop application built with:
- **Frontend**: SvelteKit (Svelte v5) + Vite + Tailwind CSS (v4)
- **Backend**: Rust + Tauri (v2)
- **Database**: SQLite (managed with `rusqlite` bundled)

### Layered Architecture Flow
All changes must strictly adhere to the layered architecture diagram:
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

#### Allowed Dependencies
- `Component` → `Service` → `Tauri Command` → `Backend Service` → `Repository` → `Database`

#### Prohibited Dependencies
- Never call Tauri `invoke()` directly from components or stores; always abstract behind a service.
- Never write SQL or connect to SQLite directly from components, services, or commands; always access SQLite through the backend repository modules.

---

## Coding Conventions & Rules

### 1. Frontend Development (Svelte 5 / JavaScript)
- **Svelte 5 Runes**: Use Svelte 5 syntax (`$state`, `$effect`, `$derived`, etc.) exclusively.
- **Tailwind CSS v4**: Styling is done using Tailwind CSS v4 utility classes.
- **Language**: The frontend is written in pure JavaScript (`.js` and `.svelte`). Do not introduce TypeScript (`.ts`) files.
- **Type Safety via JSDoc**: The compiler runs strict type check validation using TypeScript via Svelte-check (`checkJs: true` and `strict: true` in `jsconfig.json`). You **must** provide full JSDoc typing annotations for all services, utility functions, stores, and custom components.
- **Design Tokens**:
  - Spacing & Layout: Prefer structured grid-based layouts with `rounded-none` borders and containers.
  - Colors: Use the Slate color system consistently (e.g. `bg-slate-50` for page backgrounds, `bg-white` for primary containers, `text-slate-900` for headings, `border-slate-300` for form controls).
  - Fonts: Headings use the `Outfit` font family; body text uses the `Inter` font family.

### 2. Backend Development (Rust / Tauri v2)
- **Error Handling**: Tauri commands must return a standard `Result<T, String>` where the `Err` variant is a descriptive, human-readable string suitable for display in the UI. Avoid `unwrap()` and `expect()` outside of unrecoverable application startup.
- **Database Access**: All SQL queries, transactions, inserts, updates, and deletes must occur exclusively within backend repository files (e.g., `src-tauri/src/repositories/`).
- **SQLite Configuration**: Always open database connections with Write-Ahead Logging (`journal_mode=WAL`), normal synchronous mode (`synchronous=NORMAL`), and foreign key enforcement (`foreign_keys=ON`).
- **Logging**: Use the `log` crate macros (`info!`, `warn!`, `error!`, `debug!`). Do not use `println!` or `eprintln!` for application logging.
- **Style Rules**: Always run `cargo fmt` and `cargo clippy` on the backend before committing code.

### 3. Database Migrations
- **Dynamic Migrations**: Schema migrations are handled dynamically inside `run_migrations()` in [src-tauri/src/db/mod.rs](file:///c:/devprojects/clients/WIOI%20herbaria/duplicate-finder/src-tauri/src/db/mod.rs) by checking if tables or columns exist using `pragma_table_info`.
- **Immutability of Migrations**: Never modify an existing schema migration once committed. Always implement additions or schema modifications as a new checks/updates step in the migration handler.
- **Data Protection**: Local capture tables (`captured_records`, `sessions`, `users`) contain user-created data that must never be overwritten, modified destructively, or deleted without explicit backups.

---

## Detailed Rules Files

For detailed guidelines on specific areas of the codebase, refer to:
1. **General Guidelines**: [general.md](file:///c:/devprojects/clients/WIOI%20herbaria/duplicate-finder/.agents/rules/general.md)
2. **Frontend Guidelines**: [frontend.md](file:///c:/devprojects/clients/WIOI%20herbaria/duplicate-finder/.agents/rules/frontend.md)
3. **Rust Backend Guidelines**: [backend.md](file:///c:/devprojects/clients/WIOI%20herbaria/duplicate-finder/.agents/rules/backend.md)
4. **Database Guidelines**: [database.md](file:///c:/devprojects/clients/WIOI%20herbaria/duplicate-finder/.agents/rules/database.md)
