// use chrono::prelude::*;
use money_maker::{draw_candles, read_csv};
use std::io;
use tokio;

#[tokio::main]
async fn main() -> io::Result<()> {
    // let security = "MOEX";
    // let date = chrono::Utc
    //     .with_ymd_and_hms(2025, 3, 3, 0, 0, 0)
    //     .unwrap()
    //     .format("%Y-%m-%d");
    // let interval: u8 = 10;
    // let url = format!("https://iss.moex.com/iss/engines/stock/markets/shares/securities/{security}/candles.csv?from={date}&till={date}&interval={interval}");
    // download(&url).await?;

    let candles = read_csv("data/iss_moex/moex.csv").await;
    // println!("{:?}", candles);
    draw_candles(candles).await;

    Ok(())
}
