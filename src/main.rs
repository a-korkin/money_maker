use chrono::prelude::*;
use dotenv;
use money_maker::{download, draw_candles, read_csv};
use std::io;
use tokio;

#[tokio::main]
async fn main() -> io::Result<()> {
    dotenv::dotenv().ok();
    let security = "MOEX";
    let date = chrono::Utc
        .with_ymd_and_hms(2025, 3, 3, 0, 0, 0)
        .unwrap()
        .format("%Y-%m-%d");
    let interval: u8 = 10;
    let url = format!("https://iss.moex.com/iss/engines/stock/markets/shares/securities/{security}/candles.csv?from={date}&till={date}&interval={interval}");

    download(&url).await?;

    let data_dir = dotenv::var("DATA_DIR").expect("failed to read DATA_DIR");
    let candles = read_csv(&format!("{data_dir}/moex.csv")).await;
    draw_candles(candles).await;

    Ok(())
}
