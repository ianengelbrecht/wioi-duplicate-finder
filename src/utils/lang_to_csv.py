#!/usr/bin/env python3
import json
import csv
import os
import argparse

# for converting two translation JSON files into a single CSV table for easier editing and review

def main():
    parser = argparse.ArgumentParser(description="Convert two translation JSON files into a single CSV table.")
    parser.add_argument("json1", help="Path to the first translation JSON file (e.g., static/lang/en.json)")
    parser.add_argument("json2", help="Path to the second translation JSON file (e.g., static/lang/fr.json)")
    parser.add_argument("csv_output", help="Path to save the output CSV file (e.g., static/lang/en_fr.csv)")
    
    args = parser.parse_args()
    
    if not os.path.exists(args.json1):
        print(f"Error: File '{args.json1}' does not exist.")
        return
    if not os.path.exists(args.json2):
        print(f"Error: File '{args.json2}' does not exist.")
        return

    # Extract language codes from filenames (e.g. en.json -> en)
    lang1 = os.path.splitext(os.path.basename(args.json1))[0]
    lang2 = os.path.splitext(os.path.basename(args.json2))[0]
    
    # Load JSON data
    try:
        with open(args.json1, "r", encoding="utf-8") as f:
            data1 = json.load(f)
    except Exception as e:
        print(f"Error reading/parsing '{args.json1}': {e}")
        return

    try:
        with open(args.json2, "r", encoding="utf-8") as f:
            data2 = json.load(f)
    except Exception as e:
        print(f"Error reading/parsing '{args.json2}': {e}")
        return
        
    # Get unified key list preserving order from json1, then appending new keys from json2
    keys = list(data1.keys())
    for k in data2.keys():
        if k not in keys:
            keys.append(k)
            
    # Write CSV
    try:
        with open(args.csv_output, "w", encoding="utf-8", newline="") as f:
            writer = csv.writer(f)
            writer.writerow(["key", lang1, lang2])
            for k in keys:
                writer.writerow([k, data1.get(k, ""), data2.get(k, "")])
        print(f"Successfully converted '{args.json1}' and '{args.json2}' into '{args.csv_output}' with {len(keys)} keys.")
    except Exception as e:
        print(f"Error writing to '{args.csv_output}': {e}")

if __name__ == "__main__":
    main()
