use clap_log_flag::Log;
use clap_verbosity_flag::Verbosity;
use failure::ResultExt;
use serde_json;
use std::fs::read_to_string;
use std::path::PathBuf;
use structopt::StructOpt;
use ynab_sync::n26::{Cli as N26Cli, Transaction as N26Transaction};
use ynab_sync::ynab::{Cli as YNABCli, Transaction as YNABTransaction, TransactionCleared};
use ynab_sync::{ErrorKind, Result, N26, YNAB};

#[derive(StructOpt, Debug)]
struct Cli {
    #[structopt(flatten)]
    verbose: Verbosity,
    #[structopt(flatten)]
    log: Log,
    #[structopt(flatten)]
    ynab: YNABCli,
    #[structopt(flatten)]
    n26: N26Cli,
    #[structopt(
        long = "category-mapping",
        required = true,
        value_name = "FILE",
        help = "JSON file which represents the mapping between N26 and YNAB category."
    )]
    category_mapping_file: String,
    #[structopt(
        long = "days-to-sync",
        required = true,
        value_name = "INT",
        help = "Number of the past days that you want to sync from."
    )]
    days_to_sync: i32,
}

fn main() -> Result<()> {
    let cli = Cli::from_args();
    cli.log.log_all(Some(cli.verbose.log_level()))?;

    //
    // Validate that category_mapping_file file exists and that it is of JSON format
    //
    println!("[1/9] Parsing --category-mapping-file");

    if !PathBuf::from(cli.category_mapping_file.clone()).exists() {
        Err(ErrorKind::ArgParseCategoryMappingCanNotRead(
            cli.category_mapping_file.clone(),
        ))?
    }

    let category_mapping_string = read_to_string(cli.category_mapping_file.to_string())
        .with_context(|_| {
            ErrorKind::ArgParseCategoryMappingCanNotRead(cli.category_mapping_file.clone())
        })?;
    let category_mapping_value: serde_json::Value =
        serde_json::from_str(category_mapping_string.as_str()).context(
            ErrorKind::ArgParseCategoryMappingCanNotParse(cli.category_mapping_file.clone()),
        )?;

    let category_mapping = match category_mapping_value.as_object() {
        Some(x) => x,
        None => Err(ErrorKind::ArgParseCategoryMappingCanNotParse(
            cli.category_mapping_file.clone(),
        ))?,
    };

    // YNAB client
    let ynab = YNAB {
        token: cli.ynab.token.clone(),
    };

    // validate ynab cli options
    ynab.validate_cli(cli.ynab.clone(), 1, 9)?;

    // Fetch YNAB categories
    println!("[4/9] Fetching YNAB categories");
    let ynab_categories = ynab.get_categories(cli.ynab.budget_id.clone())?;

    // Fetch ynab transactions
    println!(
        "[5/9] Fetching YNAB transactions for the last {} days",
        cli.days_to_sync
    );
    let ynab_transactions = ynab.get_transactions(
        cli.ynab.budget_id.clone(),
        cli.ynab.account_id.clone(),
        cli.days_to_sync.into(),
    )?;

    // N26 client
    println!("[6/9] Fetching N26 token");
    let n26 = N26::new(cli.n26.username.clone(), cli.n26.password.clone())?;

    // Fetch n26 categories
    println!("[7/9] Fetching N26 categories");
    let n26_categories = n26.get_categories()?;

    let convert_transaction = |transaction: &N26Transaction| -> YNABTransaction {
        let category: Option<String> = n26_categories
            // select category from transaction
            .get(&transaction.category)
            // find category in category_mapping
            .and_then(|x| category_mapping.get(x))
            .and_then(|x| x.as_str())
            .map(String::from)
            // find id of the category
            .and_then(|x| ynab_categories.get(&x))
            .map(|x| x.clone().id);

        // when we can not figure out category we mark transaction as not approved
        let approved = category.is_some();

        // XXX: we can probably find more idiomatic way of doing this
        let memo = match &transaction.reference_text {
            Some(reference_text) => Some(reference_text.to_string()),
            None => match &transaction.merchant_name {
                Some(merchant_name) => match &transaction.merchant_city {
                    Some(merchant_city) => Some(format!("{} {}", merchant_name, merchant_city)),
                    None => Some(merchant_name.to_string()),
                },
                None => None,
            },
        };

        YNABTransaction {
            account_id: cli.ynab.account_id.clone().to_string(),
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

    println!("[8/9] Fetching N26 transaction and converting them to YNAB transactions");
    let transactions: Vec<YNABTransaction> = n26
        .get_transactions(cli.days_to_sync.into(), 100_000_000)? // XXX: for now we set limit to 1mio
        .into_iter()
        .map(|t| convert_transaction(&t))
        .collect();

    ynab.sync(
        transactions,
        ynab_transactions,
        cli.ynab.budget_id.clone(),
        cli.ynab.force_update,
        8,
        9,
    )?;

    Ok(())
}
