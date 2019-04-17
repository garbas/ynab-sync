# -*- coding: utf-8 -*-

import typing

import mypy_extensions

Transaction = mypy_extensions.TypedDict(
    "Transaction",
    {"Date": str, "Payee": str, "Memo": str, "Outflow": float, "Inflow": float},
)
Transactions = typing.List[Transaction]


def import_transactions(transactions):
    pass
