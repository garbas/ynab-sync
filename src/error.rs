// Handling Error
//
// `An Error and ErrorKind pair` pattern is the most robust way to manage
// errors - and also the most high maintenance. It combines some of theme
// advantages of the using ErrorKind pattern and the custom failure patterns,
// while avoiding some of theme disadvantages each of those patterns.
//
// More: https://github.com/rust-lang-nursery/failure/blob/master/book/src/error-errorkind.md

use exitfailure::ExitFailure;
use failure::{Backtrace, Context, Fail};
use std::convert::From;
use std::fmt::{self, Display};
use std::result;

pub type Result<T> = result::Result<T, ExitFailure>;

#[derive(Debug, Fail, PartialEq, Clone)]
pub enum ErrorKind {
    #[fail(display = "failed to parse option {}", _0)]
    ArgParse(String),

    #[fail(
        display = "failed to parse --days-to-sync option {} \n    => {}",
        _0, _1
    )]
    ArgParseDaysToSync(String, String),

    #[fail(
        display = "failed to read file provided via --category-mapping option: {}",
        _0
    )]
    ArgParseCategoryMappingCanNotRead(String),

    #[fail(
        display = "failed to parse file as JSON provided via --category-rules option: {}",
        _0
    )]
    ArgParseCategoryRulesCanNotRead(String),

    #[fail(
        display = "failed to read file provided via --category-rules option: {}",
        _0
    )]
    ArgParseCategoryRulesCanNotParse(String),

    #[fail(
        display = "failed to parse file as JSON provided via --category-mapping option: {}",
        _0
    )]
    ArgParseCategoryMappingCanNotParse(String),

    #[fail(display = "budget ({}) does not exists. ", _0)]
    WrongBudgetId(String),

    #[fail(display = "account ({}) does not exists. ", _0)]
    WrongAccountId(String),

    #[fail(display = "failed to parse type goal_type from YNAB category")]
    YNABCategoryGoalTypeParse,

    #[fail(display = "failed to parse type field from YNAB account")]
    YNABAccountTypeParse,

    #[fail(display = "failed to parse cleared field from YNAB transaction")]
    YNABTransactionClearedParse,

    #[fail(display = "failed to parse flag_color field from YNAB transaction")]
    YNABTransactionFlagColorParse,

    #[fail(display = "failed to fetch categories from YNAB")]
    YNABGetCategories,

    #[fail(display = "failed to fetch categories from YNAB: {} {}", _0, _1)]
    YNABGetCategoriesHttp(u16, String),

    #[fail(display = "failed to parse categories fetched from YNAB: {}", _0)]
    YNABGetCategoriesParse(String),

    #[fail(display = "failed to fetch accounts from YNAB")]
    YNABGetAccounts,

    #[fail(display = "failed to fetch accounts from YNAB: {} {}", _0, _1)]
    YNABGetAccountsHttp(u16, String),

    #[fail(display = "failed to parse accounts fetched from YNAB: {}", _0)]
    YNABGetAccountsParse(String),

    #[fail(display = "failed to fetch budgets from YNAB")]
    YNABGetBudgets,

    #[fail(display = "failed to fetch budgets from YNAB: {} {}", _0, _1)]
    YNABGetBudgetsHttp(u16, String),

    #[fail(display = "failed to parse budgets fetched from YNAB: {}", _0)]
    YNABGetBudgetsParse(String),

    #[fail(display = "failed to fetch transactions from YNAB")]
    YNABGetTransactions,

    #[fail(display = "failed to fetch transactions from YNAB: {} {}", _0, _1)]
    YNABGetTransactionsHttp(u16, String),

    #[fail(display = "failed to parse transactions fetched from YNAB: {}", _0)]
    YNABGetTransactionsParse(String),

    #[fail(display = "failed to save transactions to YNAB")]
    YNABSaveTransactions,

    #[fail(display = "failed to save transactions to YNAB: {} {}", _0, _1)]
    YNABSaveTransactionsHttp(u16, String),

    #[fail(display = "failed to authenticate against N26")]
    N26Authenticate,

    #[fail(display = "failed to get categories from N26")]
    N26GetCategories,

    #[fail(display = "failed to parse categories from N26: {}", _0)]
    N26GetCategoriesParse(String),

    #[fail(display = "failed to get categories from N26: {}, {}", _0, _1)]
    N26GetCategoriesHttp(u16, String),

    #[fail(display = "failed to get transactions from N26")]
    N26GetTransactions,

    #[fail(display = "failed to parse transactions from N26: {}", _0)]
    N26GetTransactionsParse(String),

    #[fail(display = "failed to get transactions from N26: {}, {}", _0, _1)]
    N26GetTransactionsHttp(u16, String),

    #[fail(display = "failed to open a file provided via --csv option: {}", _0)]
    IngDiBaCsvFileCanNotOpen(String),

    #[fail(display = "failed to parse transaction from: {}", _0)]
    IngDiBaCsvFileParse(String),
}

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

impl Error {
    pub fn kind(&self) -> &ErrorKind {
        &*self.inner.get_context()
    }
}

impl Fail for Error {
    fn name(&self) -> Option<&str> {
        self.inner.name()
    }

    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        let inner = Context::new(kind);
        Error { inner }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}
