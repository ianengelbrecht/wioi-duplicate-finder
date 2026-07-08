import argparse
import csv
import json
import re
from collections import Counter
from pathlib import Path


# =============================================================================
# CONFIGURATION
# =============================================================================

MAX_FIELD_SIZE = 10_000_000  # 10 MB
PROGRESS_EVERY = 10_000
SOURCE_DELIMITER = "\t"


# =============================================================================
# HELPERS
# =============================================================================


def clean(value):
    """Return a trimmed string, preserving blanks as empty strings."""
    return (value or "").strip()


def strip_line_endings(line):
    """Remove only physical line endings."""
    return line.rstrip("\n").rstrip("\r")


def make_preview(value, max_length=500):
    """Return a compact, one-line representation for diagnostic output."""
    if value is None:
        return ""

    value = str(value)
    value = value.replace("\r", "\\r").replace("\n", "\\n")

    if len(value) > max_length:
        return value[:max_length] + "... [truncated]"

    return value


def split_record_text(record_text):
    """
    Split one accumulated logical record into tab-delimited values.

    The record may contain embedded newlines inside fields.
    """
    return record_text.split(SOURCE_DELIMITER)


def flatten_embedded_newlines(value):
    """
    Replace embedded newlines inside field values with spaces.

    This makes the cleaned output much safer for downstream tools that expect
    one physical line per record.
    """
    if value is None:
        return ""

    return re.sub(r"[\r\n]+", " ", value)


def get_value(row, field_indexes, field_name):
    """Safely retrieve a named field from a row that may have too few columns."""
    index = field_indexes.get(field_name)

    if index is None or index >= len(row):
        return ""

    return row[index]


def get_output_delimiter(name):
    if name == "tab":
        return "\t"

    if name == "comma":
        return ","

    raise ValueError(f"Unsupported output delimiter: {name}")


def looks_like_record_start(line, record_start_pattern):
    """
    Return True if this physical line appears to begin a new GBIF record.

    Default pattern assumes gbifID is the first field:
    ^\\d+\\t
    """
    if not line:
        return False

    return bool(record_start_pattern.match(line))


def write_quarantine_record(
    quarantine_outfile,
    quarantine_summary_writer,
    reason,
    logical_record_number,
    physical_line_start,
    physical_line_end,
    expected_columns,
    row,
    header,
    field_indexes,
):
    actual_columns = len(row)

    missing_fields = []
    extra_values = []

    if actual_columns < expected_columns:
        missing_fields = header[actual_columns:]

    if actual_columns > expected_columns:
        extra_values = row[expected_columns:]

    quarantine_record = {
        "reason": reason,
        "logical_record_number": logical_record_number,
        "physical_line_start": physical_line_start,
        "physical_line_end": physical_line_end,
        "physical_line_count": physical_line_end - physical_line_start + 1,
        "expected_column_count": expected_columns,
        "actual_column_count": actual_columns,
        "missing_column_count": max(expected_columns - actual_columns, 0),
        "extra_column_count": max(actual_columns - expected_columns, 0),
        "missing_fields": missing_fields,
        "extra_values": extra_values,
        "row_values": row,
    }

    quarantine_outfile.write(
        json.dumps(
            quarantine_record,
            ensure_ascii=False,
        )
        + "\n"
    )

    quarantine_summary_writer.writerow(
        {
            "reason": reason,
            "logical_record_number": logical_record_number,
            "physical_line_start": physical_line_start,
            "physical_line_end": physical_line_end,
            "physical_line_count": physical_line_end - physical_line_start + 1,
            "expected_column_count": expected_columns,
            "actual_column_count": actual_columns,
            "missing_column_count": max(expected_columns - actual_columns, 0),
            "extra_column_count": max(actual_columns - expected_columns, 0),
            "gbifID": get_value(row, field_indexes, "gbifID"),
            "catalogNumber": get_value(row, field_indexes, "catalogNumber"),
            "recordedBy": get_value(row, field_indexes, "recordedBy"),
            "scientificName": get_value(row, field_indexes, "scientificName"),
            "datasetName": get_value(row, field_indexes, "datasetName"),
            "institutionCode": get_value(row, field_indexes, "institutionCode"),
            "collectionCode": get_value(row, field_indexes, "collectionCode"),
            "missing_fields": " | ".join(missing_fields),
            "extra_values_preview": " | ".join(
                make_preview(value, 250) for value in extra_values
            ),
            "row_preview": " | ".join(make_preview(value, 250) for value in row),
        }
    )


# =============================================================================
# MAIN
# =============================================================================


def stream_format_csv(
    input_directory: Path,
    output_file: Path,
    output_delimiter_name: str,
    record_start_regex: str,
    preserve_embedded_newlines: bool,
):
    input_file = input_directory / "occurrence.txt"

    output_file = input_directory / output_file
    quarantine_file = input_directory / "occurrence_quarantined_rows.jsonl"
    quarantine_summary_file = input_directory / "occurrence_quarantined_rows_summary.csv"
    report_file = input_directory / "occurrence_formatting_report.json"

    if not input_directory.exists():
        raise FileNotFoundError(f"Directory does not exist:\n{input_directory}")

    if not input_directory.is_dir():
        raise NotADirectoryError(f"Path is not a directory:\n{input_directory}")

    if not input_file.exists():
        raise FileNotFoundError(f"Could not find occurrence.txt in:\n{input_directory}")

    csv.field_size_limit(MAX_FIELD_SIZE)

    output_delimiter = get_output_delimiter(output_delimiter_name)
    record_start_pattern = re.compile(record_start_regex)

    physical_lines_read = 0
    logical_records_processed = 0
    records_written = 0
    records_quarantined = 0
    multiline_records = 0

    row_length_counts = Counter()
    quarantine_reason_counts = Counter()

    print(f"Input file:           {input_file}")
    print(f"Output file:          {output_file}")
    print(f"Input delimiter:      tab")
    print(f"Output delimiter:     {output_delimiter_name}")
    print(f"Record start regex:   {record_start_regex}")
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

        try:
            header_line = next(infile)
        except StopIteration:
            raise ValueError("The input file is empty.")

        header = strip_line_endings(header_line).split(SOURCE_DELIMITER)
        expected_columns = len(header)

        field_indexes = {
            field_name: index
            for index, field_name in enumerate(header)
        }

        required_fields = {
            "institutionCode",
            "collectionCode",
        }

        missing_required_fields = required_fields - set(field_indexes)

        if missing_required_fields:
            raise ValueError(
                "Input file is missing required field(s): "
                + ", ".join(sorted(missing_required_fields))
            )

        institution_index = field_indexes["institutionCode"]
        collection_index = field_indexes["collectionCode"]

        writer = csv.writer(
            outfile,
            delimiter=output_delimiter,
            quoting=csv.QUOTE_MINIMAL,
            lineterminator="\n",
        )
        writer.writerow(header)

        quarantine_summary_writer = csv.DictWriter(
            quarantine_summary_outfile,
            fieldnames=[
                "reason",
                "logical_record_number",
                "physical_line_start",
                "physical_line_end",
                "physical_line_count",
                "expected_column_count",
                "actual_column_count",
                "missing_column_count",
                "extra_column_count",
                "gbifID",
                "catalogNumber",
                "recordedBy",
                "scientificName",
                "datasetName",
                "institutionCode",
                "collectionCode",
                "missing_fields",
                "extra_values_preview",
                "row_preview",
            ],
        )
        quarantine_summary_writer.writeheader()

        buffer_lines = []
        buffer_start_line = None
        buffer_end_line = None

        def process_buffer():
            nonlocal logical_records_processed
            nonlocal records_written
            nonlocal records_quarantined
            nonlocal multiline_records

            if not buffer_lines:
                return

            logical_records_processed += 1

            record_text = "\n".join(buffer_lines)
            row = split_record_text(record_text)
            column_count = len(row)

            row_length_counts[column_count] += 1

            physical_line_count = buffer_end_line - buffer_start_line + 1

            if physical_line_count > 1:
                multiline_records += 1

            if column_count < expected_columns:
                reason = "too_few_columns"
            elif column_count > expected_columns:
                reason = "too_many_columns"
            else:
                reason = None

            if reason:
                quarantine_reason_counts[reason] += 1
                records_quarantined += 1

                write_quarantine_record(
                    quarantine_outfile=quarantine_outfile,
                    quarantine_summary_writer=quarantine_summary_writer,
                    reason=reason,
                    logical_record_number=logical_records_processed,
                    physical_line_start=buffer_start_line,
                    physical_line_end=buffer_end_line,
                    expected_columns=expected_columns,
                    row=row,
                    header=header,
                    field_indexes=field_indexes,
                )

                return

            if not preserve_embedded_newlines:
                row = [
                    flatten_embedded_newlines(value)
                    for value in row
                ]

            # Standardise whitespace in the two code fields before output.
            row[institution_index] = clean(row[institution_index])
            row[collection_index] = clean(row[collection_index])

            writer.writerow(row)
            records_written += 1

        for physical_line_number, raw_line in enumerate(infile, start=2):
            physical_lines_read += 1

            line = strip_line_endings(raw_line)

            is_record_start = looks_like_record_start(
                line,
                record_start_pattern,
            )

            if is_record_start:
                # A new record has started, so the previous accumulated buffer
                # is now complete and can be validated.
                if buffer_lines:
                    process_buffer()

                buffer_lines = [line]
                buffer_start_line = physical_line_number
                buffer_end_line = physical_line_number

            else:
                # This line is a continuation of the previous logical record.
                # This is how we handle embedded newlines in text fields.
                if buffer_lines:
                    buffer_lines.append(line)
                    buffer_end_line = physical_line_number
                else:
                    # Orphan continuation line before any detected gbifID.
                    # Keep it as a buffer so it will be quarantined once a
                    # real record start is encountered or at EOF.
                    buffer_lines = [line]
                    buffer_start_line = physical_line_number
                    buffer_end_line = physical_line_number

            if physical_lines_read % PROGRESS_EVERY == 0:
                print(
                    f"\rPhysical lines read {physical_lines_read:,} | "
                    f"logical records {logical_records_processed:,} | "
                    f"written {records_written:,} | "
                    f"quarantined {records_quarantined:,}    ",
                    end="",
                    flush=True,
                )

        # Process final buffered record at EOF.
        if buffer_lines:
            process_buffer()

    report = {
        "input_file": str(input_file.resolve()),
        "output_file": str(output_file.resolve()),
        "quarantine_file": str(quarantine_file.resolve()),
        "quarantine_summary_file": str(quarantine_summary_file.resolve()),
        "expected_column_count": expected_columns,
        "physical_lines_read_after_header": physical_lines_read,
        "logical_records_processed": logical_records_processed,
        "records_written_to_output_file": records_written,
        "records_quarantined": records_quarantined,
        "multiline_logical_records": multiline_records,
        "quarantine_reason_counts": dict(quarantine_reason_counts),
        "row_length_distribution": {
            str(column_count): count
            for column_count, count in sorted(row_length_counts.items())
        },
        "configuration": {
            "source_delimiter": "tab",
            "output_delimiter": output_delimiter_name,
            "record_start_regex": record_start_regex,
            "preserve_embedded_newlines": preserve_embedded_newlines,
            "max_field_size": MAX_FIELD_SIZE,
            "progress_every": PROGRESS_EVERY,
            "input_encoding": "utf-8-sig",
            "encoding_error_handling": "replace",
            "formatting_rule": (
                "Accumulate physical lines into logical records using a "
                "record-start regex, then validate each logical record against "
                "the header column count. Quarantine records with too few or "
                "too many tab-delimited values."
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
    print(f"Expected columns:              {expected_columns:,}")
    print(f"Physical lines read:           {physical_lines_read:,}")
    print(f"Logical records processed:     {logical_records_processed:,}")
    print(f"Multiline logical records:     {multiline_records:,}")
    print(f"Written to output file:        {records_written:,}")
    print(f"Quarantined:                   {records_quarantined:,}")

    if quarantine_reason_counts:
        print()
        print("Quarantine reasons:")
        for reason, count in sorted(quarantine_reason_counts.items()):
            print(f"  {reason}: {count:,}")

    print()
    print("Output files:")
    print(f"  Formatted output:    {output_file.resolve()}")
    print(f"  Quarantined rows:    {quarantine_file.resolve()}")
    print(f"  Quarantine summary:  {quarantine_summary_file.resolve()}")
    print(f"  Formatting report:   {report_file.resolve()}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description=(
            "Format occurrence.txt in a specified directory, allowing for "
            "embedded newlines inside fields, and quarantine records with "
            "too few or too many tab-delimited values."
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
        nargs="?",
        default=Path("occurrence_formatted.tsv"),
        help=(
            "Output file name. Default: occurrence_formatted.tsv. "
            "Use .csv if you also pass --output-delimiter comma."
        ),
    )

    parser.add_argument(
        "--output-delimiter",
        choices=["tab", "comma"],
        default="tab",
        help=(
            "Delimiter for the cleaned output file. Default: tab. "
            "For a true CSV file, use --output-delimiter comma."
        ),
    )

    parser.add_argument(
        "--record-start-regex",
        default=r"^\d+\t",
        help=(
            r"Regex identifying the start of a new record. "
            r"Default assumes gbifID is the first field: ^\d+\t"
        ),
    )

    parser.add_argument(
        "--preserve-embedded-newlines",
        action="store_true",
        help=(
            "Preserve embedded newlines inside field values in the cleaned output. "
            "By default, embedded newlines are replaced with spaces so the output "
            "has one physical line per record."
        ),
    )

    args = parser.parse_args()

    stream_format_csv(
        input_directory=args.input_directory,
        output_file=args.output_file,
        output_delimiter_name=args.output_delimiter,
        record_start_regex=args.record_start_regex,
        preserve_embedded_newlines=args.preserve_embedded_newlines,
    )