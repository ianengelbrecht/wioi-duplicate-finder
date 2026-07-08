import argparse
import csv
from collections import Counter
from pathlib import Path


# =============================================================================
# CONFIGURATION
# =============================================================================

PROGRESS_EVERY = 10_000


# =============================================================================
# HELPERS
# =============================================================================


def clean(value):
    """Return a trimmed string, preserving missing values as empty strings."""
    return (value or "").strip()


def resolve_output_file(input_directory: Path, input_file: Path, output_file_name: str | None):
    """
    Resolve output file path.

    If output_file_name is omitted, use:
    <input_stem>_unique_recorded_by.csv

    If output_file_name is relative, place it in input_directory.
    If output_file_name is absolute, use it directly.
    """
    if output_file_name is None:
        return input_directory / f"{input_file.stem}_unique_recorded_by.csv"

    output_path = Path(output_file_name)

    if output_path.is_absolute():
        return output_path

    return input_directory / output_path


# =============================================================================
# MAIN
# =============================================================================


def extract_unique_recorded_by(
    input_directory: Path,
    input_file_name: str,
    output_file_name: str | None = None,
):
    """Extract unique non-empty recordedBy values and their record counts."""
    if not input_directory.exists():
        raise FileNotFoundError(
            f"Directory does not exist:\n{input_directory}"
        )

    if not input_directory.is_dir():
        raise NotADirectoryError(
            f"Path is not a directory:\n{input_directory}"
        )

    input_file = input_directory / input_file_name
    output_file = resolve_output_file(
        input_directory=input_directory,
        input_file=input_file,
        output_file_name=output_file_name,
    )

    if not input_file.exists():
        raise FileNotFoundError(
            f"Input file does not exist:\n{input_file}"
        )

    if not input_file.is_file():
        raise ValueError(
            f"Input path is not a file:\n{input_file}"
        )

    records_processed = 0
    recorded_by_counts = Counter()

    print(f"Reading: {input_file.resolve()}")
    print(f"Output:  {output_file.resolve()}")
    print()

    with open(
        input_file,
        "r",
        encoding="utf-8",
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
            quoting=csv.QUOTE_ALL,
            lineterminator="\n",
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
            "from a specified occurrence CSV file in a specified directory."
        )
    )

    parser.add_argument(
        "input_directory",
        type=Path,
        help="Directory containing the input occurrence CSV.",
    )

    parser.add_argument(
        "input_file_name",
        help=(
            "Input occurrence CSV file name"
        ),
    )

    parser.add_argument(
        "output_file_name",
        nargs="?",
        default=None,
        help=(
            "Optional output CSV file name. If omitted, output will be named "
            "<input_file_stem>_unique_recorded_by.csv in the input directory."
        ),
    )

    args = parser.parse_args()

    extract_unique_recorded_by(
        input_directory=args.input_directory,
        input_file_name=args.input_file_name,
        output_file_name=args.output_file_name,
    )