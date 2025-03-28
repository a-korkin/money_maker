// use chrono::prelude::*;
use money_maker::insert_candles; //{draw_graphs, run};
mod models;
// use models::common::DateRange;
mod db;
use db::pg;
mod utils;
// use log::info;
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

    // let start = Local::now().time();
    // let date_range = DateRange(
    //     chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
    //     chrono::Utc.with_ymd_and_hms(2025, 3, 26, 0, 0, 0).unwrap(),
    // );
    //
    // let securities: Vec<&str> = vec!["LKOH"];
    // for date in date_range {
    //     let _ = run(&securities, date).await;
    // }
    // let end = Local::now().time();
    // info!("elapsed time: {}", elapsed_time(start, end));
    // Ok(())

    // draw_graphs("MOEX").await

    let pool = pg::init_db().await;
    // let secs = vec!["MOEX", "LKOH"];
    // pg::add_securities(&pool, &secs).await;

    insert_candles(&pool, "MOEX").await;
    Ok(())
}
