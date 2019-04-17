# -*- coding: utf-8 -*-

import csv
import decimal
import io

import click

import ynab_import.ynab


class CSVDialect(csv.Dialect):
    delimiter = ";"
    quotechar = '"'
    quoting = csv.QUOTE_MINIMAL
    lineterminator = "\r\n"


def read_transactions(csv_file):
    # Remove superfluous data from ING file until the transaction log starts.
    csv_file_filtered = io.StringIO()
    for line in csv_file:
        if line.startswith('"Buchung') or line.startswith("Buchung"):
            csv_file_filtered.write(line)
            break
    for line in csv_file:
        csv_file_filtered.write(line)

    csv_file_filtered.seek(0)

    # Convert the actual data
    transactions = []
    for record in csv.DictReader(csv_file_filtered, dialect=CSVDialect):
        transaction = dict()
        transaction["Date"] = record["Buchung"].replace(".", "/")
        transaction["Payee"] = record["Auftraggeber/Empfänger"]
        transaction["Memo"] = record["Verwendungszweck"]
        amount = decimal.Decimal(record["Betrag"].replace(".", "").replace(",", "."))
        if amount < 0:
            transaction["Outflow"] = -amount
        else:
            transaction["Inflow"] = amount
        transactions.append(transaction)

    return transactions


@click.command()
@click.option("csv_file", "--csv", required=True, type=click.File(encoding="latin1"))
def main(csv_file):
    """Sync Ing-DiBa accroung with YNAB account
    """
    transactions = read_transactions(csv_file)
    ynab_import.ynab.import_transactions(transactions)
