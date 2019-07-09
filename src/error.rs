use failure::{Backtrace, Context, Fail};
use std::convert::From;
use std::fmt::{self, Display};
use std::result;

pub type Result<T> = result::Result<T, Error>;

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
        display = "failed to parse file as JSON provided via --category-mapping option: {}",
        _0
    )]
    ArgParseCategoryMappingCanNotParse(String),

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
