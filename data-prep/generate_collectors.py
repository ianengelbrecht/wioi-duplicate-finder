import argparse
import csv
import re
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


def normalize_text(value):
    """
    Uppercase text, replace selected accented characters, collapse repeated
    whitespace, and remove punctuation while preserving single spaces and
    hyphens in surnames such as Van Wyk and Pole-Evans.
    """
    if not value:
        return ""

    char_map = {
        "à": "A", "á": "A", "â": "A", "ã": "A", "ä": "A", "å": "A",
        "À": "A", "Á": "A", "Â": "A", "Ã": "A", "Ä": "A", "Å": "A",
        "è": "E", "é": "E", "ê": "E", "ë": "E",
        "È": "E", "É": "E", "Ê": "E", "Ë": "E",
        "ì": "I", "í": "I", "î": "I", "ï": "I",
        "Ì": "I", "Í": "I", "Î": "I", "Ï": "I",
        "ò": "O", "ó": "O", "ô": "O", "õ": "O", "ö": "O", "ø": "O",
        "Ò": "O", "Ó": "O", "Ô": "O", "Õ": "O", "Ö": "O", "Ø": "O",
        "ù": "U", "ú": "U", "û": "U", "ü": "U",
        "Ù": "U", "Ú": "U", "Û": "U", "Ü": "U",
        "ñ": "N", "Ñ": "N",
        "ç": "C", "Ç": "C",
        "ý": "Y", "ÿ": "Y", "Ý": "Y", "Ÿ": "Y",
        "–": "-",
        "—": "-",
    }

    # Convert any run of whitespace, including tabs, to one ordinary space.
    value = re.sub(r"\s+", " ", value).strip()

    mapped = [
        char_map.get(character, character.upper())
        for character in value
    ]

    result = "".join(
        character
        for character in mapped
        if character.isalnum() or character in {" ", "-"}
    )

    # A punctuation removal could leave adjacent spaces behind.
    return re.sub(r" +", " ", result).strip()

def is_no_collector_value(value):
    """
    Identify standard placeholders indicating that no collector is recorded.
    """
    normalized = normalize_text(value)

    no_collector_values = {
        "SCOLL",
        "SINECOLL",
        "SINECOLLECTORE",
        "NOCOLLECTOR",
        "COLLECTORUNKNOWN",
        "UNKNOWNCOLLECTOR",
        "UNKNOWN",
        "ANONYMOUS",
        "ANON",
        "NOTRECORDED",
        "NOTKNOWN",
        "NONE",
        "NA",
        "ND",
    }

    return normalized in no_collector_values


def is_initials(value):
    """Return True when a value consists entirely of initials."""
    value = value.strip()
    if not value:
        return False
    tokens = [
        token
        for token in re.split(r"[ .]", value)
        if token
    ]

    if not tokens:
        return False

    return all(
        len(token) == 1 or (len(token) <= 3 and token.isupper())
        for token in tokens
    )


def split_by_other_delimiters(part):
    part = part.strip()
    if not part:
        return []

    if "|" in part:
        sub_parts = part.split("|")
        res = []
        for p in sub_parts:
            res.extend(split_by_other_delimiters(p))
        return res
    elif ";" in part:
        sub_parts = part.split(";")
        res = []
        for p in sub_parts:
            res.extend(split_by_other_delimiters(p))
        return res
    elif "," in part:
        comma_count = part.count(",")
        if comma_count == 1:
            possible_parts = part.split(",")
            part_after = possible_parts[1].strip()
            if is_initials(part_after):
                return [part]
            else:
                res = []
                for p in possible_parts:
                    res.extend(split_by_other_delimiters(p))
                return res
        else:
            possible_parts = part.split(",")
            res = []
            for p in possible_parts:
                res.extend(split_by_other_delimiters(p))
            return res
    else:
        return [part]


def split_names(value):
    """
    Split a raw collector string into individual collector names.
    """
    value = value.strip()
    if not value:
        return []

    # First split by & and case-insensitive word and
    parts = re.split(r"&|(?i)\band\b", value)

    final_collectors = []
    for part in parts:
        part = part.strip()
        if not part:
            continue
        sub_parts = split_by_other_delimiters(part)
        final_collectors.extend(sub_parts)

    return [part.strip() for part in final_collectors if part.strip()]

def strip_attached_initials(word):
    """
    Remove initials directly attached to the beginning of a surname.

    Examples:
        E.R.Harrison -> Harrison
        A.B.C.Smith  -> Smith
        E.R.Van      -> Van
    """
    return re.sub(
        r"^(?:[^\W\d_]\.)+(?=[^\W\d_])",
        "",
        word,
    )

def extract_lastname(name):
    """
    Extract the primary collector surname and return it uppercase with
    accents normalised, repeated whitespace collapsed, and single spaces
    and hyphens preserved.
    """
    name = name.strip()

    if not name or is_no_collector_value(name):
        return ""

    # Handles forms such as "Smith, J."
    if "," in name:
        surname = name.split(",")[0].strip()
        surname = strip_attached_initials(surname)
        return normalize_text(surname)

    words = name.split()

    if not words:
        return ""

    # Handles forms such as "E.R.Harrison" and "E.R.Van Wyk".
    words[0] = strip_attached_initials(words[0])

    title_prefixes = {
        "dr",
        "prof",
        "mr",
        "mrs",
        "ms",
        "sir",
        "lady",
    }

    cleaned_words = [
        word
        for word in words
        if word.lower().rstrip(".") not in title_prefixes
    ]

    if not cleaned_words:
        return ""

    suffixes = {
        "jr",
        "sr",
        "ii",
        "iii",
        "iv",
        "v",
        "fils",
        "jnr",
        "snr",
    }

    while (
        len(cleaned_words) > 1
        and cleaned_words[-1].lower().rstrip(".") in suffixes
    ):
        cleaned_words.pop()

    # Strip initials from the end (right to left).
    while len(cleaned_words) > 1 and is_initials(cleaned_words[-1]):
        cleaned_words.pop()

    surname_prefixes = {
        "de",
        "van",
        "von",
        "der",
        "den",
        "du",
        "del",
        "di",
        "da",
        "le",
        "la",
        "lo",
        "dalla",
        "delle",
        "della",
        "ter",
        "ten",
        "van-der",
        "d'",
        "l'",
    }

    for index, word in enumerate(cleaned_words):
        word_lower = word.lower().rstrip(".")

        if (
            word_lower in surname_prefixes
            or word_lower.startswith("d'")
            or word_lower.startswith("l'")
        ):
            return normalize_text(" ".join(cleaned_words[index:]))

    return normalize_text(cleaned_words[-1])


# =============================================================================
# MAIN
# =============================================================================


def extract_unique_recorded_by(input_directory: Path):
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

        field_indexes = {
            field_name: index
            for index, field_name in enumerate(header)
        }

        if "recordedBy" not in field_indexes:
            raise ValueError(
                "Input file is missing the required 'recordedBy' field."
            )

        recorded_by_index = field_indexes["recordedBy"]

        for row in reader:
            records_processed += 1

            if records_processed % PROGRESS_EVERY == 0:
                print(
                    f"\rProcessed {records_processed:,} | "
                    f"unique recordedBy values "
                    f"{len(recorded_by_counts):,}    ",
                    end="",
                    flush=True,
                )

            recorded_by_value = row[recorded_by_index].strip()

            if recorded_by_value:
                recorded_by_counts[recorded_by_value] += 1

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
                "primary_collector_lastname",
                "record_count",
            ],
        )

        writer.writeheader()

        records_to_write = []

        for recorded_by_value, count in recorded_by_counts.items():
            collectors = split_names(recorded_by_value)
            primary_collector = collectors[0] if collectors else ""

            records_to_write.append({
                "recordedBy": recorded_by_value,
                "primary_collector_lastname": extract_lastname(
                    primary_collector
                ),
                "record_count": count,
            })

        records_to_write.sort(
            key=lambda record: (
                -record["record_count"],
                record["recordedBy"].casefold(),
            )
        )

        writer.writerows(records_to_write)

    print()
    print()
    print("=" * 70)
    print("Unique recordedBy summary complete")
    print("=" * 70)
    print(f"Records processed:        {records_processed:,}")
    print(f"Unique recordedBy values: {len(recorded_by_counts):,}")
    print()
    print(f"Output file:\n{output_file.resolve()}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description=(
            "Create a summary of unique recordedBy values and their "
            "primary collector surnames."
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
