import argparse
import csv
import json
from collections import Counter
from pathlib import Path


# =============================================================================
# CONFIGURATION
# =============================================================================

MAX_FIELD_SIZE = 10_000_000
PROGRESS_EVERY = 10_000


# =============================================================================
# HELPERS
# =============================================================================


# we need this because we have user input
def clean(value):
    """Return a trimmed string, preserving blanks as empty strings."""
    return (value or "").strip()


# =============================================================================
# LOAD CODE RULES
# =============================================================================


def load_code_rules(codes_file: Path):
    """
    Read the reviewed institution/collection code file.

    Every row retained in this file represents a code combination to keep.

    The corrected column, where non-blank, replaces collectionCode in matching
    occurrence records.
    """
    required_fields = {
        "institutionCode",
        "collectionCode",
        "corrected",
    }

    code_rules = {}
    duplicate_rules = []
    records_read = 0

    with open(
        codes_file,
        "r",
        encoding="utf-8-sig",
        errors="replace",
        newline="",
    ) as infile:
        reader = csv.DictReader(infile)

        if reader.fieldnames is None:
            raise ValueError("The codes file is empty.")

        missing_fields = required_fields - set(reader.fieldnames)

        if missing_fields:
            raise ValueError(
                "Codes file is missing required field(s): "
                + ", ".join(sorted(missing_fields))
            )

        for row_number, row in enumerate(reader, start=2):
            records_read += 1

            institution_code = clean(row["institutionCode"])
            collection_code = clean(row["collectionCode"])
            corrected_collection_code = clean(row["corrected"])

            key = (institution_code, collection_code)

            if key in code_rules:
                duplicate_rules.append({
                    "row_number": row_number,
                    "institutionCode": institution_code,
                    "collectionCode": collection_code,
                    "corrected": corrected_collection_code,
                })

                # Stop if duplicate rows give conflicting instructions.
                if code_rules[key] != corrected_collection_code:
                    raise ValueError(
                        "Conflicting duplicate entries in the codes file for "
                        f"institutionCode='{institution_code}' and "
                        f"collectionCode='{collection_code}'."
                    )

                continue

            code_rules[key] = corrected_collection_code

    return code_rules, records_read, duplicate_rules


# =============================================================================
# MAIN
# =============================================================================


def filter_and_correct_occurrence_file(
    input_directory: Path,
    occurrence_filename: str,
    codes_filename: str,
):
    if not input_directory.exists():
        raise FileNotFoundError(
            f"Directory does not exist:\n{input_directory}"
        )

    if not input_directory.is_dir():
        raise NotADirectoryError(
            f"Path is not a directory:\n{input_directory}"
        )

    occurrence_file = input_directory / occurrence_filename
    codes_file = input_directory / codes_filename
    output_file = input_directory / "occurrence_formatted_filtered.csv"
    report_file = input_directory / "occurrence_formatted_filtering_report.json"

    if not occurrence_file.exists():
        raise FileNotFoundError(
            f"Occurrence file does not exist:\n{occurrence_file}"
        )

    if not codes_file.exists():
        raise FileNotFoundError(
            f"Codes file does not exist:\n{codes_file}"
        )

    csv.field_size_limit(MAX_FIELD_SIZE)

    code_rules, code_rows_read, duplicate_rules = load_code_rules(codes_file)

    if not code_rules:
        raise ValueError(
            "No retained code combinations were found in the codes file."
        )

    records_processed = 0
    records_written = 0
    records_filtered_out = 0
    records_corrected = 0
    bad_width_records = 0

    matched_code_combinations = Counter()
    unmatched_code_combinations = Counter()

    print(f"Occurrence file: {occurrence_file}")
    print(f"Codes file:      {codes_file}")
    print(f"Output file:     {output_file}")
    print()
    print(f"Retained code combinations: {len(code_rules):,}")
    print()

    with open(
        occurrence_file,
        "r",
        encoding="utf-8-sig",
        errors="replace",
        newline="",
    ) as infile, open(
        output_file,
        "w",
        encoding="utf-8",
        newline="",
    ) as outfile:

        reader = csv.reader(
            infile,
            delimiter=",",
            quotechar='"',
            strict=False,
        )

        try:
            header = next(reader)
        except StopIteration:
            raise ValueError("The occurrence file is empty.")

        expected_columns = len(header)

        field_indexes = {
            field_name: index
            for index, field_name in enumerate(header)
        }

        required_fields = {
            "institutionCode",
            "collectionCode",
        }

        missing_fields = required_fields - set(field_indexes)

        if missing_fields:
            raise ValueError(
                "Occurrence file is missing required field(s): "
                + ", ".join(sorted(missing_fields))
            )

        institution_index = field_indexes["institutionCode"]
        collection_index = field_indexes["collectionCode"]

        writer = csv.writer(
            outfile,
            quoting=csv.QUOTE_MINIMAL,
            lineterminator="\n",
        )
        writer.writerow(header)

        for row in reader:
            records_processed += 1

            if records_processed % PROGRESS_EVERY == 0:
                print(
                    f"\rProcessed {records_processed:,} | "
                    f"written {records_written:,} | "
                    f"filtered out {records_filtered_out:,} | "
                    f"corrected {records_corrected:,}    ",
                    end="",
                    flush=True,
                )

            if len(row) != expected_columns:
                bad_width_records += 1
                continue

            institution_code = row[institution_index]
            collection_code = row[collection_index]
            code_key = (institution_code, collection_code)

            corrected_collection_code = code_rules.get(code_key)

            # A missing key means the user removed this combination from the
            # reviewed codes file, so exclude its occurrence records.
            if code_key not in code_rules:
                records_filtered_out += 1
                unmatched_code_combinations[code_key] += 1
                continue

            matched_code_combinations[code_key] += 1

            # Replace collectionCode only where the corrected column is filled.
            if corrected_collection_code:
                row[collection_index] = corrected_collection_code
                records_corrected += 1

            writer.writerow(row)
            records_written += 1

    unused_code_rules = sorted(
        set(code_rules) - set(matched_code_combinations),
        key=lambda item: (item[0].lower(), item[1].lower()),
    )

    report = {
        "occurrence_file": str(occurrence_file.resolve()),
        "codes_file": str(codes_file.resolve()),
        "output_file": str(output_file.resolve()),
        "code_rows_read": code_rows_read,
        "retained_code_combinations": len(code_rules),
        "duplicate_code_rows_ignored": duplicate_rules,
        "records_processed": records_processed,
        "records_written": records_written,
        "records_filtered_out": records_filtered_out,
        "records_with_collection_code_corrected": records_corrected,
        "malformed_width_rows_skipped": bad_width_records,
        "code_combinations_in_codes_file_not_found_in_occurrence_file": [
            {
                "institutionCode": institution_code,
                "collectionCode": collection_code,
                "corrected": code_rules[(institution_code, collection_code)],
            }
            for institution_code, collection_code in unused_code_rules
        ],
        "most_common_filtered_out_combinations": [
            {
                "institutionCode": institution_code,
                "collectionCode": collection_code,
                "record_count": count,
            }
            for (institution_code, collection_code), count in (
                unmatched_code_combinations.most_common(50)
            )
        ],
    }

    with open(
        report_file,
        "w",
        encoding="utf-8",
    ) as report_outfile:
        json.dump(
            report,
            report_outfile,
            indent=2,
            ensure_ascii=False,
        )

    print()
    print()
    print("=" * 70)
    print("Filtering and correction pass complete")
    print("=" * 70)
    print(f"Records processed:              {records_processed:,}")
    print(f"Written to filtered CSV:        {records_written:,}")
    print(f"Filtered out:                   {records_filtered_out:,}")
    print(f"Collection codes corrected:     {records_corrected:,}")
    print(f"Malformed-width rows skipped:   {bad_width_records:,}")
    print()
    print(f"Filtered CSV:\n{output_file.resolve()}")
    print(f"Filtering report:\n{report_file.resolve()}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description=(
            "Filter occurrence records using a reviewed institution/collection "
            "code file, and apply collectionCode corrections."
        )
    )

    parser.add_argument(
        "input_directory",
        type=Path,
        help="Directory containing the occurrence and codes CSV files.",
    )

    parser.add_argument(
        "occurrence_filename",
        nargs="?",
        default="occurrence_formatted.csv",
        help=(
           "Name of the formatted occurrence CSV file "
            "(default: occurrence_formatted.csv)."
        ),
    )

    parser.add_argument(
        "codes_filename",
        nargs="?",
        default="occurrence_formatted_inst_coll_codes.csv",
        help=(
            "Name of the reviewed institution/collection codes CSV file "
            "(default: occurrence_formatted_inst_coll_codes.csv)."
        ),
    )

    args = parser.parse_args()

    filter_and_correct_occurrence_file(
        args.input_directory,
        args.occurrence_filename,
        args.codes_filename,
    )