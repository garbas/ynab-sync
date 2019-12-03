use crate::convert_to_int;
use crate::{ErrorKind, Result};
use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Duration, Utc};
use dirs::cache_dir;
use failure::ResultExt;
use log::{debug, info};
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env::current_dir;
use std::fs::{read_to_string, write};
use std::thread::sleep;
use std::time;
use structopt::StructOpt;

const API_URL: &str = "https://api.tech26.de";
const API_BASIC_AUTH_HEADER: &str = "Basic YW5kcm9pZDpzZWNyZXQ=";
const API_USER_AGENT : &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/59.0.3071.86 Safari/537.36";

#[derive(StructOpt, Debug)]
pub struct Cli {
    #[structopt(
        long = "n26-username",
        required = true,
        value_name = "TEXT",
        env = "N26_USERNAME",
        help = "Username that you use to login to https://app.n26.com"
    )]
    pub username: String,
    #[structopt(
        long = "n26-password",
        required = true,
        value_name = "TEXT",
        env = "N26_PASSWORD",
        help = "Password that you use to login to https://app.n26.com"
    )]
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct N26 {
    pub expiration_time: i64,

    pub access_token: String,

    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct MFAToken {
    pub error: String,

    #[serde(rename = "mfaToken")]
    pub mfa_token: String,
}

#[derive(Debug, Deserialize)]
pub struct TokenData {
    pub access_token: String,

    pub token_type: String,

    pub refresh_token: String,

    pub expires_in: i64,
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

    #[serde(deserialize_with = "convert_to_int")]
    pub amount: i32,

    #[serde(rename = "currencyCode")]
    pub currency_code: String, // XXX: enum

    // TODO: Doesn't work with Option
    //
    // #[serde(rename = "originalAmount", deserialize_with = "convert_to_int")]
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

fn complete_mfa_approval(mfa_token: String) -> Result<N26> {
    info!("Calling complete_mfa_approval");

    let client = reqwest::Client::new();

    let mut data = HashMap::new();
    data.insert("grant_type", "mfa_oob");
    data.insert("mfaToken", mfa_token.as_str());

    let url = format!("{}/oauth/token", API_URL);
    debug!("Url to complete mfa is: {}", url);
    let mut res = client
        .post(&url)
        .header(header::AUTHORIZATION, API_BASIC_AUTH_HEADER)
        .header(header::USER_AGENT, API_USER_AGENT)
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&data)
        .send()
        .context(ErrorKind::N26AuthenticateCompleteMFA)?;

    let body = res.text().context(ErrorKind::N26AuthenticateCompleteMFA)?;
    debug!("{}", body);

    if res.status() == 200 {
        let data: TokenData = serde_json::from_str(&body)
            .with_context(|e| ErrorKind::N26AuthenticateCompleteMFAParse(e.to_string()))?;
        Ok(N26 {
            expiration_time: Utc::now().timestamp() + data.expires_in,
            access_token: data.access_token.clone(),
            refresh_token: data.refresh_token.clone(),
        })
    } else {
        Err(ErrorKind::N26AuthenticateCompleteMFA)?
    }
}

fn request_mfa_approval(mfa_token: String) -> Result<N26> {
    info!("Calling request_mfa_approval");

    let client = reqwest::Client::new();

    let mut data = HashMap::new();
    data.insert("challengeType", "oob");
    data.insert("mfaToken", mfa_token.as_str());

    let url = format!("{}/api/mfa/challenge", API_URL);
    debug!("Url to start mfa approval is: {}", url);
    let mut res = client
        .post(&url)
        .header(header::AUTHORIZATION, API_BASIC_AUTH_HEADER)
        .header(header::USER_AGENT, API_USER_AGENT)
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/json")
        .json(&data)
        .send()
        .context(ErrorKind::N26AuthenticateMfaApproval)?;

    let body = res.text().context(ErrorKind::N26AuthenticateMfaApproval)?;
    debug!("{}", body);

    if res.status() != 201 {
        Err(ErrorKind::N26AuthenticateMfaApproval)?
    } else {
        let mut token = complete_mfa_approval(mfa_token.clone());
        if token.is_ok() {
            token
        } else {
            for i in 1..13 {
                debug!("Sleeping for 5 seconds");
                sleep(time::Duration::from_secs(5));
                token = complete_mfa_approval(mfa_token.clone());
                debug!("token data: {:?}", token);
                if token.is_ok() {
                    break;
                }
                info!("Remaining {} seconds", (12 - i) * 5);
            }
            token
        }
    }
}

fn new_authenticate(username: String, password: String) -> Result<N26> {
    info!("Calling new_authenticate");

    let client = reqwest::Client::new();

    let mut data = HashMap::new();
    data.insert("grant_type", "password");
    data.insert("username", username.as_str());
    data.insert("password", password.as_str());

    let url = format!("{}/oauth2/token", API_URL);
    debug!("Url to start authorization is: {}", url);
    let mut res = client
        .post(&url)
        .header(header::AUTHORIZATION, API_BASIC_AUTH_HEADER)
        .header(header::USER_AGENT, API_USER_AGENT)
        .header(header::ACCEPT, "application/json")
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .form(&data)
        .send()
        .context(ErrorKind::N26AuthenticateNew)?;

    let body = res.text().context(ErrorKind::N26AuthenticateNew)?;
    debug!("{}", body);

    if res.status() != 403 {
        Err(ErrorKind::N26AuthenticateNew)?
    } else {
        let data: MFAToken = serde_json::from_str(&body)
            .with_context(|e| ErrorKind::N26AuthenticateNewParse(e.to_string()))?;

        if data.error != "mfa_required" {
            Err(ErrorKind::N26AuthenticateNew)?
        } else {
            request_mfa_approval(data.mfa_token)
        }
    }
}

fn refresh_authenticate(
    username: String,
    password: String,
    refresh_token: Option<String>,
) -> Result<N26> {
    info!("Calling refresh_authenticate");
    debug!("refresh_token is: {:?}", refresh_token);

    let client = reqwest::Client::new();

    let n26 = if let Some(token) = refresh_token {
        let mut data = HashMap::new();
        data.insert("grant_type", "refresh_token");
        data.insert("refresh_token", token.as_str());
        debug!("{}", token);

        let url = format!("{}/oauth/token", API_URL);
        let mut res = client
            .post(&url)
            .header(header::AUTHORIZATION, API_BASIC_AUTH_HEADER)
            .header(header::USER_AGENT, API_USER_AGENT)
            .header(header::ACCEPT, "application/json")
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .form(&data)
            .send()
            .context(ErrorKind::N26AuthenticateRefreshToken)?;

        let body = res.text().context(ErrorKind::N26AuthenticateRefreshToken)?;
        debug!("{}", body);

        if res.status() != 403 {
            let data: TokenData = serde_json::from_str(&body)
                .with_context(|e| ErrorKind::N26AuthenticateRefreshTokenParse(e.to_string()))?;
            N26 {
                expiration_time: Utc::now().timestamp() + data.expires_in,
                access_token: data.access_token.clone(),
                refresh_token: data.refresh_token.clone(),
            }
        } else {
            new_authenticate(username, password)?
        }
    } else {
        new_authenticate(username, password)?
    };

    // save token to file
    let mut config_file = cache_dir().unwrap_or(current_dir().context(ErrorKind::CurrentDir)?);
    config_file.push("ynab-sync-token-data.json");
    info!("Cache token file is: {}", config_file.to_string_lossy());

    let config_file_content =
        serde_json::to_string(&n26).context(ErrorKind::N26WritingToTokenFile)?;

    write(config_file, config_file_content).context(ErrorKind::N26WritingToTokenFile)?;

    Ok(n26)
}

impl N26 {
    pub fn new(username: String, password: String) -> Result<Self> {
        let mut config_file = cache_dir().unwrap_or(current_dir().context(ErrorKind::CurrentDir)?);
        config_file.push("ynab-sync-token-data.json");
        info!("Cache token file is: {}", config_file.to_string_lossy());

        let n26 = if config_file.exists() {
            let n26_string =
                read_to_string(config_file).context(ErrorKind::N26TokenDataFileCanNotRead)?;
            let n26: N26 = serde_json::from_str(&n26_string)
                .context(ErrorKind::N26TokenDataFileCanNotParse)?;

            if n26.is_valid() {
                info!("Using token from file");
                n26
            } else {
                refresh_authenticate(username, password, Some(n26.refresh_token))?
            }
        } else {
            refresh_authenticate(username, password, None)?
        };

        Ok(n26)
    }

    pub fn is_valid(self: &Self) -> bool {
        Utc::now().timestamp() < self.expiration_time
    }

    pub fn get_categories(self: &Self) -> Result<HashMap<String, String>> {
        let url = format!("{}/api/smrt/categories", API_URL);

        let client = reqwest::Client::new();
        let authorization = format!("Bearer {}", self.access_token);
        let mut res = client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .send()
            .context(ErrorKind::N26GetCategories)?;

        let body = res.text().context(ErrorKind::N26GetCategories)?;
        debug!("{}", body);

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
        let days_ago = now - Duration::days(days);

        // `from` and `to` have to be used together.
        let from = days_ago.timestamp_millis();
        let to = now.timestamp_millis();
        let url = format!(
            "{}/api/smrt/transactions?from={}&to={}&limit={}",
            API_URL, from, to, limit
        );

        let client = reqwest::Client::new();
        let authorization = format!("Bearer {}", self.access_token);
        let mut res = client
            .get(&url)
            .header(header::AUTHORIZATION, authorization)
            .send()
            .context(ErrorKind::N26GetTransactions)?;

        let body = res.text().context(ErrorKind::N26GetTransactions)?;
        debug!("{}", body);

        if !res.status().is_success() {
            let http_error = ErrorKind::N26GetTransactionsHttp(res.status().as_u16(), body.clone());
            Err(http_error)?;
        }

        let transactions = serde_json::from_str(&body)
            .with_context(|e| ErrorKind::N26GetTransactionsParse(e.to_string()))?;

        Ok(transactions)
    }
}
