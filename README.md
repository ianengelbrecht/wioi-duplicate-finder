# WIOI Herbarium Duplicate Specimen Finder & Capture Tool

An advanced, high-performance desktop application designed for botanists and herbarium curators to capture specimen records and automatically identify duplicate specimens. Leveraging a fast local SQLite database pre-populated with millions of reference records from the **GBIF (Global Biodiversity Information Facility)** and **Kew's WCVP (World Checklist of Vascular Plants)** taxonomy, the tool streamlines specimen digitisation, standardises taxonomic names, and maintains high data integrity.

---

## 🚀 Key Features

### 🔍 1. Reference Database Search
* **Rule-Based Constraints**: Prevents database scans through search constraints on collector names, collector numbers, date elements, and geographic areas.
* **FTS5 Full-Text Search**: Instant search capability across millions of records.
* **Dynamic Indicators**: Real-time display showing the exact count of records inside the GBIF reference table (e.g. `530,767 Records`) directly in the search panel.

### 📝 2. Specimen Capture Form
* **Verbatim Date Parser**: An intelligent `onblur` event parser that extracts year, month, and day integers from verbatim dates (e.g. `12 October 2026`, `May 1994`, `2026-10-12`) automatically.
* **Hierarchical Geography Autocompletes**: Implements single-value autocompletes for **Country**, **Admin 2 (State/Province)**, **Admin 3 (County)**, and **Admin 4 (Municipality)**. Options are filtered dynamically based on parent selections in the hierarchy (e.g. choosing Madagascar restricts Admin 2 suggestions to Madagascan regions).
* **Taxonomic Autocomplete**: Integrates Kew WCVP v12 taxonomy autocomplete, querying over 1.4M taxa using sequence-exact phrase match searches to prevent matching spelling variations. Displays taxonomic version and total taxa count (e.g., `Kew WCVP v12 (1,441,155 Taxa)`) dynamically on the form header.
* **Collector Matching**: Normalises spelling, diacritics, and punctuation (e.g. matching `Müller-Landry` when searching `mullerlandry`) to identify spelling variations across collectors.
* **Proper-Case Helpers (`Aa`)**: Quick-format buttons next to text inputs to automatically proper-case geographic names and localities.
* **Clean Formatting**: Autocomplete fields automatically hide placeholder hints if any one of the geographic input fields is populated.

### 💾 3. Data Integrity & Performance
* **WAL Mode (Write-Ahead Logging)**: Configured with `journal_mode=WAL` and `synchronous=NORMAL` to prevent database locks during concurrent search queries and database writes.
* **Startup/Shutdown Checks**: Performs a fast `quick_check` structural scan on startup, and runs query-planner index optimisations (`PRAGMA optimize`) and checks on exit.
* **Optimized Bulk Normalization**: Fast, trigger-less startup indexing that automatically processes externally inserted records in seconds.
* **Bundled DB Seeding Protection**: Fully protects pre-populated databases from being dropped or overwritten during migrations.

### 📥 4. Session & Darwin Core Export
* **Native File Exporter**: Uses native platform save dialogues to prompt users for locations to save files.
* **Darwin Core Mappings**: Exports captured sessions directly into standard Darwin Core CSV formatted data, maintaining precise column layouts.
* **Safe Deletion**: Session deletion warning prompts ensure data is not lost accidentally.

---

## 🛠️ Tech Stack

* **Backend**: Rust, [Tauri v2](https://tauri.app/) (native OS wrapper & security layer), [rusqlite](https://github.com/rusqlite/rusqlite) (SQLite interface).
* **Frontend**: SvelteKit, [Svelte 5](https://svelte.dev/) (reactive UI framework), [Tailwind CSS](https://tailwindcss.com/) (modern typography, dark layout styles, and fluid grid systems).
* **Database**: SQLite with **FTS5** (Full-Text Search) extension.

---

## 💻 Installation & Setup

### Prerequisites
* [Node.js](https://nodejs.org/) (v18 or higher)
* [Rust Compiler](https://www.rust-lang.org/) (cargo, rustc)
* C++ Build tools installed (via Visual Studio Installer on Windows)

### 1. Clone the Repository & Install Node Dependencies
```bash
npm install
```

### 2. Prepare the Reference Database
Place your pre-populated SQLite database in the Tauri resources folder:
```text
src-tauri/resources/reference.db
```
*Note: On startup, if no local database file exists in your user's AppData directory, the app will automatically copy this pre-bundled template into your application folder.*

---

## 🏃 Running the Application

### Start Development Server
Launches the Svelte dev server and boots up the Tauri desktop container:
```bash
npm run tauri dev
```

### Build Production Desktop Application
Generates a standalone, optimized installer (`.msi` or `.exe` on Windows):
```bash
npm run tauri build
```

---

## 📁 Recommended IDE Setup

* **VS Code** + **Svelte Beta/Official** extension + **Tauri VSCode** + **rust-analyzer**.
