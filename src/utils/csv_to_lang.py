#!/usr/bin/env python3
import json
import csv
import os
import argparse

def main():
    parser = argparse.ArgumentParser(description="Apply translation updates from a CSV file back to language JSON files.")
    parser.add_argument("csv_input", help="Path to the input CSV file containing columns: key, lang1, lang2...")
    parser.add_argument("--dir", "-d", default="static/lang", help="Directory where the language JSON files are stored (default: static/lang)")
    
    args = parser.parse_args()
    
    if not os.path.exists(args.csv_input):
        print(f"Error: CSV file '{args.csv_input}' does not exist.")
        return
        
    if not os.path.exists(args.dir):
        print(f"Error: Language directory '{args.dir}' does not exist.")
        return

    # Read CSV
    with open(args.csv_input, "r", encoding="utf-8") as f:
        reader = csv.reader(f)
        try:
            headers = next(reader)
        except StopIteration:
            print("Error: The CSV file is empty.")
            return

    if not headers or headers[0].lower() != "key":
        print("Error: The first column of the CSV must be 'key'.")
        return

    lang_codes = headers[1:]
    if not lang_codes:
        print("Error: No language columns found in CSV header.")
        return

    # Initialize translation updates for each language column
    updates = {lang: {} for lang in lang_codes}
    
    # Read rows
    with open(args.csv_input, "r", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            key = row.get("key")
            if not key:
                continue
            for lang in lang_codes:
                # Store the value, converting None to empty string if missing in a row
                val = row.get(lang)
                updates[lang][key] = val if val is not None else ""

    # For each language, load existing JSON, merge updates, and write back
    for lang in lang_codes:
        json_path = os.path.join(args.dir, f"{lang}.json")
        
        # Load existing data to preserve keys that might not be in the CSV
        existing_data = {}
        if os.path.exists(json_path):
            try:
                with open(json_path, "r", encoding="utf-8") as f:
                    existing_data = json.load(f)
            except Exception as e:
                print(f"Warning: Failed to parse existing JSON at '{json_path}'. Starting fresh. Error: {e}")
        
        # Merge CSV updates into existing data
        for key, val in updates[lang].items():
            existing_data[key] = val
            
        # Write back to JSON
        try:
            with open(json_path, "w", encoding="utf-8") as f:
                json.dump(existing_data, f, indent=2, ensure_ascii=False)
            print(f"Successfully updated '{json_path}' with changes from CSV.")
        except Exception as e:
            print(f"Error: Failed to write to '{json_path}'. Error: {e}")

if __name__ == "__main__":
    main()
