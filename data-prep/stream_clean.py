import argparse
import csv
import json
from collections import Counter
from pathlib import Path


# =============================================================================
# CONFIGURATION
# =============================================================================

MAX_FIELD_SIZE = 10_000_000  # 10 MB
PROGRESS_EVERY = 10_000


# =============================================================================
# HELPERS
# =============================================================================


def clean(value):
    """Return a trimmed string, preserving blanks as empty strings."""
    return (value or "").strip()


def make_preview(value, max_length=500):
    """Return a compact, one-line representation for diagnostic output."""
    if value is None:
        return ""

    value = str(value)
    value = value.replace("\r", "\\r").replace("\n", "\\n")

    if len(value) > max_length:
        return value[:max_length] + "... [truncated]"

    return value


def get_value(row, field_indexes, field_name):
    """Safely retrieve a named field from a row that may have too few columns."""
    index = field_indexes.get(field_name)

    if index is None or index >= len(row):
        return ""

    return row[index]


# =============================================================================
# MAIN
# =============================================================================


def stream_format_csv(input_directory: Path, output_file: Path):
    input_file = input_directory / "occurrence.txt"

    output_file = input_directory / output_file
    quarantine_file = input_directory / "occurrence_quarantined_rows.jsonl"
    quarantine_summary_file = (
        input_directory / "occurrence_quarantined_rows_summary.csv"
    )
    report_file = input_directory / "occurrence_formatting_report.json"

    if not input_directory.exists():
        raise FileNotFoundError(
            f"Directory does not exist:\n{input_directory}"
        )

    if not input_directory.is_dir():
        raise NotADirectoryError(
            f"Path is not a directory:\n{input_directory}"
        )

    if not input_file.exists():
        raise FileNotFoundError(
            f"Could not find occurrence.txt in:\n{input_directory}"
        )

    csv.field_size_limit(MAX_FIELD_SIZE)

    records_processed = 0
    records_written = 0
    records_quarantined = 0

    parser_error = None

    row_length_counts = Counter()
    quarantine_reason_counts = Counter()

    print(f"Input file:   {input_file}")
    print(f"Output file:  {output_file}")
    print()

    with open(
        input_file,
        "r",
        encoding="utf-8-sig",
        errors="replace",
        newline="",
    ) as infile, open(
        output_file,
        "w",
        encoding="utf-8",
        newline="",
    ) as outfile, open(
        quarantine_file,
        "w",
        encoding="utf-8",
    ) as quarantine_outfile, open(
        quarantine_summary_file,
        "w",
        encoding="utf-8",
        newline="",
    ) as quarantine_summary_outfile:

        reader = csv.reader(
            infile,
            delimiter="\t",
            quotechar=None,
            quoting=csv.QUOTE_NONE,
            strict=True,
        )

        try:
            header = next(reader)

        except StopIteration:
            raise ValueError("The input file is empty.")

        except csv.Error as exc:
            raise RuntimeError(
                f"Could not parse file header: {exc}"
            ) from exc

        expected_columns = len(header)
        header_line_end = reader.line_num

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
                "Input file is missing required field(s): "
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

        quarantine_summary_writer = csv.DictWriter(
            quarantine_summary_outfile,
            fieldnames=[
                "reason",
                "csv_record_number",
                "physical_line_start",
                "physical_line_end",
                "physical_line_count",
                "column_count",
                "gbifID",
                "catalogNumber",
                "recordedBy",
                "scientificName",
                "datasetName",
                "institutionCode",
                "collectionCode",
                "row_preview",
            ],
        )
        quarantine_summary_writer.writeheader()

        previous_line_end = header_line_end

        while True:
            try:
                row = next(reader)

            except StopIteration:
                break

            except csv.Error as exc:
                parser_error = {
                    "message": str(exc),
                    "physical_line_number": reader.line_num,
                }
                break

            records_processed += 1
            csv_record_number = records_processed + 1

            if records_processed % PROGRESS_EVERY == 0:
                print(
                    f"\rProcessed {records_processed:,} | "
                    f"written {records_written:,} | "
                    f"quarantined {records_quarantined:,}    ",
                    end="",
                    flush=True,
                )

            start_line = previous_line_end + 1
            end_line = reader.line_num
            previous_line_end = end_line

            physical_line_count = end_line - start_line + 1
            column_count = len(row)

            row_length_counts[column_count] += 1

            if column_count != expected_columns:
                reason = (
                    "too_few_columns"
                    if column_count < expected_columns
                    else "too_many_columns"
                )

                quarantine_reason_counts[reason] += 1
                records_quarantined += 1

                quarantine_record = {
                    "reason": reason,
                    "csv_record_number": csv_record_number,
                    "physical_line_start": start_line,
                    "physical_line_end": end_line,
                    "physical_line_count": physical_line_count,
                    "expected_column_count": expected_columns,
                    "actual_column_count": column_count,
                    "row_values": row,
                }

                quarantine_outfile.write(
                    json.dumps(
                        quarantine_record,
                        ensure_ascii=False,
                    )
                    + "\n"
                )

                quarantine_summary_writer.writerow({
                    "reason": reason,
                    "csv_record_number": csv_record_number,
                    "physical_line_start": start_line,
                    "physical_line_end": end_line,
                    "physical_line_count": physical_line_count,
                    "column_count": column_count,
                    "gbifID": get_value(
                        row, field_indexes, "gbifID"
                    ),
                    "catalogNumber": get_value(
                        row, field_indexes, "catalogNumber"
                    ),
                    "recordedBy": get_value(
                        row, field_indexes, "recordedBy"
                    ),
                    "scientificName": get_value(
                        row, field_indexes, "scientificName"
                    ),
                    "datasetName": get_value(
                        row, field_indexes, "datasetName"
                    ),
                    "institutionCode": get_value(
                        row, field_indexes, "institutionCode"
                    ),
                    "collectionCode": get_value(
                        row, field_indexes, "collectionCode"
                    ),
                    "row_preview": " | ".join(
                        make_preview(value, 250)
                        for value in row
                    ),
                })

                continue

            # Standardise whitespace in the two code fields before output.
            row[institution_index] = clean(row[institution_index])
            row[collection_index] = clean(row[collection_index])

            writer.writerow(row)
            records_written += 1

    report = {
        "input_file": str(input_file.resolve()),
        "output_file": str(output_file.resolve()),
        "quarantine_file": str(quarantine_file.resolve()),
        "quarantine_summary_file": str(
            quarantine_summary_file.resolve()
        ),
        "expected_column_count": expected_columns,
        "records_processed": records_processed,
        "records_written_to_formatted_file": records_written,
        "records_quarantined": records_quarantined,
        "quarantine_reason_counts": dict(quarantine_reason_counts),
        "row_length_distribution": {
            str(column_count): count
            for column_count, count in sorted(row_length_counts.items())
        },
        "parser_error": parser_error,
        "configuration": {
            "source_delimiter": "tab",
            "output_delimiter": "comma",
            "max_field_size": MAX_FIELD_SIZE,
            "progress_every": PROGRESS_EVERY,
            "input_encoding": "utf-8-sig",
            "encoding_error_handling": "replace",
            "formatting_rule": (
                "Write rows with the same number of columns as the header; "
                "quarantine structurally malformed rows; trim whitespace "
                "from institutionCode and collectionCode."
            ),
        },
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
    print("Formatting pass complete")
    print("=" * 70)
    print(f"Records processed:       {records_processed:,}")
    print(f"Written to formatted CSV: {records_written:,}")
    print(f"Quarantined:             {records_quarantined:,}")
    print()
    print("Output files:")
    print(f"  Formatted CSV:      {output_file.resolve()}")
    print(f"  Quarantined rows:   {quarantine_file.resolve()}")
    print(
        f"  Quarantine summary: {quarantine_summary_file.resolve()}"
    )
    print(f"  Formatting report:  {report_file.resolve()}")

    if parser_error:
        print()
        print("WARNING: CSV parsing stopped early.")
        print(
            f"Parser error near physical line "
            f"{parser_error['physical_line_number']:,}: "
            f"{parser_error['message']}"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description=(
            "Format occurrence.txt in a specified directory and "
            "quarantine structurally malformed rows."
        )
    )

    parser.add_argument(
        "input_directory",
        type=Path,
        help="Directory containing occurrence.txt",
    )

    parser.add_argument(
        "output_file",
        type=Path,
        default="occurrence_formatted.csv",
        help="The output file name, ending in .csv, default is occurrence_formatted.csv",
    )


    args = parser.parse_args()

    stream_format_csv(args.input_directory, args.output_file)