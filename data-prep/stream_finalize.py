import argparse
import csv
import re
from pathlib import Path


# =============================================================================
# CONFIGURATION
# =============================================================================

DEFAULT_OCCURRENCE_FILENAME = "occurrence_formatted_filtered.csv"
DEFAULT_COLLECTORS_FILENAME = "occurrence_unique_recorded_by.csv"
OUTPUT_FILENAME = "occurrence_final.csv"
COUNTRY_CODES_FILENAME = "countryCodes.csv"

MAX_FIELD_SIZE = 10_000_000
PROGRESS_EVERY = 10_000


# =============================================================================
# LOOKUP LOADERS
# =============================================================================


def load_country_lookup(country_codes_file: Path):
    """
    Load countryCodes.csv from the same directory as this script.

    Expected columns:
        code,country
    """
    if not country_codes_file.exists():
        raise FileNotFoundError(
            f"Could not find country code file:\n{country_codes_file}"
        )

    country_lookup = {}

    with open(
        country_codes_file,
        "r",
        encoding="utf-8-sig",
        errors="replace",
        newline="",
    ) as infile:
        reader = csv.DictReader(infile)

        if reader.fieldnames is None:
            raise ValueError("countryCodes.csv is empty.")

        required_fields = {"code", "country"}
        missing_fields = required_fields - set(reader.fieldnames)

        if missing_fields:
            raise ValueError(
                "countryCodes.csv is missing required field(s): "
                + ", ".join(sorted(missing_fields))
            )

        for row_number, row in enumerate(reader, start=2):
            code = (row["code"] or "").strip().upper()
            country = (row["country"] or "").strip()

            if not code:
                continue

            if (
                code in country_lookup
                and country_lookup[code] != country
            ):
                raise ValueError(
                    "Conflicting country entries in countryCodes.csv "
                    f"for code '{code}' at row {row_number}."
                )

            country_lookup[code] = country

    return country_lookup


def load_collector_lookup(collectors_file: Path):
    """
    Load the manually reviewed collector-name file.

    Expected columns:
        recordedBy,primary_collector_lastname

    The recordedBy value is used as an exact, trimmed lookup key.
    """
    if not collectors_file.exists():
        raise FileNotFoundError(
            f"Collector names file does not exist:\n{collectors_file}"
        )

    collector_lookup = {}

    with open(
        collectors_file,
        "r",
        encoding="utf-8-sig",
        errors="replace",
        newline="",
    ) as infile:
        reader = csv.DictReader(infile)

        if reader.fieldnames is None:
            raise ValueError("The collector names file is empty.")

        required_fields = {
            "recordedBy",
            "primary_collector_lastname",
        }
        missing_fields = required_fields - set(reader.fieldnames)

        if missing_fields:
            raise ValueError(
                "Collector names file is missing required field(s): "
                + ", ".join(sorted(missing_fields))
            )

        for row_number, row in enumerate(reader, start=2):
            recorded_by = (row["recordedBy"] or "").strip()
            lastname = (
                row["primary_collector_lastname"] or ""
            ).strip().upper()

            if not recorded_by:
                continue

            if (
                recorded_by in collector_lookup
                and collector_lookup[recorded_by] != lastname
            ):
                raise ValueError(
                    "Conflicting collector entries for "
                    f"recordedBy='{recorded_by}' at row {row_number}."
                )

            collector_lookup[recorded_by] = lastname

    return collector_lookup


# =============================================================================
# VALUE TRANSFORMATIONS
# =============================================================================


def clean_record_number(value):
    """
    Remove alphabetic prefixes and suffixes from a collector number.

    Examples:
        PRE1234A      -> 1234
        AB 1990/1234C -> 1990/1234
        1234          -> 1234
    """
    value = (value or "").strip()

    if not value:
        return ""

    value = re.sub(r"^[A-Za-z]+[\s._-]*", "", value)
    value = re.sub(r"[\s._-]*[A-Za-z]+$", "", value)

    return re.sub(r"\s*/\s*", "/", value).strip()


def extract_record_number_from_recorded_by(value):
    """
    Identify a trailing collector number embedded in recordedBy.

    Accepted examples:
        Smith 1234       -> ("Smith", "1234")
        Smith 1990/1234  -> ("Smith", "1990/1234")

    A number at the start of the value is ignored. The collector-name portion
    must not contain another number.
    """
    value = (value or "").strip()

    if not value:
        return value, ""

    match = re.match(
        r"^(?P<name>\D*?\S)\s+(?P<number>\d+(?:\s*/\s*\d+)?)\s*$",
        value,
    )

    if not match:
        return value, ""

    cleaned_name = match.group("name").strip()
    record_number = re.sub(
        r"\s*/\s*",
        "/",
        match.group("number"),
    )

    return cleaned_name, record_number


def normalize_date(value):
    """
    Normalize an ISO-like date and derive its known precision.

    - Removes any time component following T.
    - For dates before 1925 where day is 01, omits the day.
    - If that date is also in January, omits the month.

    Returns:
        normalized_date,
        components dictionary,
        suppress_month,
        suppress_day
    """
    value = (value or "").strip()

    if not value:
        return "", None, False, False

    date_part = value.split("T", 1)[0].strip()

    match = re.fullmatch(
        r"(?P<year>\d{4})"
        r"(?:-(?P<month>\d{1,2})"
        r"(?:-(?P<day>\d{1,2}))?)?",
        date_part,
    )

    if not match:
        return date_part, None, False, False

    year = match.group("year")
    month = match.group("month")
    day = match.group("day")

    month_number = int(month) if month else None
    day_number = int(day) if day else None

    suppress_month = False
    suppress_day = False

    if (
        month_number is not None
        and day_number == 1
        and int(year) < 1925
    ):
        suppress_day = True
        day_number = None

        if month_number == 1:
            suppress_month = True
            month_number = None

    components = {
        "year": year,
        "month": str(month_number) if month_number is not None else "",
        "day": str(day_number) if day_number is not None else "",
    }

    normalized_date = year

    if month_number is not None:
        normalized_date += f"-{month_number:02d}"

    if day_number is not None:
        normalized_date += f"-{day_number:02d}"

    return (
        normalized_date,
        components,
        suppress_month,
        suppress_day,
    )


def apply_date_values(
    row,
    field_indexes,
    date_field,
    year_field,
    month_field,
    day_field,
):
    """
    Normalize a date field and populate associated year/month/day fields.

    Existing components are retained unless a historical first-of-month rule
    deliberately removes their precision.
    """
    date_index = field_indexes[date_field]
    year_index = field_indexes[year_field]
    month_index = field_indexes[month_field]
    day_index = field_indexes[day_field]

    (
        normalized_date,
        components,
        suppress_month,
        suppress_day,
    ) = normalize_date(row[date_index])

    row[date_index] = normalized_date

    if components is None:
        return

    if not row[year_index].strip():
        row[year_index] = components["year"]

    if suppress_month:
        row[month_index] = ""

    elif components["month"] and not row[month_index].strip():
        row[month_index] = components["month"]

    if suppress_day:
        row[day_index] = ""

    elif components["day"] and not row[day_index].strip():
        row[day_index] = components["day"]


# =============================================================================
# MAIN
# =============================================================================


def enrich_occurrence_file(
    input_directory: Path,
    occurrence_filename: str,
    collectors_filename: str,
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
    collectors_file = input_directory / collectors_filename
    output_file = input_directory / OUTPUT_FILENAME

    script_directory = Path(__file__).resolve().parent
    country_codes_file = script_directory / COUNTRY_CODES_FILENAME

    if not occurrence_file.exists():
        raise FileNotFoundError(
            f"Occurrence file does not exist:\n{occurrence_file}"
        )

    csv.field_size_limit(MAX_FIELD_SIZE)

    country_lookup = load_country_lookup(country_codes_file)
    collector_lookup = load_collector_lookup(collectors_file)

    print(f"Occurrence file: {occurrence_file}")
    print(f"Collectors file: {collectors_file}")
    print(f"Country codes:   {country_codes_file}")
    print(f"Output file:     {output_file}")
    print()
    print(f"Collector-name lookup entries: {len(collector_lookup):,}")
    print(f"Country-code lookup entries:   {len(country_lookup):,}")
    print()

    records_processed = 0
    collector_matches = 0
    country_matches = 0
    record_numbers_cleaned = 0
    record_numbers_extracted = 0

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

        required_fields = {
            "recordedBy",
            "countryCode",
            "recordNumber",
            "eventDate",
            "dateIdentified",
            "decimalLatitude",
            "decimalLongitude",
        }

        field_indexes = {
            field_name: index
            for index, field_name in enumerate(header)
        }

        missing_fields = required_fields - set(field_indexes)

        if missing_fields:
            raise ValueError(
                "Occurrence file is missing required field(s): "
                + ", ".join(sorted(missing_fields))
            )

        output_header = list(header)

        fields_to_add = [
            "searchRecordedBy",
            "country",
            "verbatimCoordinates",
            "year",
            "month",
            "day",
            "yearIdentified",
            "monthIdentified",
            "dayIdentified",
        ]

        for field_name in fields_to_add:
            if field_name not in field_indexes:
                field_indexes[field_name] = len(output_header)
                output_header.append(field_name)

        writer = csv.writer(
            outfile,
            quoting=csv.QUOTE_MINIMAL,
            lineterminator="\n",
        )
        writer.writerow(output_header)

        recorded_by_index = field_indexes["recordedBy"]
        search_recorded_by_index = field_indexes["searchRecordedBy"]
        country_code_index = field_indexes["countryCode"]
        country_index = field_indexes["country"]
        record_number_index = field_indexes["recordNumber"]
        decimal_latitude_index = field_indexes["decimalLatitude"]
        decimal_longitude_index = field_indexes["decimalLongitude"]
        verbatim_coordinates_index = field_indexes["verbatimCoordinates"]

        for row in reader:
            records_processed += 1

            if records_processed % PROGRESS_EVERY == 0:
                print(
                    f"\rProcessed {records_processed:,} | "
                    f"collector matches {collector_matches:,} | "
                    f"country matches {country_matches:,} | "
                    f"record numbers extracted {record_numbers_extracted:,}    ",
                    end="",
                    flush=True,
                )

            # Add cells for any newly appended output columns.
            row.extend([""] * (len(output_header) - len(row)))

            # -------------------------------------------------------------
            # Populate searchRecordedBy using the original recordedBy value.
            # -------------------------------------------------------------
            original_recorded_by = row[recorded_by_index].strip()

            if original_recorded_by in collector_lookup:
                row[search_recorded_by_index] = (
                    collector_lookup[original_recorded_by]
                    .strip()
                    .upper()
                )
                collector_matches += 1
            else:
                row[search_recorded_by_index] = ""

            # -------------------------------------------------------------
            # Populate country using countryCode.
            # -------------------------------------------------------------
            country_code = row[country_code_index].strip().upper()
            country = country_lookup.get(country_code, "")

            row[country_index] = country

            if country:
                country_matches += 1

            # -------------------------------------------------------------
            # Clean an existing recordNumber.
            # -------------------------------------------------------------
            original_record_number = row[record_number_index]
            cleaned_record_number = clean_record_number(
                original_record_number
            )

            row[record_number_index] = cleaned_record_number

            if cleaned_record_number != original_record_number.strip():
                record_numbers_cleaned += 1

            # -------------------------------------------------------------
            # When recordNumber is blank, extract a trailing number from
            # recordedBy and remove it from recordedBy.
            # -------------------------------------------------------------
            if not cleaned_record_number:
                (
                    cleaned_recorded_by,
                    extracted_record_number,
                ) = extract_record_number_from_recorded_by(
                    row[recorded_by_index]
                )

                if extracted_record_number:
                    row[recorded_by_index] = cleaned_recorded_by
                    row[record_number_index] = extracted_record_number
                    record_numbers_extracted += 1

            # -------------------------------------------------------------
            # Normalize eventDate and populate year/month/day.
            # -------------------------------------------------------------
            apply_date_values(
                row=row,
                field_indexes=field_indexes,
                date_field="eventDate",
                year_field="year",
                month_field="month",
                day_field="day",
            )

            # -------------------------------------------------------------
            # Normalize dateIdentified and populate its components.
            # -------------------------------------------------------------
            apply_date_values(
                row=row,
                field_indexes=field_indexes,
                date_field="dateIdentified",
                year_field="yearIdentified",
                month_field="monthIdentified",
                day_field="dayIdentified",
            )

            # -------------------------------------------------------------
            # Populate verbatimCoordinates from verbatim latitude/longitude.
            # Fall back to decimal latitude/longitude where either verbatim
            # coordinate is missing.
            # -------------------------------------------------------------
            decimal_latitude = row[decimal_latitude_index].strip()
            decimal_longitude = row[decimal_longitude_index].strip()

            if decimal_latitude and decimal_longitude:
                row[verbatim_coordinates_index] = (
                    f"{decimal_latitude}, {decimal_longitude}"
                )
            else:
                row[verbatim_coordinates_index] = ""

            writer.writerow(row)

    print()
    print()
    print("=" * 70)
    print("Final enrichment pass complete")
    print("=" * 70)
    print(f"Records processed:              {records_processed:,}")
    print(f"Collector-name matches:         {collector_matches:,}")
    print(f"Country-code matches:           {country_matches:,}")
    print(f"Existing record numbers cleaned: {record_numbers_cleaned:,}")
    print(f"Record numbers extracted:       {record_numbers_extracted:,}")
    print()
    print(f"Output file:\n{output_file.resolve()}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description=(
            "Enrich a formatted, filtered occurrence file using reviewed "
            "collector names and country codes."
        )
    )

    parser.add_argument(
        "input_directory",
        type=Path,
        help=(
            "Directory containing the occurrence and collector-name CSV files."
        ),
    )

    parser.add_argument(
        "occurrence_filename",
        nargs="?",
        default=DEFAULT_OCCURRENCE_FILENAME,
        help=(
            "Occurrence CSV filename within the directory "
            f"(default: {DEFAULT_OCCURRENCE_FILENAME})."
        ),
    )

    parser.add_argument(
        "collectors_filename",
        nargs="?",
        default=DEFAULT_COLLECTORS_FILENAME,
        help=(
            "Collector-name CSV filename within the directory "
            f"(default: {DEFAULT_COLLECTORS_FILENAME})."
        ),
    )

    args = parser.parse_args()

    enrich_occurrence_file(
        input_directory=args.input_directory,
        occurrence_filename=args.occurrence_filename,
        collectors_filename=args.collectors_filename,
    )
