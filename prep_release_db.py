#!/usr/bin/env python3
"""
Prep Release Database Script
Checks the version of the application in tauri.conf.json,
copies the working database to src-tauri/resources/reference.db,
and empties out all data tables, triggers, and virtual FTS tables
to create a clean, lightweight schema database for distribution.
"""

import os
import sys
import json
import shutil
import sqlite3
import argparse

def get_tauri_app_data_dir():
    """Resolves the AppData/Application Support directory for the Tauri app."""
    if sys.platform == 'win32':
        base = os.environ.get('APPDATA')
        if base:
            return os.path.join(base, 'com.ianic.tauri-app')
    elif sys.platform == 'darwin':
        base = os.path.expanduser('~/Library/Application Support')
        return os.path.join(base, 'com.ianic.tauri-app')
    else:
        # Linux / Unix
        base = os.environ.get('XDG_DATA_HOME')
        if not base:
            base = os.path.expanduser('~/.local/share')
        return os.path.join(base, 'com.ianic.tauri-app')
    return None

def find_working_db():
    """Auto-detects the current active working database path."""
    app_data_dir = get_tauri_app_data_dir()
    if not app_data_dir or not os.path.exists(app_data_dir):
        return None

    # Try reading config.json for customized database_path
    config_path = os.path.join(app_data_dir, 'config.json')
    if os.path.exists(config_path):
        try:
            with open(config_path, 'r', encoding='utf-8') as f:
                config = json.load(f)
                db_path = config.get('database_path')
                if db_path and os.path.exists(db_path):
                    return db_path
        except Exception as e:
            print(f"Warning: Failed to parse config.json: {e}", file=sys.stderr)

    # Fallback to standard names in the AppData directory
    for name in ['reference.db', 'duplicate-finder.db']:
        p = os.path.join(app_data_dir, name)
        if os.path.exists(p):
            return p
            
    return None

def main():
    parser = argparse.ArgumentParser(
        description="Prepares a clean, empty reference database for Tauri application release bundling."
    )
    parser.add_argument(
        "--src",
        help="Path to the source working database. If not specified, attempts to auto-detect from Tauri AppData folder."
    )
    parser.add_argument(
        "--dest",
        help="Path where the cleaned database should be saved. Defaults to 'src-tauri/resources/reference.db'."
    )
    parser.add_argument(
        "--tauri-config",
        default=os.path.join("src-tauri", "tauri.conf.json"),
        help="Path to the tauri.conf.json file. Defaults to 'src-tauri/tauri.conf.json'."
    )
    parser.add_argument(
        "--no-vacuum",
        action="store_true",
        help="Disable running VACUUM on the database to reclaim space."
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Dry run mode. Shows what would be cleared without modifying files."
    )

    args = parser.parse_args()

    # 1. Read the application version from tauri.conf.json
    version = "Unknown"
    if os.path.exists(args.tauri_config):
        try:
            with open(args.tauri_config, 'r', encoding='utf-8') as f:
                tauri_conf = json.load(f)
                version = tauri_conf.get('version', '0.0.0')
            print(f"[+] Read application version: {version}")
        except Exception as e:
            print(f"[-] Warning: Failed to read version from {args.tauri_config}: {e}", file=sys.stderr)
    else:
        print(f"[-] Warning: {args.tauri_config} not found. Cannot check version.", file=sys.stderr)

    # 2. Determine source database path
    src_db = args.src
    if not src_db:
        src_db = find_working_db()
        if not src_db:
            print("[-] Error: Could not auto-detect working database. Please specify it using --src <path>.", file=sys.stderr)
            sys.exit(1)
    
    if not os.path.exists(src_db):
        print(f"[-] Error: Source database not found at '{src_db}'", file=sys.stderr)
        sys.exit(1)

    print(f"[+] Found working database at: {src_db}")

    # 3. Determine destination path
    dest_path = args.dest
    if not dest_path:
        # Check standard path
        dest_dir = os.path.join("src-tauri", "resources")
        dest_path = os.path.join(dest_dir, "reference.db")
    else:
        dest_dir = os.path.dirname(dest_path)

    print(f"[+] Destination path: {dest_path}")

    if args.dry_run:
        print("[*] Dry run mode enabled. No files will be modified.")
        conn = sqlite3.connect(src_db)
        cursor = conn.cursor()
        try:
            cursor.execute("SELECT name FROM sqlite_master WHERE type='trigger';")
            triggers = [r[0] for r in cursor.fetchall()]
            cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND sql LIKE '%USING fts%';")
            virtual_tables = [r[0] for r in cursor.fetchall()]
            cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';")
            standard_tables = [r[0] for r in cursor.fetchall()]
            
            print(f"Triggers that would be dropped: {triggers}")
            print(f"FTS tables that would be dropped: {virtual_tables}")
            print(f"Data tables that would be cleared: {standard_tables}")
        finally:
            conn.close()
        return

    # 4. Copy database to destination
    if dest_dir and not os.path.exists(dest_dir):
        print(f"[+] Creating destination directory: {dest_dir}")
        os.makedirs(dest_dir, exist_ok=True)

    if os.path.exists(dest_path):
        print(f"[+] Overwriting existing database file at destination: {dest_path}")
        try:
            os.remove(dest_path)
        except Exception as e:
            print(f"[-] Warning: Failed to remove old dest file: {e}. Will attempt to overwrite.", file=sys.stderr)

    print("[+] Copying working database to destination using SQLite Backup API...")
    try:
        src_conn = sqlite3.connect(src_db)
        dest_conn = sqlite3.connect(dest_path)
        with src_conn:
            src_conn.backup(dest_conn)
        dest_conn.close()
        src_conn.close()
        print("[+] Copy completed successfully.")
    except Exception as e:
        print(f"[-] Warning: Backup API copy failed: {e}. Falling back to file copy.", file=sys.stderr)
        try:
            shutil.copy2(src_db, dest_path)
            print("[+] Fallback file copy completed.")
        except Exception as copy_err:
            print(f"[-] Error: Copy failed: {copy_err}", file=sys.stderr)
            sys.exit(1)

    initial_size = os.path.getsize(dest_path)
    print(f"[+] Copied database initial size: {initial_size / (1024*1024):.2f} MB")

    # 5. Connect to copied database and clear it
    print("[+] Connecting to copied database to clear tables...")
    conn = sqlite3.connect(dest_path)
    cursor = conn.cursor()

    try:
        # Disable foreign keys during table clearing
        cursor.execute("PRAGMA foreign_keys = OFF;")

        # Find and drop triggers
        cursor.execute("SELECT name FROM sqlite_master WHERE type='trigger';")
        triggers = [r[0] for r in cursor.fetchall()]
        for trigger in triggers:
            cursor.execute(f"DROP TRIGGER IF EXISTS {trigger};")
            print(f"    - Dropped trigger: {trigger}")

        # Find and drop FTS virtual tables
        cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND sql LIKE '%USING fts%';")
        virtual_tables = [r[0] for r in cursor.fetchall()]
        for vt in virtual_tables:
            cursor.execute(f"DROP TABLE IF EXISTS {vt};")
            print(f"    - Dropped FTS virtual table: {vt}")

        # Find and clear all standard tables
        cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%';")
        standard_tables = [r[0] for r in cursor.fetchall()]
        for table in standard_tables:
            cursor.execute(f"DELETE FROM {table};")
            print(f"    - Cleared data table: {table}")

        # Reset autoincrement sequence counters
        try:
            cursor.execute("DELETE FROM sqlite_sequence;")
            print("    - Cleared sqlite_sequence")
        except sqlite3.OperationalError:
            pass

        conn.commit()
        print("[+] All tables successfully cleared and committed.")

        # Re-enable foreign keys and run integrity check
        cursor.execute("PRAGMA foreign_keys = ON;")
        cursor.execute("PRAGMA integrity_check;")
        status = cursor.fetchone()[0]
        if status != "ok":
            print(f"[-] Warning: Database integrity check returned: {status}", file=sys.stderr)
        else:
            print("[+] Database integrity check: OK")

        # Reclaim space
        if not args.no_vacuum:
            print("[+] Vacuuming database to reclaim space...")
            cursor.execute("VACUUM;")
            print("[+] Vacuum completed.")

    except Exception as e:
        print(f"[-] Error during database cleaning: {e}", file=sys.stderr)
        conn.rollback()
        conn.close()
        sys.exit(1)
    finally:
        conn.close()

    final_size = os.path.getsize(dest_path)
    print(f"[+] Prepared database final size: {final_size / 1024:.2f} KB")
    print(f"[+] Space saved: {(initial_size - final_size) / (1024*1024):.2f} MB")
    print(f"[+] Successfully prepared reference database for version {version}!")

if __name__ == "__main__":
    main()
