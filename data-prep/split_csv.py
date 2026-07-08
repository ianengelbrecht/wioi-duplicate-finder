# split a csv file into subsets based on values in a particular column, e.g. countryCode. 
# intended to help managing datasets during the data prep workflow

import argparse
import csv
import re
from pathlib import Path

# DO NOT USE MAX_FIELD_SIZE, DESPITE WHAT AI SAYS. IF YOU NEED THIS, THE DATASET FORMATTING DID NOT WORK

PROGRESS_EVERY = 10_000


def safe_filename_value(value: str) -> str:
    """
    Convert a column value into a safe filename component.

    Examples:
        "South Africa" -> "South_Africa"
        "A/B:C"        -> "A_B_C"
        ""             -> "blank"
    """
    value = (value or "").strip()

    if not value:
        return "blank"

    # Replace characters that are invalid in Windows filenames.
    value = re.sub(r'[<>:"/\\|?*\x00-\x1f]', "_", value)

    # Collapse whitespace and repeated underscores.
    value = re.sub(r"\s+", "_", value)
    value = re.sub(r"_+", "_", value)

    # Avoid awkward leading/trailing characters.
    value = value.strip(" ._")

    return value or "blank"


def split_csv_by_column(
    input_directory: Path,
    input_filename: str,
    column_name: str,
):
    """
    Split a CSV into separate files based on unique values in one column.

    Output files are written in the same directory as the input file:

        input.csv
        input_VALUE.csv
    """
    if not input_directory.exists():
        raise FileNotFoundError(
            f"Directory does not exist:\n{input_directory}"
        )

    if not input_directory.is_dir():
        raise NotADirectoryError(
            f"Path is not a directory:\n{input_directory}"
        )

    input_file = input_directory / input_filename

    if not input_file.exists():
        raise FileNotFoundError(
            f"Input file does not exist:\n{input_file}"
        )

    if not input_file.is_file():
        raise ValueError(
            f"Input path is not a file:\n{input_file}"
        )

    output_handles = {}
    output_writers = {}
    output_paths = {}
    row_counts = {}

    try:
        with open(
            input_file,
            "r",
            encoding="utf-8-sig",
            errors="replace",
            newline="",
        ) as infile:
            reader = csv.DictReader(infile)

            if reader.fieldnames is None:
                raise ValueError("The input CSV is empty.")

            if column_name not in reader.fieldnames:
                raise ValueError(
                    f"Column '{column_name}' was not found.\n"
                    f"Available columns:\n{', '.join(reader.fieldnames)}"
                )

            input_stem = input_file.stem
            input_suffix = input_file.suffix

            records_processed = 0

            for row in reader:
                records_processed += 1

                raw_value = (row.get(column_name) or "").strip()
                filename_value = safe_filename_value(raw_value)

                # A different source value can theoretically produce the same
                # safe filename, so retain the raw value as the lookup key.
                output_key = raw_value

                if output_key not in output_writers:
                    output_file = (
                        input_directory
                        / f"{input_stem}_{filename_value}{input_suffix}"
                    )

                    # Prevent accidental overwriting where separate values
                    # resolve to the same safe filename.
                    if output_file in output_paths.values():
                        counter = 2
                        base_output_file = output_file

                        while output_file in output_paths.values():
                            output_file = (
                                input_directory
                                / (
                                    f"{base_output_file.stem}_{counter}"
                                    f"{input_suffix}"
                                )
                            )
                            counter += 1

                    outfile = open(
                        output_file,
                        "w",
                        encoding="utf-8",
                        newline="",
                    )

                    writer = csv.DictWriter(
                        outfile,
                        fieldnames=reader.fieldnames,
                        quoting=csv.QUOTE_MINIMAL,
                        lineterminator="\n",
                    )

                    writer.writeheader()

                    output_handles[output_key] = outfile
                    output_writers[output_key] = writer
                    output_paths[output_key] = output_file
                    row_counts[output_key] = 0

                output_writers[output_key].writerow(row)
                row_counts[output_key] += 1

                if records_processed % PROGRESS_EVERY == 0:
                    print(
                        f"\rProcessed {records_processed:,} records "
                        f"into {len(output_writers):,} files...",
                        end="",
                        flush=True,
                    )

    finally:
        for outfile in output_handles.values():
            outfile.close()

    print()
    print()
    print("=" * 70)
    print("CSV split complete")
    print("=" * 70)
    print(f"Input file:        {input_file}")
    print(f"Records processed: {sum(row_counts.values()):,}")
    print(f"Files created:     {len(output_paths):,}")
    print()

    for output_key in sorted(
        output_paths,
        key=lambda value: output_paths[value].name.lower(),
    ):
        print(
            f"{output_paths[output_key].name}: "
            f"{row_counts[output_key]:,} records"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description=(
            "Split a CSV file into separate CSV files based on unique values "
            "in a chosen column."
        )
    )

    parser.add_argument(
        "input_directory",
        type=Path,
        help="Directory containing the input CSV file.",
    )

    parser.add_argument(
        "input_filename",
        help="Filename of the input CSV within the input directory.",
    )

    parser.add_argument(
        "column_name",
        help=(
            "Name of the column whose unique values should define "
            "the output files."
        ),
    )

    args = parser.parse_args()

    split_csv_by_column(
        input_directory=args.input_directory,
        input_filename=args.input_filename,
        column_name=args.column_name,
    )
