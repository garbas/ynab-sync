# -*- coding: utf-8 -*-

import csv
import decimal
import io

import click


class CSVDialect(csv.Dialect):
    delimiter = ";"
    quotechar = '"'
    quoting = csv.QUOTE_MINIMAL
    lineterminator = "\r\n"


def read_transactions(filename):
    ing_file = open(filename, newline="", encoding="latin1")
    # Remove superfluous data from ING file until the transaction log starts.
    ing_file_filtered = io.StringIO()
    for line in ing_file:
        if line.startswith('"Buchung') or line.startswith("Buchung"):
            ing_file_filtered.write(line)
            break
    for line in ing_file:
        ing_file_filtered.write(line)
    ing_file = ing_file_filtered
    ing_file.seek(0)

    # Convert the actual data
    transactions = []
    for record in csv.DictReader(ing_file, dialect=CSVDialect):
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
@click.option("csv_file", "--csv", type=click.File())
def main(csv_file):
    """Sync Ing-DiBa accroung with YNAB account
    """
