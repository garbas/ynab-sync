# -*- coding: utf-8 -*-

import datetime
import itertools
import json
import math
import time
import typing

import click
import mypy_extensions
import requests
import typing_extensions


class Api:

    GET = "get"
    POST = "post"

    def __init__(self, token):
        self.token: str = token

    def do_request(
        self, method: str = GET, url: str = "/", params: dict = None, data: dict = None
    ) -> typing.Any:
        headers = {
            "Authorization": f"Bearer {self.token}",
            "Accept": "application/json",
        }
        url = self.create_request_url(url, params)

        if method is self.GET:
            response = requests.get(url, headers=headers)
        elif method is self.POST:
            response = requests.post(url, headers=headers, json=data)
        else:
            raise ValueError(f"Unsupported method: {method}")

        response.raise_for_status()
        return response.json()

    def create_request_url(self, url: str, params: dict = None):
        if params:
            first_param = True
            for k, v in sorted(params.items(), key=lambda entry: entry[0]):
                if not v:
                    # skip None values
                    continue

                if first_param:
                    url += "?"
                    first_param = False
                else:
                    url += "&"

                url += f"{k}={v}"

        return url


N26Currency = typing_extensions.Literal["EUR", "US"]
N26TransactionMerchant = mypy_extensions.TypedDict(
    "N26TransactionMerchant",
    {
        "accountId": str,
        "amount": float,
        "cardId": str,
        "category": str,
        "confirmed": int,
        "createdTS": int,
        "currencyCode": N26Currency,
        "exchangeRate": float,
        "id": str,
        "linkId": str,
        "mcc": int,
        "mccGroup": int,
        "merchantCity": str,
        "merchantCountry": int,
        "merchantName": str,
        "originalAmount": float,
        "originalCurrency": N26Currency,
        "partnerAccountIsSepa": bool,
        "pending": bool,
        "recurring": bool,
        "smartLinkId": str,
        "transactionNature": str,
        "transactionTerminal": str,
        "type": str,
        "userCertified": int,
        "userId": str,
        "visibleTS": int,
    },
)
N26TransactionPartner = mypy_extensions.TypedDict(
    "N26TransactionPartner",
    {
        "accountId": str,
        "amount": float,
        "category": str,
        "confirmed": int,
        "createdTS": int,
        "currencyCode": N26Currency,
        "id": str,
        "linkId": str,
        "mcc": int,
        "mccGroup": int,
        "partnerAccountIsSepa": bool,
        "partnerBic": str,
        "partnerIban": str,
        "partnerName": str,
        "pending": bool,
        "purposeCode": str,
        "recurring": bool,
        "referenceText": str,
        "smartLinkId": str,
        "transactionNature": str,
        "transactionTerminal": str,
        "type": str,
        "userAccepted": int,
        "userCertified": int,
        "userId": str,
        "visibleTS": int,
    },
)
N26Transaction = typing.Union[N26TransactionMerchant, N26TransactionPartner]


class N26Api(Api):
    """Code borrowed from https://github.com/femueller/python-n26/blob/master/n26/api.py
    """

    BASE_URL = "https://api.tech26.de"
    BASIC_AUTH_HEADERS = {"Authorization": "Basic YW5kcm9pZDpzZWNyZXQ="}

    EXPIRATION_TIME_KEY = "expiration_time"
    ACCESS_TOKEN_KEY = "access_token"
    REFRESH_TOKEN_KEY = "refresh_token"

    def __init__(self, username: str, password: str):
        self.username = username
        self.password = password
        self.token_data: dict = {}

    def get_info(self) -> dict:
        return self.do_request(self.GET, f"{self.BASE_URL}/api/me")

    def get_account(self) -> dict:
        return self.do_request(self.GET, f"{self.BASE_URL}/api/accounts")

    def get_available_categories(self) -> dict:
        return self.do_request(self.GET, f"{self.BASE_URL}/api/smrt/categories")

    def get_transactions(
        self,
        from_time: int = None,
        to_time: int = None,
        limit: int = None,
        pending: bool = None,
        categories: str = None,
        text_filter: str = None,
        last_id: str = None,
    ) -> dict:
        if pending and limit:
            # pending does not support limit
            limit = None

        return self.do_request(
            self.GET,
            f"{self.BASE_URL}/api/smrt/transactions",
            {
                "from": from_time,
                "to": to_time,
                "limit": limit,
                "pending": pending,
                "categories": categories,
                "textFilter": text_filter,
                "lastId": last_id,
            },
        )

    @property
    def token(self):
        if not self.validate_token(self.token_data):
            if self.REFRESH_TOKEN_KEY in self.token_data:
                refresh_token = self.token_data[self.REFRESH_TOKEN_KEY]
                self.token_data = self.refresh_token(refresh_token)
            else:
                self.token_data = self.request_token(self.username, self.password)
            self.token_data[self.EXPIRATION_TIME_KEY] = (
                time.time() + self.token_data["expires_in"]
            )

        if not self.validate_token(self.token_data):
            raise PermissionError("Unable to request authentication token")

        return self.token_data[self.ACCESS_TOKEN_KEY]

    def request_token(self, username: str, password: str):
        values_token = {
            "grant_type": "password",
            "username": username,
            "password": password,
        }

        response = requests.post(
            f"{self.BASE_URL}/oauth/token",
            data=values_token,
            headers=self.BASIC_AUTH_HEADERS,
        )
        response.raise_for_status()
        return response.json()

    def refresh_token(self, refresh_token: str):
        values_token = {
            "grant_type": self.REFRESH_TOKEN_KEY,
            "refresh_token": refresh_token,
        }

        response = requests.post(
            f"{self.BASE_URL}/oauth/token",
            data=values_token,
            headers=self.BASIC_AUTH_HEADERS,
        )
        response.raise_for_status()
        return response.json()

    def validate_token(self, token_data: dict):
        if self.EXPIRATION_TIME_KEY not in token_data:
            # there was a problem adding the expiration_time property
            return False
        elif time.time() >= token_data[self.EXPIRATION_TIME_KEY]:
            # token has expired
            return False

        return self.ACCESS_TOKEN_KEY in token_data and token_data[self.ACCESS_TOKEN_KEY]


YNABTransactionCleared = typing_extensions.Literal["cleared", "uncleared", "reconciled"]
YNABTransactionFlagColor = typing_extensions.Literal[
    "red", "orange", "yellow", "green", "blue", "purple"
]
YNABTransaction = mypy_extensions.TypedDict(
    "YNABTransaction",
    {
        "account_id": str,
        "date": str,
        "amount": int,
        "payee_id": typing.Optional[str],
        "payee_name": typing.Optional[str],
        "category_id": typing.Optional[str],
        "memo": typing.Optional[str],
        "cleared": typing.Optional[YNABTransactionCleared],
        "approved": typing.Optional[bool],
        "flag_color": typing.Optional[YNABTransactionFlagColor],
        "import_id": typing.Optional[str],
    },
)
YNABPayee = mypy_extensions.TypedDict(
    "YNABPayee", {"id": str, "name": str, "transfer_account_id": str, "deleted": bool}
)
YNABGetCategory = mypy_extensions.TypedDict(
    "YNABGetCategory",
    {
        "id": str,
        "category_group_id": str,
        "name": str,
        "hidden": str,
        "original_category_group_id": typing.Optional[str],
        "note": str,
        "budgeted": int,
        "activity": int,
        "balance": int,
        "goal_type": typing_extensions.Literal["TB", "TBD", "MF"],
        "goal_creation_month": str,
        "goal_target": int,
        "goal_target_month": int,
        "goal_percentage_complete": int,
        "deleted": bool,
    },
)
YNABGetCategoryGroup = mypy_extensions.TypedDict(
    "YNABGetCategoryGroup",
    {
        "id": str,
        "name": str,
        "hidden": bool,
        "deleted": bool,
        "categories": typing.List[YNABGetCategory],
    },
)
YNABGetCategoriesData = mypy_extensions.TypedDict(
    "YNABGetCategoriesData",
    {"category_groups": typing.List[YNABGetCategoryGroup], "server_knowledge": int},
)
YNABGetCategories = mypy_extensions.TypedDict(
    "YNABGetCategories", {"data": YNABGetCategoriesData}
)
YNABGetPayeesData = mypy_extensions.TypedDict(
    "YNABGetPayeesData", {"payees": typing.List[YNABPayee], "server_knowledge": int}
)
YNABGetPayees = mypy_extensions.TypedDict("YNABGetPayees", {"data": YNABGetPayeesData})
YNABGetTransactionsData = mypy_extensions.TypedDict(
    "YNABGetTransactionsData", {"transactions": typing.List[YNABTransaction]}
)
YNABGetTransactions = mypy_extensions.TypedDict(
    "YNABGetTransactions", {"data": YNABGetTransactionsData}
)


class YNABApi(Api):

    BASE_URL = "https://api.youneedabudget.com/v1"

    def get_categories(self, budget_id: str) -> YNABGetCategories:
        return self.do_request(
            self.GET, f"{self.BASE_URL}/budgets/{budget_id}/categories"
        )

    def get_payees(self, budget_id: str) -> YNABGetPayees:
        return self.do_request(self.GET, f"{self.BASE_URL}/budgets/{budget_id}/payees")

    def get_budgets(self) -> dict:
        return self.do_request(self.GET, f"{self.BASE_URL}/budgets")

    def get_accounts(self, budget_id: str) -> dict:
        return self.do_request(
            self.GET, f"{self.BASE_URL}/budgets/{budget_id}/accounts"
        )

    def get_account_transations(
        self, budget_id: str, account_id: str, since_date: str = None
    ) -> YNABGetTransactions:
        params = {}
        if since_date:
            params["since_date"] = since_date
        return self.do_request(
            self.GET,
            f"{self.BASE_URL}/budgets/{budget_id}/accounts/{account_id}/transactions",
            params,
        )

    def create_transactions(
        self, budget_id: str, transactions: typing.List[YNABTransaction]
    ) -> dict:
        transactions_ = []
        for transaction in transactions:
            transactions_.append(
                {k: v for k, v in transaction.items() if v is not None}
            )
        return self.do_request(
            self.POST,
            f"{self.BASE_URL}/budgets/{budget_id}/transactions",
            data=dict(transactions=transactions_),
        )


def convert_n26_transaction(
    account_id,
    ynab_payees: typing.List[YNABPayee],
    categories: dict,
    ynab_categories: dict,
    n26_categories: dict,
    transaction: dict,
) -> YNABTransaction:

    memo = None
    if "referenceText" in transaction:
        memo = transaction["referenceText"]
    elif "merchantName" in transaction:
        memo = transaction["merchantName"]
        if "merchantCity" in transaction:
            memo += " " + transaction["merchantCity"]

    # TODO: we can convert the category from transaction['category']
    category = None

    if (
        "category" in transaction
        and transaction["category"] in n26_categories
        and n26_categories[transaction["category"]] in categories
        and categories[n26_categories[transaction["category"]]] in ynab_categories
    ):
        category = ynab_categories[categories[n26_categories[transaction["category"]]]]

    return {
        "account_id": account_id,
        "date": datetime.datetime.fromtimestamp(
            transaction["visibleTS"] / 1000
        ).strftime("%Y-%m-%d"),
        "amount": math.ceil(transaction["amount"] * 1000),
        # TODO: for regular tranfers we can detect the payee
        "payee_id": None,  # typing.Optional[str],
        "payee_name": None,  # typing.Optional[str],
        "category_id": category,
        "memo": memo,
        "cleared": "cleared",
        "approved": category is not None,
        "flag_color": None,  # typing.Optional[YNABTransactionFlagColor],
        "import_id": transaction["id"],
    }


@click.command()
@click.option("--n26-username", prompt=True)
@click.option("--n26-password", prompt=True)
@click.option("--ynab-token", prompt=True)
@click.option("--ynab-budget-id", prompt=True)
@click.option("--ynab-account-id", prompt=True)
@click.option("--last-synced", prompt=True)
@click.option("--categories", type=click.File())
def main(
    n26_username: str,
    n26_password: str,
    ynab_token: str,
    ynab_budget_id: str,
    ynab_account_id: str,
    last_synced: str,
    categories: typing.IO[str],
):
    """Sync n26 account with YNAB account
    """
    categories_ = json.loads(categories.read())
    n26 = N26Api(n26_username, n26_password)
    ynab = YNABApi(ynab_token)

    # get n26 account info, this also checks that n26 credentials are correct
    n26_info = n26.get_info()
    n26_account = n26.get_account()

    # check if ynab_budget_id and ynab_account_id are correct, this also checks
    # that ynab token is correct
    ynab_budgets = ynab.get_budgets().get("data", {}).get("budgets", {})
    if ynab_budget_id not in list(map(lambda x: x["id"], ynab_budgets)):
        click.secho(
            f"ERROR: '{ynab_budget_id}' is not correct YNAB budget id.", fg="red"
        )
        click.echo("")
        click.echo(f"Please select one of:")
        for budget in ynab_budgets:
            click.echo(f" - {budget['name']} - {budget['id']}")

    ynab_accounts = (
        ynab.get_accounts(ynab_budget_id).get("data", {}).get("accounts", {})
    )
    if ynab_account_id not in list(map(lambda x: x["id"], ynab_accounts)):
        click.secho(
            f"ERROR: '{ynab_account_id}' is not correct YNAB account id.", fg="red"
        )
        click.echo("")
        click.echo(f"Please select one of:")
        for account in ynab_accounts:
            click.echo(f" - {account['name']} - {account['id']}")
    ynab_account = [i for i in ynab_accounts if i["id"] == ynab_account_id][0]

    # get ynab payees
    ynab_payees = ynab.get_payees(ynab_budget_id).get("data", {}).get("payees", [])

    # get ynab payees
    ynab_categories = {
        name: id_
        for name, id_ in list(
            itertools.chain(
                *[
                    [(i["name"], i["id"]) for i in j["categories"]]
                    for j in ynab.get_categories(ynab_budget_id)
                    .get("data", {})
                    .get("category_groups", [])
                ]
            )
        )
    }

    # get ynab transactions since last sync
    ynab_transations = (
        ynab.get_account_transations(ynab_budget_id, ynab_account_id, last_synced)
        .get("data", {})
        .get("transactions", [])
    )

    # get n26 categories
    n26_categories = {i["id"]: i["name"] for i in n26.get_available_categories()}

    # get n26 transactions since last sync
    n26_transactions = sorted(
        n26.get_transactions(
            from_time=int(
                datetime.datetime.strptime(last_synced, "%Y-%m-%d").timestamp() * 1000
            ),
            limit=100_000_000,  # something very high
        ),
        reverse=True,
        key=lambda x: x["visibleTS"],
    )

    # calculate new transactions to be created
    ynab_transations_import_ids = [t["import_id"] for t in ynab_transations]
    new_transactions = [
        convert_n26_transaction(
            ynab_account_id,
            ynab_payees,
            categories_,
            ynab_categories,
            n26_categories,
            t,
        )
        for t in n26_transactions
        if t["id"] not in ynab_transations_import_ids
    ]

    # print short preview and ask if you want to see longer review which
    # transaction is going to be created
    fullName = f"{n26_info['firstName']} {n26_info['lastName']}"
    click.echo(f"You are about create")
    click.echo(f"    {len(new_transactions)} new transactions")
    click.echo(f"to")
    click.echo(f"    {ynab_account['name']} YNAB account")
    click.echo(f"from")
    click.echo(f"    {n26_account['iban']} ({fullName}) bank account.")
    click.echo("")
    if not click.confirm("Do you want to continue?"):
        click.echo("")
        click.secho("No transactions were created.", fg="yellow")
        return

    # create transactions
    result = ynab.create_transactions(ynab_budget_id, new_transactions).get("data", {})
    created_transaction_ids = result.get("transaction_ids", [])
    click.secho(
        f"{len(created_transaction_ids)} transactions were created successfully!",
        fg="green",
    )
