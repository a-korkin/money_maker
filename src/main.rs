use chrono::prelude::*;
use money_maker::run;
use std::io::Result;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    let securities: Vec<&str> = vec!["MOEX", "LKOH"];
    let date = chrono::Utc.with_ymd_and_hms(2025, 3, 3, 0, 0, 0).unwrap();
    run(securities, date).await
}
