use chrono::prelude::*;
use dotenv;
use money_maker::{download, draw_candles, read_csv};
use std::{io, path::Path};
use tokio;

#[tokio::main]
async fn main() -> io::Result<()> {
    dotenv::dotenv().ok();
    let security = "MOEX";
    // let date = chrono::Utc
    //     .with_ymd_and_hms(2025, 3, 3, 0, 0, 0)
    //     .unwrap()
    //     .format("%Y-%m-%d");
    // let interval: u8 = 1;
    // let start: u32 = 100;
    // let iss_moex = dotenv::var("ISS_MOEX").expect("failed to read ISS_MOEX");
    // let url =
    //     format!("{iss_moex}/{security}/candles.csv?from={date}&till={date}&interval={interval}&start={start}");
    // download(&url, security).await?;

    let data_dir = dotenv::var("DATA_DIR").expect("failed to read DATA_DIR");
    let candles = read_csv(
        Path::new(&data_dir)
            .join(security)
            .join("moex2.csv")
            .to_str()
            .unwrap(),
    )
    .await;
    draw_candles(candles).await;

    Ok(())
}
