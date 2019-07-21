use crate::{convert_to_int_eu_style, convert_to_local_date};
use crate::{ErrorKind, Result};
use chrono::{NaiveDate, Utc};
use csv::ReaderBuilder;
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use failure::ResultExt;
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Clone, PartialEq, Debug, Deserialize)]
pub struct Transaction {
    #[serde(deserialize_with = "convert_to_local_date")]
    pub ts: NaiveDate,
    #[serde(deserialize_with = "convert_to_local_date")]
    pub currency_ts: NaiveDate,
    pub entity: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub memo: String,
    #[serde(deserialize_with = "convert_to_int_eu_style")]
    pub balance: i32,
    pub balance_currency: String,
    #[serde(deserialize_with = "convert_to_int_eu_style")]
    pub amount: i32,
    pub amount_currency: String,
}

pub struct IngDiBa {
    pub transactions: Vec<Transaction>,
    pub days_to_sync: i64,
}

impl IngDiBa {
    pub fn new(csv_file: String) -> Result<Self> {
        let mut csv: Vec<String> = vec![];
        let reader = BufReader::new(
            DecodeReaderBytesBuilder::new()
                .encoding(Some(WINDOWS_1252))
                .build(
                    File::open(&csv_file)
                        .context(ErrorKind::IngDiBaCsvFileCanNotOpen(csv_file.clone()))?,
                ),
        );
        for rline in reader.lines() {
            let line = rline.context(ErrorKind::IngDiBaCsvFileParse(csv_file.clone()))?;
            if (csv.is_empty() && line != "" && line.starts_with("Buchung")) || !csv.is_empty() {
                csv.push(line.clone());
            }
        }

        csv.remove(0);
        csv.insert(
            0,
            "ts;currency_ts;entity;type;memo;balance;balance_currency;amount;amount_currency"
                .to_string(),
        );

        let csv_data = csv.join("\n");
        let mut reader = ReaderBuilder::new()
            .delimiter(b';')
            .from_reader(csv_data.as_bytes());
        let mut transactions = vec![];
        for result in reader.deserialize() {
            let transaction: Transaction =
                result.context(ErrorKind::IngDiBaCsvFileParse(csv_file.clone()))?;
            transactions.push(transaction);
        }

        transactions.sort_by_key(|x| x.ts);
        transactions.reverse();
        let today = Utc::today().naive_local();
        let days_to_sync = transactions
            .last()
            .map(|x| NaiveDate::signed_duration_since(today, x.ts).num_days())
            .unwrap_or(0);

        Ok(IngDiBa {
            transactions,
            days_to_sync,
        })
    }
}
