#!/usr/bin/env python3
"""
validate.py

Validates that a dataset directory contains occurrence_final.csv,
that all required columns for the database are present (including fallback checks),
and that basic record-level constraints are met.
"""

import argparse
import csv
import sys
from collections import Counter
from pathlib import Path

# =============================================================================
# CONFIGURATION & CONSTANTS
# =============================================================================

MAX_FIELD_SIZE = 10_000_000
PROGRESS_EVERY = 10_000

# Full list of columns required for mapping to the database (including new fields)
DATABASE_FIELDS = [
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
    "searchRecordedBy",
    "islandGroup",
    "island",
]


# =============================================================================
# HELPER FUNCTIONS
# =============================================================================

def clean(value):
    """Return a trimmed string, preserving blanks as empty strings."""
    return (value or "").strip()


# =============================================================================
# VALIDATION ENGINE
# =============================================================================

def validate_dataset(filepath: Path, max_rows: int = None):
    print(f"[*] Starting validation for dataset: {filepath.name}")
    print(f"[*] File Size: {filepath.stat().st_size / (1024 * 1024):.2f} MB")
    
    # Increase CSV field size limit
    csv.field_size_limit(MAX_FIELD_SIZE)

    # 1. Delimiter warning sniff
    with open(filepath, "r", encoding="utf-8-sig", errors="replace") as f:
        first_line = f.readline()
    
    tab_count = first_line.count("\t")
    comma_count = first_line.count(",")
    if tab_count > comma_count and tab_count > 1:
        print("[!] Warning: Detected more tabs than commas in the header line.")
        print("    The application expects a comma-separated CSV file (not a tab-separated TSV).")

    # 2. Header parsing
    with open(
        filepath,
        "r",
        encoding="utf-8-sig",
        errors="replace",
        newline=""
    ) as infile:
        reader = csv.reader(infile)
        try:
            headers = next(reader)
        except StopIteration:
            print("[-] Error: The file is empty.", file=sys.stderr)
            return False
        except csv.Error as e:
            print(f"[-] Error parsing header line: {e}", file=sys.stderr)
            return False

        header_map = {h.lower(): idx for idx, h in enumerate(headers)}
        get_idx = lambda name: header_map.get(name.lower())

        # Check column presence with alternative names
        col_indices = {}
        missing_fields = []

        for field in DATABASE_FIELDS:
            idx = get_idx(field)
            if idx is None:
                # Fallback alternatives
                if field == "gbifID":
                    idx = get_idx("id")
                elif field == "locationRemarks":
                    idx = get_idx("locationnotes")
            
            if idx is not None:
                col_indices[field] = idx
            else:
                missing_fields.append(field)

        # Diagnostic header output
        print("\n=== Header Analysis ===")
        
        # Primary Key check
        id_field = "gbifID" if "gbifID" in col_indices else ("id" if "id" in header_map else None)
        if id_field:
            id_idx = col_indices.get("gbifID") if "gbifID" in col_indices else header_map.get("id")
            print(f"[+] Found Primary ID column: '{headers[id_idx]}'")
        else:
            print("[-] Error: Missing primary identifier column ('gbifID' or 'id').")

        # Remarks fallback mapping check
        remarks_field = "locationRemarks" if "locationRemarks" in col_indices else ("locationnotes" if "locationnotes" in header_map else None)
        if remarks_field:
            remarks_idx = col_indices.get("locationRemarks") if "locationRemarks" in col_indices else header_map.get("locationnotes")
            print(f"[+] Found Location Remarks column: '{headers[remarks_idx]}'")

        # Missing database columns report
        if missing_fields:
            print("\n[-] Error: The following required columns are missing from the CSV:")
            for field in sorted(missing_fields):
                # Provide hints for alternatives
                if field == "gbifID":
                    print("  - gbifID (or 'id')")
                elif field == "locationRemarks":
                    print("  - locationRemarks (or 'locationnotes')")
                else:
                    print(f"  - {field}")
            print("\n[-] Validation halted: Missing required database fields.")
            return False
        else:
            print("[+] All database columns are present in headers.")

        # 3. Row-by-Row Checks (Minimal structure checking)
        print("\n=== Row-by-Row Validation ===")
        id_col_idx = col_indices["gbifID"]
        expected_cols = len(headers)
        
        total_rows = 0
        errors_count = 0
        warnings_count = 0
        
        seen_ids = set()
        duplicate_ids = set()
        col_mismatch_count = 0
        
        sample_errors = []
        sample_warnings = []
        
        def add_error(row_num, message):
            nonlocal errors_count
            errors_count += 1
            if len(sample_errors) < 20:
                sample_errors.append(f"Row {row_num}: {message}")

        def add_warning(row_num, message):
            nonlocal warnings_count
            warnings_count += 1
            if len(sample_warnings) < 20:
                sample_warnings.append(f"Row {row_num}: {message}")

        for row_idx, row in enumerate(reader, start=2):
            if max_rows and total_rows >= max_rows:
                print(f"[*] Hit scan limit of {max_rows} rows. Stopping scan.")
                break

            total_rows += 1
            if total_rows % PROGRESS_EVERY == 0:
                print(f"\rScanned {total_rows:,} rows...", end="", flush=True)

            actual_cols = len(row)
            if actual_cols != expected_cols:
                col_mismatch_count += 1
                add_error(row_idx, f"Column count mismatch. Expected {expected_cols}, got {actual_cols}.")
                continue

            # Verify Primary Key (gbifID/id) is a positive integer and unique
            raw_id = clean(row[id_col_idx])
            if not raw_id:
                add_error(row_idx, f"ID field is blank.")
            else:
                try:
                    val_id = int(raw_id)
                    if val_id <= 0:
                        add_error(row_idx, f"ID field must be a positive integer (got '{raw_id}').")
                    elif val_id in seen_ids:
                        duplicate_ids.add(val_id)
                        add_error(row_idx, f"Duplicate ID detected: {val_id}.")
                    else:
                        seen_ids.add(val_id)
                except ValueError:
                    add_error(row_idx, f"ID field is not a valid integer: '{raw_id}'.")

        # Print new line if progress line was written
        if total_rows >= PROGRESS_EVERY:
            print()

        # 4. Print Summary Report
        print("\n=== Validation Summary ===")
        print(f"Total Rows Scanned: {total_rows:,}")
        print(f"Total Errors Found: {errors_count:,}")
        print(f"Duplicate IDs:      {len(duplicate_ids):,}")
        
        if col_mismatch_count > 0:
            print(f"Column count mismatches: {col_mismatch_count:,}")

        # Print samples if any
        if sample_errors:
            print("\n--- Error Samples (Max 20 Shown) ---")
            for err in sample_errors:
                print(f"  [Error] {err}")

        # Final Verdict
        print("\n=== Final Verdict ===")
        if errors_count > 0:
            print("[FAIL] VALIDATION FAILED: The file contains errors that will break the database schema or prevent records from importing correctly.")
            print("       Please fix the errors (e.g. non-integer primary keys, blank/duplicate IDs, column mismatches) before importing.")
            return False
        else:
            print("[SUCCESS] VALIDATION SUCCESSFUL: The file contains all required headers and correct formatting!")
            return True


# =============================================================================
# ENTRY POINT
# =============================================================================

def main():
    parser = argparse.ArgumentParser(
        description="Validates a CSV dataset for importing into the duplicate finder application."
    )
    parser.add_argument(
        "directory",
        type=str,
        help="Path to the directory containing the dataset (expects 'occurrence_final.csv' inside it)."
    )
    parser.add_argument(
        "-l", "--limit",
        type=int,
        default=None,
        help="Limit the validation to the first N records (for quick testing on large files)."
    )
    
    args = parser.parse_args()
    
    dir_path = Path(args.directory)
    if not dir_path.is_dir():
        print(f"[-] Error: Directory does not exist or is not a directory: {dir_path}", file=sys.stderr)
        sys.exit(1)
        
    filepath = dir_path / "occurrence_final.csv"
    if not filepath.exists():
        print(f"[-] Error: Input file 'occurrence_final.csv' not found in directory: {dir_path}", file=sys.stderr)
        sys.exit(1)
        
    success = validate_dataset(filepath, max_rows=args.limit)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
