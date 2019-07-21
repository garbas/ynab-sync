extern crate serde_str;

use crate::{ErrorKind, Result};
use chrono::{Duration, Utc};
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use failure::ResultExt;
use log::info;
use reqwest::{header, Method};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fmt;
use std::iter::FromIterator;
use std::result;
use std::str::FromStr;
use structopt::StructOpt;

const API_URL: &str = "https://api.youneedabudget.com/v1";

#[derive(Clone, StructOpt, Debug)]
pub struct Cli {
    #[structopt(
        long = "ynab-token",
        required = true,
        value_name = "TEXT",
        env = "YNAB_TOKEN",
        help = "YNAB token."
    )]
    pub token: String,
    #[structopt(
        long = "ynab-account-id",
        required = true,
        value_name = "TEXT",
        env = "YNAB_ACCOUNT_ID",
        help = "YNAB account id which you want to sync."
    )]
    pub account_id: String,
    #[structopt(
        long = "ynab-budget-id",
        required = true,
        value_name = "TEXT",
        env = "YNAB_BUDGET_ID",
        help = "YNAB budget id which you want to sync."
    )]
    pub budget_id: String,
    #[structopt(
        long = "force-update",
        help = "Force updating all transactions on YNAB."
    )]
    pub force_update: bool,
}

#[derive(Debug)]
pub struct YNAB {
    pub token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoriesRequest {
    pub data: CategoriesWrapper,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoriesWrapper {
    pub category_groups: Vec<CategoryGroup>,
    pub server_knowledge: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryGroup {
    pub id: String,
    pub name: String,
    pub hidden: bool,
    pub deleted: bool,
    pub categories: Vec<Category>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub category_group_id: String,
    pub name: String,
    pub hidden: bool,
    pub original_category_group_id: Option<String>,
    pub note: Option<String>,
    pub budgeted: i64,
    pub activity: i64,
    pub balance: i64,
    // #[serde(deserialize_with = "option_category_goal_type")]
    // pub goal_type: Option<CategoryGoalType>,
    pub goal_creation_month: Option<String>, // date
    pub goal_target: Option<i64>,
    pub goal_target_month: Option<String>, // date
    pub goal_percentage_complete: Option<i64>,
    pub deleted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CategoryGoalType {
    TB,
    TBD,
    MF,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountsRequest {
    pub data: AccountsWrapper,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountsWrapper {
    pub accounts: Vec<Account>,
    pub server_knowledge: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    #[serde(rename = "type", with = "serde_str")]
    pub type_: AccountType,
    pub on_budget: bool,
    pub closed: bool,
    pub note: Option<String>,
    pub balance: i64,
    pub cleared_balance: i64,
    pub uncleared_balance: i64,
    pub transfer_payee_id: String,
    pub deleted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AccountType {
    Checking,
    Savings,
    Cash,
    CreditCard,
    LineOfCredit,
    OtherAsset,
    OtherLiability,
    PayPal,
    MerchantAccount,
    InvestmentAccount,
    Mortgage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetsRequest {
    pub data: BudgetsWrapper,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BudgetsWrapper {
    pub budgets: Vec<Budget>,
    pub default_budget: Option<Budget>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Budget {
    pub id: String,
    pub name: String,
    pub last_modified_on: String, // datetime
    pub first_month: String,      // date
    pub last_month: String,       // date
    pub date_format: DateFormat,
    pub currency_format: CurrencyFormat,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DateFormat {
    format: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrencyFormat {
    pub iso_code: String,
    pub example_format: String,
    pub decimal_digits: i64,
    pub decimal_separator: String,
    pub symbol_first: bool,
    pub group_separator: String,
    pub currency_symbol: String,
    pub display_symbol: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionsRequest {
    pub data: TransactionsWrapper,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransactionsWrapper {
    pub transactions: Vec<Transaction>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub account_id: String,
    pub date: String,
    pub amount: i32,
    pub payee_id: Option<String>,
    pub payee_name: Option<String>,
    pub category_id: Option<String>,
    pub memo: Option<String>,
    #[serde(with = "serde_str")]
    pub cleared: TransactionCleared,
    pub approved: bool,
    pub flag_color: Option<TransactionFlagColor>,
    pub import_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionCleared {
    Cleared,
    Uncleared,
    Reconciled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransactionFlagColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
}

impl fmt::Display for CategoryGoalType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                CategoryGoalType::TB => "TB",
                CategoryGoalType::TBD => "TBD",
                CategoryGoalType::MF => "MF",
            },
        )
    }
}

impl FromStr for CategoryGoalType {
    type Err = ErrorKind;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s {
            "TB" => Ok(CategoryGoalType::TB),
            "TBD" => Ok(CategoryGoalType::TBD),
            "MF" => Ok(CategoryGoalType::MF),
            _ => Err(ErrorKind::YNABCategoryGoalTypeParse),
        }
    }
}

impl fmt::Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                AccountType::Checking => "checking",
                AccountType::Savings => "savings",
                AccountType::Cash => "cash",
                AccountType::CreditCard => "creditCard",
                AccountType::LineOfCredit => "lineOfCredit",
                AccountType::OtherAsset => "otherAsset",
                AccountType::OtherLiability => "otherLiability",
                AccountType::PayPal => "payPal",
                AccountType::MerchantAccount => "merchantAccount",
                AccountType::InvestmentAccount => "investmentAccount",
                AccountType::Mortgage => "mortgage",
            },
        )
    }
}

impl FromStr for AccountType {
    type Err = ErrorKind;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s {
            "checking" => Ok(AccountType::Checking),
            "savings" => Ok(AccountType::Savings),
            "cash" => Ok(AccountType::Cash),
            "creditCard" => Ok(AccountType::CreditCard),
            "lineOfCredit" => Ok(AccountType::LineOfCredit),
            "otherAsset" => Ok(AccountType::OtherAsset),
            "otherLiability" => Ok(AccountType::OtherLiability),
            "payPal" => Ok(AccountType::PayPal),
            "merchantAccount" => Ok(AccountType::MerchantAccount),
            "investmentAccount" => Ok(AccountType::InvestmentAccount),
            "mortgage" => Ok(AccountType::Mortgage),
            _ => Err(ErrorKind::YNABAccountTypeParse),
        }
    }
}

impl fmt::Display for TransactionCleared {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                TransactionCleared::Cleared => "cleared",
                TransactionCleared::Uncleared => "uncleared",
                TransactionCleared::Reconciled => "reconciled",
            },
        )
    }
}

impl FromStr for TransactionCleared {
    type Err = ErrorKind;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s {
            "cleared" => Ok(TransactionCleared::Cleared),
            "uncleared" => Ok(TransactionCleared::Uncleared),
            "reconciled" => Ok(TransactionCleared::Reconciled),
            _ => Err(ErrorKind::YNABTransactionClearedParse),
        }
    }
}

impl fmt::Display for TransactionFlagColor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                TransactionFlagColor::Red => "red",
                TransactionFlagColor::Orange => "orange",
                TransactionFlagColor::Yellow => "yellow",
                TransactionFlagColor::Green => "green",
                TransactionFlagColor::Blue => "blue",
                TransactionFlagColor::Purple => "purple",
            }
        )
    }
}

impl FromStr for TransactionFlagColor {
    type Err = ErrorKind;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s {
            "red" => Ok(TransactionFlagColor::Red),
            "orange" => Ok(TransactionFlagColor::Orange),
            "yellow" => Ok(TransactionFlagColor::Yellow),
            "green" => Ok(TransactionFlagColor::Green),
            "blue" => Ok(TransactionFlagColor::Blue),
            "purple" => Ok(TransactionFlagColor::Purple),
            _ => Err(ErrorKind::YNABTransactionFlagColorParse),
        }
    }
}

impl YNAB {
    pub fn validate_cli(&self, cli: Cli, step: i32, steps: i32) -> Result<()> {
        // Fetch budgets and verify that budget_id is correct
        println!("[{}/{}] Verifying --budget-id", step + 1, steps);
        if self
            .get_budgets()?
            .into_iter()
            .filter(|x| x.id == cli.budget_id)
            .count()
            != 1
        {
            Err(ErrorKind::WrongBudgetId(cli.budget_id.clone()))?
        }

        // Fetch accounts and verify that account_id is correct
        println!("[{}/{}] Verifying --account-id", step + 2, steps);
        if self
            .get_accounts(cli.budget_id.clone())?
            .into_iter()
            .filter(|x| x.id == cli.account_id)
            .count()
            != 1
        {
            Err(ErrorKind::WrongAccountId(cli.account_id.clone()))?
        }

        Ok(())
    }
    pub fn get_categories(&self, budget_id: String) -> Result<HashMap<String, Category>> {
        let url = format!("{}/budgets/{}/categories", API_URL, budget_id);
        let authorization = format!("Bearer {}", self.token);
        let client = reqwest::Client::new();
        let mut res = client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .send()
            .context(ErrorKind::YNABGetCategories)?;

        let body = res.text().context(ErrorKind::YNABGetCategories)?;
        info!("{}", body);

        if !res.status().is_success() {
            let http_error = ErrorKind::YNABGetCategoriesHttp(res.status().as_u16(), body.clone());
            Err(http_error)?;
        }

        let req: CategoriesRequest = serde_json::from_str(&body)
            .with_context(|e| ErrorKind::YNABGetCategoriesParse(e.to_string()))?;

        let categories = req
            .data
            .category_groups
            .into_iter()
            .map(|x| x.categories)
            .flatten()
            .map(|x| (x.name.clone(), x.clone()));

        Ok(HashMap::from_iter(categories))
    }

    pub fn get_budgets(&self) -> Result<Vec<Budget>> {
        let url = format!("{}/budgets", API_URL,);
        let authorization = format!("Bearer {}", self.token);
        let client = reqwest::Client::new();
        let mut res = client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .send()
            .context(ErrorKind::YNABGetBudgets)?;

        let body = res.text().context(ErrorKind::YNABGetBudgets)?;
        info!("{}", body);

        if !res.status().is_success() {
            let http_error = ErrorKind::YNABGetBudgetsHttp(res.status().as_u16(), body.clone());
            Err(http_error)?;
        }

        let req: BudgetsRequest = serde_json::from_str(&body)
            .with_context(|e| ErrorKind::YNABGetBudgetsParse(e.to_string()))?;

        Ok(req.data.budgets)
    }

    pub fn get_accounts(&self, budget_id: String) -> Result<Vec<Account>> {
        let url = format!("{}/budgets/{}/accounts", API_URL, budget_id);
        let authorization = format!("Bearer {}", self.token);
        let client = reqwest::Client::new();
        let mut res = client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .send()
            .context(ErrorKind::YNABGetAccounts)?;

        let body = res.text().context(ErrorKind::YNABGetAccounts)?;
        info!("{}", body);

        if !res.status().is_success() {
            let http_error = ErrorKind::YNABGetAccountsHttp(res.status().as_u16(), body.clone());
            Err(http_error)?;
        }

        let req: AccountsRequest = serde_json::from_str(&body)
            .with_context(|e| ErrorKind::YNABGetAccountsParse(e.to_string()))?;

        Ok(req.data.accounts)
    }
    pub fn get_transactions(
        &self,
        budget_id: String,
        account_id: String,
        days: i64,
    ) -> Result<HashMap<String, Transaction>> {
        let now = Utc::now();
        let days_ago = now - Duration::days(days);
        let since_date = days_ago.format("%Y-%m-%d");

        let url = format!(
            "{}/budgets/{}/accounts/{}/transactions?since_date={}",
            API_URL, budget_id, account_id, since_date
        );
        let authorization = format!("Bearer {}", self.token);
        let client = reqwest::Client::new();
        let mut res = client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .send()
            .context(ErrorKind::YNABGetTransactions)?;

        let body = res.text().context(ErrorKind::YNABGetTransactions)?;
        info!("{}", body);

        if !res.status().is_success() {
            let http_error =
                ErrorKind::YNABGetTransactionsHttp(res.status().as_u16(), body.clone());
            Err(http_error)?;
        }

        let req: TransactionsRequest = serde_json::from_str(&body)
            .with_context(|e| ErrorKind::YNABGetTransactionsParse(e.to_string()))?;

        let transactions = HashMap::from_iter(
            req.data
                .transactions
                .iter()
                .filter(|x| x.import_id.is_some())
                .map(|x| {
                    (
                        x.import_id.clone().unwrap_or_else(|| String::from("")),
                        x.clone(),
                    )
                }),
        );

        Ok(transactions)
    }
    pub fn sync(
        &self,
        transactions: Vec<Transaction>,
        existing_transactions: HashMap<String, Transaction>,
        budget_id: String,
        force_update: bool,
        step: i32,
        steps: i32,
    ) -> Result<()> {
        // figure out which transactions are new and which we need to update
        let mut new_transactions: Vec<Transaction> = vec![];
        let mut update_transactions: Vec<Transaction> = vec![];
        for transaction in transactions.iter() {
            if let Some(import_id) = transaction.import_id.clone() {
                // filter out transactions that don't need to be updated
                // that means if import_id matches amount and date should
                // be the same as in n26 transaction
                let existing_transaction = existing_transactions.get(&import_id);
                if existing_transaction.map(|x| x.amount) == Some(transaction.amount)
                    && existing_transaction.map(|x| x.date.clone())
                        == Some(transaction.date.clone())
                    && (!force_update
                        || existing_transaction.map(|x| x.category_id.clone())
                            == Some(transaction.category_id.clone()))
                {
                    continue;
                }
                if existing_transactions.contains_key(import_id.as_str()) {
                    update_transactions.push(transaction.clone());
                } else {
                    new_transactions.push(transaction.clone());
                }
            } else {
                new_transactions.push(transaction.clone());
            }
        }

        if new_transactions.is_empty() && update_transactions.is_empty() {
            println!("[{}/{}] No transactions to update.", step, steps);
            return Ok(());
        }

        let selections = &["Yes", "No"];
        let prompt = format!(
            "[[{: >2}/10] ] Do you want to sync transactions with YNAB [{}/{}]?",
            step + 1,
            new_transactions.len(),
            update_transactions.len(),
        );
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt(&prompt)
            .default(1)
            .items(&selections[..])
            .interact()
            .unwrap();

        if selection == 0 {
            if !new_transactions.is_empty() {
                println!(" => Creating new YNAB transactions");
                self.save_transactions(new_transactions, budget_id.clone(), Method::POST)?;
            }
            if !update_transactions.is_empty() {
                println!(" => Updating YNAB transactions");
                self.save_transactions(update_transactions, budget_id.clone(), Method::PATCH)?;
            }
        }

        Ok(())
    }
    fn save_transactions(
        &self,
        transactions: Vec<Transaction>,
        budget_id: String,
        method: Method,
    ) -> Result<()> {
        let wrapper = TransactionsWrapper { transactions };

        let url = format!("{}/budgets/{}/transactions", API_URL, budget_id);
        let authorization = format!("Bearer {}", self.token);
        let req_body =
            serde_json::to_string(&wrapper).context(ErrorKind::YNABSaveTransactions.clone())?;
        info!("{}", req_body);

        let client = reqwest::Client::new();
        let mut res = client
            .request(method, &url)
            .header(header::AUTHORIZATION, authorization)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/json")
            .body(req_body)
            .send()
            .context(ErrorKind::YNABSaveTransactions.clone())?;

        if !res.status().is_success() {
            let res_body = res
                .text()
                .context(ErrorKind::YNABSaveTransactions.clone())?;
            let http_error =
                ErrorKind::YNABSaveTransactionsHttp(res.status().as_u16(), res_body.clone());
            Err(http_error)?;
        }

        Ok(())
    }
}
