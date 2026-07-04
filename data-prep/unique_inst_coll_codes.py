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
# MAIN
# =============================================================================


def extract_institution_collection_combinations(
    input_directory: Path,
    input_filename: str,
):
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

    output_file = input_directory / "occurrence_formatted_inst_coll_codes.csv"

    csv.field_size_limit(MAX_FIELD_SIZE)

    records_processed = 0
    bad_width_records = 0
    combinations = Counter()

    print(f"Reading: {input_file}")
    print()

    with open(
        input_file,
        "r",
        encoding="utf-8-sig",
        errors="replace",
        newline="",
    ) as infile:

        reader = csv.reader(
            infile,
            delimiter=",",
            quotechar='"',
            strict=False,
        )

        try:
            header = next(reader)
        except StopIteration:
            raise ValueError("The input file is empty.")

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
                "Input file is missing required field(s): "
                + ", ".join(sorted(missing_fields))
            )

        institution_index = field_indexes["institutionCode"]
        collection_index = field_indexes["collectionCode"]

        for row in reader:
            records_processed += 1

            if records_processed % PROGRESS_EVERY == 0:
                print(
                    f"\rProcessed {records_processed:,} | "
                    f"unique combinations {len(combinations):,}    ",
                    end="",
                    flush=True,
                )

            if len(row) != expected_columns:
                bad_width_records += 1
                continue

            institution_code = row[institution_index]
            collection_code = row[collection_index]

            combinations[(institution_code, collection_code)] += 1

    with open(
        output_file,
        "w",
        encoding="utf-8",
        newline="",
    ) as outfile:

        writer = csv.DictWriter(
            outfile,
            fieldnames=[
                "institutionCode",
                "collectionCode",
                "record_count",
                "corrected",
            ],
        )

        writer.writeheader()

        for (institution_code, collection_code), count in sorted(
            combinations.items(),
            key=lambda item: (
                -item[1],
                item[0][0].lower(),
                item[0][1].lower(),
            ),
        ):
            writer.writerow({
                "record_count": count,
                "institutionCode": institution_code,
                "collectionCode": collection_code,
                "corrected": "",
            })

    print()
    print()
    print("=" * 70)
    print("Institution / collection code summary complete")
    print("=" * 70)
    print(f"Records processed: {records_processed:,}")
    print(f"Malformed-width rows skipped: {bad_width_records:,}")
    print(f"Unique code combinations: {len(combinations):,}")
    print()
    print(f"Output file:\n{output_file.resolve()}")

    print()
    print("Most common combinations:")

    for (institution_code, collection_code), count in combinations.most_common(20):
        print(
            f"  {count:,} | "
            f"institutionCode={institution_code or '[blank]'} | "
            f"collectionCode={collection_code or '[blank]'}"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description=(
            "Create a summary of institutionCode and collectionCode "
            "combinations in a formatted occurrence CSV file."
        )
    )

    parser.add_argument(
        "input_directory",
        type=Path,
        help="Directory containing the input CSV file.",
    )

    parser.add_argument(
        "input_filename",
        nargs="?",
        default="occurrence_formatted.csv",
        help=(
            "Name of the input CSV file within the directory "
            "(default: occurrence_formatted.csv)."
        ),
    )

    args = parser.parse_args()

    extract_institution_collection_combinations(
        args.input_directory,
        args.input_filename,
    )