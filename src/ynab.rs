extern crate serde_str;

use crate::{ErrorKind, Result};
use failure::ResultExt;
use reqwest::{header, Method};
use serde::{Serialize};
use std::fmt;

const API_URL: &str = "https://api.youneedabudget.com/v1";

#[derive(Debug)]
pub struct YNAB {
    pub token: String,

    pub budget_id: String,

    pub account_id: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionsRequest {
    pub data: TransactionsWrapper,
}

#[derive(Clone, Debug, Serialize)]
pub struct TransactionsWrapper {
    pub transactions: Vec<Transaction>,
}

#[derive(Clone, Debug, Serialize)]
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

#[derive(Clone, Debug, Serialize)]
pub enum TransactionCleared {
    Cleared,
    Uncleared,
    Reconciled,
}

#[derive(Clone, Debug, Serialize)]
pub enum TransactionFlagColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
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
            },
        )
    }
}

impl YNAB {
    pub fn get_transactions(&self) -> Vec<Transaction> {
        // TODO: account id should be passed in
        //unimplemented!()
        vec![]
    }

    pub fn save_transactions(&self, transactions: Vec<Transaction>) -> Result<()> {
        let wrapper = TransactionsWrapper { transactions };

        let url = format!("{}/budgets/{}/transactions", API_URL, self.budget_id);
        let authorization = format!("Bearer {}", self.token);
        let req_body =
            serde_json::to_string(&wrapper).context(ErrorKind::YNABSaveTransactions.clone())?;

        let client = reqwest::Client::new();
        let mut res = client
            .request(Method::POST, &url)
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
