use clap_log_flag::Log;
use clap_verbosity_flag::Verbosity;
use crypto::digest::Digest;
use crypto::sha1::Sha1;
use failure::ResultExt;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::result;
use std::str::FromStr;
use structopt::StructOpt;
use ynab_sync::error::{ErrorKind, Result};
use ynab_sync::ingdiba::{IngDiBa, Transaction as IngDiBaTransaction};
use ynab_sync::ynab::{
    Category, Cli as YNABCli, Transaction as YNABTransaction, TransactionCleared, YNAB,
};

#[derive(StructOpt, Debug)]
struct Cli {
    #[structopt(flatten)]
    verbose: Verbosity,
    #[structopt(flatten)]
    log: Log,
    #[structopt(flatten)]
    ynab: YNABCli,
    #[structopt(
        long = "category-rules",
        required = true,
        value_name = "FILE",
        help = "JSON file which represents mapping rules between IngDiba and YNAB categories."
    )]
    category_rules_file: String,
    #[structopt(
        long = "csv",
        required = true,
        value_name = "FILE",
        help = "CSV file which you exported from Ing-DiBa."
    )]
    csv_file: String,
}

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
    Entity,
}

impl fmt::Display for TransactionField {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                TransactionField::Memo => "memo",
                TransactionField::Entity => "entity",
            },
        )
    }
}

impl FromStr for TransactionField {
    type Err = ErrorKind;

    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s {
            "memo" => Ok(TransactionField::Memo),
            "entity" => Ok(TransactionField::Entity),
            _ => Err(ErrorKind::YNABAccountTypeParse),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::from_args();
    cli.log.log_all(Some(cli.verbose.log_level()))?;

    // check if --category-rules file exists and that it is of JSON format
    if !PathBuf::from(cli.category_rules_file.clone()).exists() {
        Err(ErrorKind::ArgParseCategoryRulesCanNotRead(
            cli.csv_file.clone(),
        ))?
    }
    let category_rules_string =
        read_to_string(cli.category_rules_file.to_string()).with_context(|_| {
            ErrorKind::ArgParseCategoryRulesCanNotRead(cli.category_rules_file.clone())
        })?;
    let rules: Vec<Rules> = serde_json::from_str(category_rules_string.as_str()).context(
        ErrorKind::ArgParseCategoryRulesCanNotParse(cli.category_rules_file.clone()),
    )?;

    println!("[1/7] Parsing --csv file");
    let ingdiba = IngDiBa::new(cli.csv_file)?;

    // YNAB client
    let ynab = YNAB {
        token: cli.ynab.token.clone(),
    };

    // validate ynab cli options
    ynab.validate_cli(cli.ynab.clone(), 1, 7)?;

    // Fetch YNAB categories
    println!("[4/7] Fetching YNAB categories");
    let ynab_categories = ynab.get_categories(cli.ynab.budget_id.clone())?;

    // Fetch ynab transactions
    println!(
        "[5/7] Fetching YNAB transactions for the last {} days",
        ingdiba.days_to_sync
    );
    let ynab_transactions = ynab.get_transactions(
        cli.ynab.budget_id.clone(),
        cli.ynab.account_id.clone(),
        ingdiba.days_to_sync,
    )?;

    let apply_rules = |transaction: &IngDiBaTransaction| -> Option<Category> {
        for rule in &rules {
            match rule {
                Rules::Contains {
                    value,
                    field,
                    category,
                } => {
                    let text = match field {
                        TransactionField::Memo => &transaction.memo,
                        TransactionField::Entity => &transaction.entity,
                    };
                    if text.to_lowercase().contains(&value.to_lowercase()) {
                        return ynab_categories.get(category).cloned();
                    }
                }
                Rules::StartsWith {
                    value,
                    field,
                    category,
                } => {
                    let text = match field {
                        TransactionField::Memo => &transaction.memo,
                        TransactionField::Entity => &transaction.entity,
                    };
                    if text.to_lowercase().starts_with(&value.to_lowercase()) {
                        return ynab_categories.get(category).cloned();
                    }
                }
                Rules::EndsWith {
                    value,
                    field,
                    category,
                } => {
                    let text = match field {
                        TransactionField::Memo => &transaction.memo,
                        TransactionField::Entity => &transaction.entity,
                    };
                    if text.to_lowercase().ends_with(&value.to_lowercase()) {
                        return ynab_categories.get(category).cloned();
                    }
                }
            }
        }
        None
    };

    let convert_transaction =
        |account_id: &str, transaction: &IngDiBaTransaction| -> YNABTransaction {
            // apply category rules
            let category: Option<String> = apply_rules(transaction).map(|x| x.id);

            // when we can not figure out category we mark transaction as not approved
            let approved = category.is_some();

            // XXX: we can probably find more idiomatic way of doing this
            let memo = format!(
                "{} :: {}",
                transaction.entity.clone(),
                transaction.memo.clone()
            );

            let date = transaction.ts.format("%Y-%m-%d").to_string();

            let mut import_id_sha = Sha1::new();
            import_id_sha.input_str(&date);
            import_id_sha.input_str(&format!("{}", transaction.amount));
            import_id_sha.input_str(&memo);
            let import_id = import_id_sha.result_str()[..36].to_string();

            YNABTransaction {
                account_id: account_id.to_string(),
                date,
                amount: transaction.amount,
                // TODO: we would need to have payee_mapping
                payee_id: None,
                payee_name: None,
                category_id: category,
                memo: Some(memo),
                cleared: TransactionCleared::Cleared,
                approved,
                flag_color: None,
                import_id: Some(import_id),
            }
        };

    println!("[6/7] Convert IngDiBa transactions to YNAB transactions");
    let account_id = cli.ynab.account_id.as_str();
    let transactions: Vec<YNABTransaction> = ingdiba
        .transactions
        .into_iter()
        .map(|t| convert_transaction(account_id, &t))
        .collect();

    ynab.sync(
        transactions,
        ynab_transactions,
        cli.ynab.budget_id,
        cli.ynab.force_update,
        6,
        7,
    )?;

    Ok(())
}
