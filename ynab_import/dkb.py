import click
import csv
import decimal
import io


class CSVDialect(csv.Dialect):
    delimiter = ";"
    quotechar = '"'
    quoting = csv.QUOTE_MINIMAL
    lineterminator = "\n"


def read_transactions(filename):
    dkb_file = open(filename, newline="", encoding="latin1")

    # Remove superfluous data from dkb file until the transaction log starts.
    dkb_file_filtered = io.StringIO()
    for line in dkb_file:
        if line.startswith('"Buchungstag'):
            dkb_file_filtered.write(line)
            break
    for line in dkb_file:
        dkb_file_filtered.write(line)
    dkb_file = dkb_file_filtered
    dkb_file.seek(0)

    transactions = []
    for record in csv.DictReader(dkb_file, dialect=CSVDialect):
        transaction = {}
        if not record["Betrag (EUR)"].strip():
            continue
        transaction["Date"] = record["Wertstellung"].replace(".", "/")
        transaction["Payee"] = record[u"Auftraggeber / Begünstigter"]
        transaction["Memo"] = record["Verwendungszweck"]
        amount = decimal.Decimal(
            record["Betrag (EUR)"].replace(".", "").replace(",", ".")
        )
        if amount < 0:
            transaction.Outflow = -amount
        else:
            transaction.Inflow = amount
        transactions.append(transaction)

    return transactions


@click.command()
@click.option("csv_file", "--csv", type=click.File())
def main(csv_file):
    """Sync Ing-DiBa accroung with YNAB account
    """
