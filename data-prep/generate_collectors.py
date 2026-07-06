import argparse
import csv
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


def clean(value):
    """Return a trimmed string, preserving missing values as empty strings."""
    return (value or "").strip()


# =============================================================================
# MAIN
# =============================================================================


def extract_unique_recorded_by(input_directory: Path):
    """Extract unique non-empty recordedBy values and their record counts."""
    if not input_directory.exists():
        raise FileNotFoundError(
            f"Directory does not exist:\n{input_directory}"
        )

    if not input_directory.is_dir():
        raise NotADirectoryError(
            f"Path is not a directory:\n{input_directory}"
        )

    input_file = input_directory / "occurrence_formatted_filtered.csv"
    output_file = input_directory / "occurrence_unique_recorded_by.csv"

    if not input_file.exists():
        raise FileNotFoundError(
            f"Required occurrence file does not exist:\n{input_file}"
        )

    csv.field_size_limit(MAX_FIELD_SIZE)

    records_processed = 0
    recorded_by_counts = Counter()

    print(f"Reading: {input_file}")
    print()

    with open(
        input_file,
        "r",
        encoding="utf-8-sig",
        errors="replace",
        newline="",
    ) as infile:
        reader = csv.DictReader(infile)

        if not reader.fieldnames:
            raise ValueError("The input file is empty.")

        if "recordedBy" not in reader.fieldnames:
            raise ValueError(
                "Input file is missing the required 'recordedBy' field."
            )

        for row in reader:
            records_processed += 1

            if records_processed % PROGRESS_EVERY == 0:
                print(
                    f"\rProcessed {records_processed:,} | "
                    f"unique recordedBy values: {len(recorded_by_counts):,}",
                    end="",
                    flush=True,
                )

            recorded_by = clean(row.get("recordedBy"))

            if recorded_by:
                recorded_by_counts[recorded_by] += 1

    with open(
        output_file,
        "w",
        encoding="utf-8",
        newline="",
    ) as outfile:
        writer = csv.DictWriter(
            outfile,
            fieldnames=[
                "recordedBy",
                "record_count",
            ],
        )

        writer.writeheader()

        for recorded_by, count in sorted(
            recorded_by_counts.items(),
            key=lambda item: (-item[1], item[0].casefold()),
        ):
            writer.writerow(
                {
                    "recordedBy": recorded_by,
                    "record_count": count,
                }
            )

    print()
    print()
    print("=" * 70)
    print("Unique recordedBy extraction complete")
    print("=" * 70)
    print(f"Records processed:        {records_processed:,}")
    print(f"Unique recordedBy values: {len(recorded_by_counts):,}")
    print()
    print(f"Output file:\n{output_file.resolve()}")
    print()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description=(
            "Extract unique non-empty recordedBy values and their counts "
            "from occurrence_formatted_filtered.csv."
        )
    )

    parser.add_argument(
        "input_directory",
        type=Path,
        help=(
            "Directory containing occurrence_formatted_filtered.csv."
        ),
    )

    args = parser.parse_args()

    extract_unique_recorded_by(args.input_directory)