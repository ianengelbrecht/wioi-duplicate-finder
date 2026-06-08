import sqlite3
import shutil
import os

db_path = r"C:\Users\ianic\AppData\Roaming\com.ianic.tauri-app\reference.db"
backup_path = r"C:\Users\ianic\AppData\Roaming\com.ianic.tauri-app\reference_backup_duplicates_fix.db"

# 1. Back up database
print(f"Checking database path: {db_path}")
if not os.path.exists(db_path):
    print(f"Error: Database file not found at {db_path}")
    exit(1)

shutil.copy2(db_path, backup_path)
print(f"Backup created successfully at: {backup_path}")

# 2. Recreate column structure via swap
conn = sqlite3.connect(db_path)
cursor = conn.cursor()

try:
    cursor.execute("BEGIN TRANSACTION;")
    
    # Add duplicates_temp as TEXT
    print("Adding duplicates_temp column...")
    cursor.execute("ALTER TABLE captured_records ADD COLUMN duplicates_temp TEXT;")
    
    # Copy and cast duplicates to TEXT
    print("Copying and casting duplicates to duplicates_temp...")
    cursor.execute("UPDATE captured_records SET duplicates_temp = CAST(duplicates AS TEXT);")
    
    # Drop original duplicates column
    print("Dropping original duplicates column...")
    cursor.execute("ALTER TABLE captured_records DROP COLUMN duplicates;")
    
    # Add new duplicates column as TEXT
    print("Adding new duplicates TEXT column...")
    cursor.execute("ALTER TABLE captured_records ADD COLUMN duplicates TEXT;")
    
    # Copy from duplicates_temp back to duplicates
    print("Copying back to duplicates column...")
    cursor.execute("UPDATE captured_records SET duplicates = duplicates_temp;")
    
    # Drop duplicates_temp
    print("Dropping duplicates_temp column...")
    cursor.execute("ALTER TABLE captured_records DROP COLUMN duplicates_temp;")
    
    conn.commit()
    print("Database schema updated successfully! Column 'duplicates' is now TEXT.")
except Exception as e:
    cursor.execute("ROLLBACK;")
    print(f"Error migrating database: {e}")
finally:
    conn.close()
