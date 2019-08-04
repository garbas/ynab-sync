use crate::{ErrorKind, Result};
use crate::ynab::{Transaction, Category};
use failure::ResultExt;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fmt;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::result;
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "rule")]
enum Rules {
    Contains {
        value: String,
        #[serde(with = "serde_str")]
        field: TransactionField,
        category: String,
    },
    StartsWith {
        value: String,
        #[serde(with = "serde_str")]
        field: TransactionField,
        category: String,
    },
    EndsWith {
        value: String,
        #[serde(with = "serde_str")]
        field: TransactionField,
        category: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
enum TransactionField {
    Memo,
    Payee,
}

impl fmt::Display for TransactionField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                TransactionField::Memo => "memo",
                TransactionField::Payee => "payee",
            },
        )
    }
}

impl FromStr for TransactionField {
    type Err = ErrorKind;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s {
            "memo" => Ok(TransactionField::Memo),
            "payee" => Ok(TransactionField::Payee),
            _ => Err(ErrorKind::YNABAccountTypeParse),
        }
    }
}

fn read_rules(category_rules_file: String) -> Result<Vec<Rules>> {
    // check if --category-rules file exists and that it is of JSON format
    if !PathBuf::from(category_rules_file.clone()).exists() {
        Err(ErrorKind::ArgParseCategoryRulesCanNotRead(
            category_rules_file.clone(),
        ))?
    }
    let category_rules_string =
        read_to_string(category_rules_file.to_string()).with_context(|_| {
            ErrorKind::ArgParseCategoryRulesCanNotRead(category_rules_file.clone())
        })?;
    serde_json::from_str(&category_rules_string).context(
        ErrorKind::ArgParseCategoryRulesCanNotParse(category_rules_file.clone()),
    )?
}

fn apply_rules(rules: Vec<Rules>, categories: HashMap<String, Category>, transaction: Transaction) -> Option<Category> {
    let memo = transaction.clone().memo.unwrap_or("".to_string());
    let payee = transaction.clone().payee_name.unwrap_or("".to_string());
    for rule in &rules {
        match rule {
            Rules::Contains {
                value,
                field,
                category,
            } => {
                let text = match field {
                    TransactionField::Memo => &memo,
                    TransactionField::Payee => &payee,
                };
                if text.to_lowercase().contains(&value.to_lowercase()) {
                    return categories.get(category).cloned();
                }
            }
            Rules::StartsWith {
                value,
                field,
                category,
            } => {
                let text = match field {
                    TransactionField::Memo => &memo,
                    TransactionField::Payee => &payee,
                };
                if text.to_lowercase().starts_with(&value.to_lowercase()) {
                    return categories.get(category).cloned();
                }
            }
            Rules::EndsWith {
                value,
                field,
                category,
            } => {
                let text = match field {
                    TransactionField::Memo => &memo,
                    TransactionField::Payee => &payee,
                };
                if text.to_lowercase().ends_with(&value.to_lowercase()) {
                    return categories.get(category).cloned();
                }
            }
        }
    };
    None
}
