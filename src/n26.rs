use crate::{ErrorKind, Result};
use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Duration, Utc};
use failure::ResultExt;
use oauth2::{AuthType, Config, Token};
use reqwest::header;
use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::result;

const API_URL: &str = "https://api.tech26.de";

#[derive(Debug)]
pub struct N26 {
    access_token: Token,
}

#[derive(Debug, Deserialize)]
pub struct Category {
    id: String,
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct Transaction {
    pub id: String,

    #[serde(rename = "userId")]
    pub user_id: String,

    #[serde(rename = "type")]
    pub type_: String, // XXX: enum

    #[serde(deserialize_with = "convert_to_cents")]
    pub amount: i32,

    #[serde(rename = "currencyCode")]
    pub currency_code: String, // XXX: enum

    // TODO: Doesn't work with Option
    //
    // #[serde(rename = "originalAmount", deserialize_with = "convert_to_cents")]
    // pub original_amount: Option<i32>,
    //
    //#[serde(rename = "originalCurrency")]
    //pub original_currency: String, // XXX: enum
    //
    #[serde(rename = "exchangeRate")]
    pub exchange_rate: Option<f64>,

    #[serde(rename = "merchantCity")]
    pub merchant_city: Option<String>,

    #[serde(rename = "visibleTS", with = "ts_milliseconds")]
    pub visible_ts: DateTime<Utc>,

    #[serde(rename = "mcc")]
    pub mcc: Option<i32>,

    #[serde(rename = "mccGroup")]
    pub mcc_group: Option<i32>,

    #[serde(rename = "merchantName")]
    pub merchant_name: Option<String>,

    #[serde(rename = "recurring")]
    pub recurring: bool,

    #[serde(rename = "partnerBic")]
    pub partner_bic: Option<String>,

    #[serde(rename = "partnerAccountIsSepa")]
    pub partner_account_is_sepa: Option<bool>,

    #[serde(rename = "partnerName")]
    pub partner_name: Option<String>,

    #[serde(rename = "accountId")]
    pub account_id: String,

    #[serde(rename = "partnerIban")]
    pub partner_iban: Option<String>,

    #[serde(rename = "category")]
    pub category: String,

    #[serde(rename = "cardId")]
    pub card_id: Option<String>,

    #[serde(rename = "referenceText")]
    pub reference_text: Option<String>,

    // TODO: Doesn't work with Option
    //
    // #[serde(rename = "userAccepted", with = "ts_milliseconds")]
    // pub user_accepted: Option<DateTime<Utc>>,
    //
    #[serde(rename = "userCertified", with = "ts_milliseconds")]
    pub user_certified: DateTime<Utc>,

    #[serde(rename = "pending")]
    pub pending: bool,

    #[serde(rename = "transactionNature")]
    pub transaction_nature: String, // XXX: enum

    #[serde(rename = "createdTS", with = "ts_milliseconds")]
    pub created_ts: DateTime<Utc>,

    #[serde(rename = "merchantCountry")]
    pub merchant_country: Option<i32>,

    #[serde(rename = "smartLinkId")]
    pub smart_link_id: String,

    #[serde(rename = "linkId")]
    pub link_id: String,

    #[serde(rename = "confirmed", with = "ts_milliseconds")]
    pub confirmed: DateTime<Utc>,
}

fn convert_to_cents<'de, D>(deserializer: D) -> result::Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    struct I32Visitor;

    impl<'de> Visitor<'de> for I32Visitor {
        type Value = i32;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a cent representation in i32 of an amount provided in f32")
        }
        fn visit_f64<E>(self, value: f64) -> result::Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(((value * 100.0).round()) as Self::Value)
        }
    }

    deserializer.deserialize_f64(I32Visitor)
}

impl N26 {
    pub fn new(username: String, password: String) -> Result<Self> {
        let authorize_url = format!("{}/noop", API_URL);
        let token_url = format!("{}/oauth/token", API_URL);
        let mut config = Config::new("android", "secret", authorize_url, token_url);
        config = config.set_auth_type(AuthType::BasicAuth);

        let access_token = config
            .exchange_password(username, password)
            .context(ErrorKind::N26Authenticate)?;

        let client = N26 { access_token };
        Ok(client)
    }

    pub fn get_categories(self: &Self) -> Result<HashMap<String, String>> {
        let url = format!("{}/api/smrt/categories", API_URL);

        let client = reqwest::Client::new();
        let authorization = format!("Bearer {}", self.access_token.access_token);
        let mut res = client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .send()
            .context(ErrorKind::N26GetCategories)?;

        let body = res.text().context(ErrorKind::N26GetCategories)?;

        if !res.status().is_success() {
            let http_error = ErrorKind::N26GetCategoriesHttp(res.status().as_u16(), body.clone());
            Err(http_error)?;
        }

        let categories_vec: Vec<Category> = serde_json::from_str(&body)
            .with_context(|e| ErrorKind::N26GetCategoriesParse(e.to_string()))?;

        let mut categories = HashMap::new();
        for category in &categories_vec {
            categories.insert(category.id.clone(), category.name.clone());
        }

        Ok(categories)
    }

    pub fn get_transactions(self: &Self, days: i64, limit: i64) -> Result<Vec<Transaction>> {
        let now = Utc::now();
        let a_month_ago = now - Duration::days(days);

        // `from` and `to` have to be used together.
        let from = a_month_ago.timestamp_millis();
        let to = now.timestamp_millis();
        let url = format!(
            "{}/api/smrt/transactions?from={}&to={}&limit={}",
            API_URL, from, to, limit
        );

        let client = reqwest::Client::new();
        let authorization = format!("Bearer {}", self.access_token.access_token);
        let mut res = client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .send()
            .context(ErrorKind::N26GetTransactions)?;

        let body = res.text().context(ErrorKind::N26GetTransactions)?;

        if !res.status().is_success() {
            let http_error = ErrorKind::N26GetTransactionsHttp(res.status().as_u16(), body.clone());
            Err(http_error)?;
        }

        let transactions = serde_json::from_str(&body)
            .with_context(|e| ErrorKind::N26GetTransactionsParse(e.to_string()))?;

        Ok(transactions)
    }
}
