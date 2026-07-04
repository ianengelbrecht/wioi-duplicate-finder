# extract all reference records in the database currently to a new csv file
# might be useful if you want to share your full reference dataset with someone else as it currently stands

import argparse
import csv
import sqlite3
from pathlib import Path


BATCH_SIZE = 10_000


def export_table_to_csv(
    database_file: Path,
    table_name: str,
    output_file: Path,
):
    if not database_file.exists():
        raise FileNotFoundError(
            f"Database file does not exist:\n{database_file}"
        )

    with sqlite3.connect(database_file) as connection:
        cursor = connection.cursor()

        # Confirm that the table exists.
        table_check = cursor.execute(
            """
            SELECT name
            FROM sqlite_master
            WHERE type = 'table'
              AND name = ?
            """,
            (table_name,),
        ).fetchone()

        if table_check is None:
            raise ValueError(
                f"Table '{table_name}' does not exist in:\n{database_file}"
            )

        cursor.execute(f'SELECT * FROM "{table_name}"')

        column_names = [
            description[0]
            for description in cursor.description
        ]

        records_written = 0

        with open(
            output_file,
            "w",
            encoding="utf-8",
            newline="",
        ) as outfile:
            writer = csv.writer(
                outfile,
                quoting=csv.QUOTE_MINIMAL,
                lineterminator="\n",
            )

            writer.writerow(column_names)

            while True:
                rows = cursor.fetchmany(BATCH_SIZE)

                if not rows:
                    break

                writer.writerows(rows)
                records_written += len(rows)

                print(
                    f"\rExported {records_written:,} records",
                    end="",
                    flush=True,
                )

    print()
    print()
    print("=" * 70)
    print("SQLite export complete")
    print("=" * 70)
    print(f"Database: {database_file.resolve()}")
    print(f"Table:    {table_name}")
    print(f"Records:  {records_written:,}")
    print(f"Output:   {output_file.resolve()}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Export every record from a SQLite table to a CSV file."
    )

    parser.add_argument(
        "database_file",
        type=Path,
        help="Path to the SQLite database file.",
    )

    parser.add_argument(
        "table_name",
        help="Name of the SQLite table to export.",
    )

    parser.add_argument(
        "output_file",
        type=Path,
        help="Path for the exported CSV file.",
    )

    args = parser.parse_args()

    export_table_to_csv(
        database_file=args.database_file,
        table_name=args.table_name,
        output_file=args.output_file,
    )