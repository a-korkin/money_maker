use chrono::prelude::*;
use money_maker::{draw_graphs, run};
mod models;
use models::common::DateRange;
mod utils;
use std::io::Result;
use tokio;
use utils::logger;

#[tokio::main]
async fn main() -> Result<()> {
    logger::init().expect("failed to init logging");
    // let securities: Vec<&str> = vec!["MOEX", "LKOH"];
    // let date = chrono::Utc.with_ymd_and_hms(2025, 3, 3, 0, 0, 0).unwrap();
    // let result = run(securities, date).await;
    // return result;

    let date_range = DateRange(
        chrono::Utc.with_ymd_and_hms(2025, 2, 1, 0, 0, 0).unwrap(),
        chrono::Utc.with_ymd_and_hms(2025, 3, 1, 0, 0, 0).unwrap(),
    );

    for date in date_range {
        println!("{:?}", date);
    }

    Ok(())

    // draw_graphs("MOEX").await
}
