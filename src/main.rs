use chrono::prelude::*;
use money_maker::run;
mod models;
use models::common::DateRange;
use std::io::Result;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    let securities: Vec<&str> = vec!["MOEX", "LKOH"];
    let date = chrono::Utc.with_ymd_and_hms(2025, 3, 3, 0, 0, 0).unwrap();
    let _result = run(securities, date).await;
    // return result;

    let date_range = DateRange(
        chrono::Utc.with_ymd_and_hms(2025, 2, 1, 0, 0, 0).unwrap(),
        chrono::Utc.with_ymd_and_hms(2025, 3, 1, 0, 0, 0).unwrap(),
    );

    for date in date_range {
        println!("{:?}", date);
    }

    Ok(())
}
