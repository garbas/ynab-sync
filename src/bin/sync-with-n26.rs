#[macro_use]
extern crate clap;
extern crate ynab_sync;

use clap::{App, Arg};
use failure::ResultExt;
use serde_json;
use std::collections::HashMap;
use std::fs::read_to_string;
use ynab_sync::n26::Transaction as N26Transaction;
use ynab_sync::ynab::{Transaction as YNABTransaction, TransactionCleared};
use ynab_sync::{ErrorKind, Result, N26, YNAB};
use std::iter::FromIterator;

// TODO: switch to structopt
// TODO: provide nicer output using ascii_term, use console crate to prirnt colored output
// TODO: review error handling and use failure and exitfailure, understand current error module
// TODO: use progress bar when making http calls (https://mattgathu.github.io/writing-cli-app-rust/)
fn main() -> Result<()> {
    let app = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .arg(
            Arg::with_name("ynab_token")
                .long("ynab-token")
                .value_name("TEXT")
                .env("YNAB_TOKEN")
                .required(true),
        )
        .arg(
            Arg::with_name("ynab_account_id")
                .required(true)
                .long("ynab-account-id")
                .value_name("TEXT")
                .env("YNAB_ACCOUNT_ID"),
        )
        .arg(
            Arg::with_name("ynab_budget_id")
                .required(true)
                .long("ynab-budget-id")
                .value_name("TEXT")
                .env("YNAB_BUDGET_ID"),
        )
        .arg(
            Arg::with_name("n26_username")
                .required(true)
                .long("n26-username")
                .value_name("TEXT")
                .env("N26_USERNAME"),
        )
        .arg(
            Arg::with_name("n26_password")
                .required(true)
                .long("n26-password")
                .value_name("TEXT")
                .env("N26_PASSWORD"),
        )
        .arg(
            Arg::with_name("category_mapping_file")
                .required(true)
                .long("category-mapping")
                .value_name("FILE"),
        )
        .arg(
            Arg::with_name("days_to_sync")
                .required(true)
                .long("days-to-sync")
                .value_name("INT"),
        )
        .get_matches();

    // Validate that days_to_sync is correct
    let days_to_sync_as_str = app.value_of("days_to_sync").unwrap_or("30");
    let days_to_sync = match days_to_sync_as_str.parse::<i64>() {
        Ok(d) => d,
        Err(e) => Err(ErrorKind::ArgParseDaysToSync(
            days_to_sync_as_str.to_string(),
            e.to_string(),
        ))?,
    };

    // Calling .unwrap() is safe here because an argument should be required.
    let get_required_arg = |arg_name| app.value_of(arg_name).unwrap().to_string();
    let n26_username = get_required_arg("n26_username");
    let n26_password = get_required_arg("n26_password");
    let ynab_token = get_required_arg("ynab_token");
    let ynab_budget_id = get_required_arg("ynab_budget_id");
    let ynab_account_id = get_required_arg("ynab_account_id");

    // create both clients
    let n26 = N26::new(n26_username, n26_password)?;
    let ynab = YNAB {
        token: ynab_token,
        budget_id: ynab_budget_id,
        account_id: ynab_account_id.clone(),
    };

    // Validate that category_mapping_file file exists and that it is of JSON format
    let category_mapping_file = get_required_arg("category_mapping_file");
    let category_mapping_string =
        read_to_string(category_mapping_file.to_string()).with_context(|_| {
            ErrorKind::ArgParseCategoryMappingCanNotRead(category_mapping_file.clone())
        })?;
    let category_mapping_value: serde_json::Value =
        serde_json::from_str(category_mapping_string.as_str()).with_context(|_| {
            ErrorKind::ArgParseCategoryMappingCanNotParse(category_mapping_file.clone())
        })?;

    let category_mapping = match category_mapping_value.as_object() {
        Some(x) => x,
        None => {
            return Err(ynab_sync::Error::from(
                ErrorKind::ArgParseCategoryMappingCanNotParse(category_mapping_file.clone()),
            ))
        }
    };
    println!("{:?}", category_mapping);

    // Fetch n26 categories
    let n26_categories = n26.get_categories()?;
    println!("{:?}", n26_categories);

    // TODO: verify that budget_id are correct account_id

    // Fetch n26 categories
    let ynab_transactions: HashMap<String, YNABTransaction> = HashMap::from_iter(
        ynab.get_transactions()
            .iter()
            .filter(|x| x.import_id.is_some())
            .map(|x| (x.import_id.clone().unwrap_or(String::from("")), x.clone()))
    );

    let convert_transaction = |transaction: &N26Transaction| -> YNABTransaction {
        // find category in category_mapping
        let category = n26_categories
            .get(&transaction.category)
            .and_then(|x| category_mapping.get(x))
            .and_then(|x| x.as_str())
            .map(String::from);

        // when we can not figure out category we mark transaction as not approved
        let approved = category.is_some();

        // XXX: we can probably find more idiomatic way of doing this
        let memo = match &transaction.reference_text {
            Some(reference_text) => Some(format!("{}", reference_text)),
            None => match &transaction.merchant_name {
                Some(merchant_name) => match &transaction.merchant_city {
                    Some(merchant_city) => Some(format!("{} {}", merchant_name, merchant_city)),
                    None => Some(format!("{}", merchant_name)),
                },
                None => None,
            },
        };

        YNABTransaction {
            account_id: ynab_account_id.to_string(),
            date: transaction.visible_ts.format("%Y-%m-%d").to_string(),
            amount: transaction.amount,
            // TODO: we would need to have payee_mapping
            payee_id: None,
            payee_name: None,
            category_id: category,
            memo,
            cleared: TransactionCleared::Cleared,
            approved,
            flag_color: None,
            import_id: Some(transaction.id.clone()),
        }
    };

    let transactions: Vec<YNABTransaction> = n26
        .get_transactions(days_to_sync, 100000000)? // XXX: for now we set limit to 1mio
        .into_iter()
        .map(|t| convert_transaction(&t))
        .collect();

    // figure out which transactions are new and which we need to update
    let mut new_transactions: Vec<YNABTransaction> = vec![];
    let mut update_transactions: Vec<YNABTransaction> = vec![];
    for transaction in transactions.iter() {
        match transaction.import_id.clone() {
            Some(import_id) => {
                // filter out transactions that don't need to be updated
                // that means if import_id matches amount and date should
                // be the same as in n26 transaction
                let ynab_transaction = ynab_transactions.get(&import_id);
                if ynab_transaction.map(|x| x.amount) == Some(transaction.amount) &&
                   ynab_transaction.map(|x| x.date.clone()) == Some(transaction.date.clone()) {
                  continue;
                }
                if ynab_transactions.contains_key(import_id.as_str()) {
                    update_transactions.push(transaction.clone());
                } else {
                    new_transactions.push(transaction.clone());
                }
            },
            None => {},
        };
    }

    println!("New transactions: {:?}", new_transactions.len());
    println!("Transactions to update: {:?}", update_transactions.len());

    // TODO: test on testing budget_id and account_id
    //let new_transactions = ynab.save_transactions(transactions);

    Ok(())
}
