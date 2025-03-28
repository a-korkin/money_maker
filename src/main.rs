use chrono::prelude::*;
use money_maker::{insert_candles, run};
mod models;
// use models::common::DateRange;
mod db;
use db::pg;
mod utils;
// use log::info;
use clap::Parser;
use models::common::DateRange;
use std::io::Result;
use tokio;
use utils::logger;

/// Money maker app
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// List of securities separated by comma
    #[arg(short, long)]
    secs: String,

    /// Download csv files
    #[arg(short, long)]
    download: bool,

    // Adding to DB
    #[arg(short, long)]
    add: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    logger::init().expect("failed to init logging");

    let args = Args::parse();

    let securities = args
        .secs
        .split(",")
        .map(|s| s.trim().to_uppercase().to_owned())
        .collect::<Vec<String>>();

    let start: DateTime<Utc> = dotenv::var("PERIOD_START")
        .expect("failed to get PERIOD_START")
        .parse::<NaiveDate>()
        .expect("failed parse to DateTime")
        .and_time(NaiveTime::default())
        .and_local_timezone(Utc)
        .unwrap();

    let end: DateTime<Utc> = dotenv::var("PERIOD_END")
        .expect("failed to get PERIOD_END")
        .parse::<NaiveDate>()
        .expect("failed parse to DateTime")
        .and_time(NaiveTime::default())
        .and_local_timezone(Utc)
        .unwrap();

    let date_range = DateRange(start, end);

    if args.download {
        for date in date_range {
            let _ = run(&securities, date).await;
        }
    }

    if args.add {
        let pool = pg::init_db().await;
        pg::add_securities(&pool, &securities).await;

        for security in securities {
            insert_candles(&pool, &security).await;
        }
    }

    Ok(())
}
