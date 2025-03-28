use chrono::prelude::*;
use chrono::Duration;
use csv;
use dotenv;
use log::{error, info};
use plotters::prelude::*;
use reqwest;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use std::thread;
use std::time;

mod models;
use models::common::Candle;
mod db;
mod utils;
use db::pg::add_candles;
use sqlx::postgres::PgPool;

pub async fn run(securities: &Vec<&str>, date: DateTime<Utc>) -> std::io::Result<()> {
    let date = date.format("%Y-%m-%d");
    let interval: u8 = 1;
    let iss_moex = dotenv::var("ISS_MOEX").expect("failed to read ISS_MOEX");

    for security in securities {
        let mut start: u32 = 0;
        let mut i = 1;
        loop {
            let url = format!(
                "{iss_moex}/{security}/candles.csv?from={date}\
                &till={date}&interval={interval}&start={start}"
            );
            let file_name = &format!("{date}_{i}.csv");
            match download(&url, security, file_name).await {
                Ok(added) => {
                    if added < 0 {
                        break;
                    }
                    start += added as u32;
                    i += 1;
                    info!("{security} => {file_name}, count => {added}");
                    thread::sleep(time::Duration::from_millis(500));
                }
                Err(e) => {
                    error!("{}", e);
                    break;
                }
            }
        }
    }

    Ok(())
}

pub fn elapsed_time(start: NaiveTime, end: NaiveTime) -> String {
    let diff = end - start;
    format!(
        "{:0>2}:{:0>2}:{:0>2}",
        diff.num_hours(),
        diff.num_minutes() % 60,
        diff.num_seconds() % 60
    )
}

pub async fn draw_graphs(security: &str) -> std::io::Result<()> {
    let data_dir = dotenv::var("DATA_DIR").expect("failed to read DATA_DIR");

    let path = Path::new(&data_dir)
        .join(security)
        .to_str()
        .unwrap()
        .to_owned();
    for entry in fs::read_dir(path).unwrap() {
        let file = entry.unwrap();
        let file_type = file.file_type().unwrap();

        if file_type.is_file() {
            let candles =
                get_candles_from_csv(file.path().to_str().expect("failed to get filepath")).await;
            draw_candles(candles, security, file.file_name().to_str().unwrap()).await;
        }
    }

    Ok(())
}

pub async fn download(url: &str, security: &str, file_name: &str) -> std::io::Result<i64> {
    let response = reqwest::get(url).await.expect("failed to send request");
    if response.status() != 200 {
        error!("response status: {}", response.status());
        return Ok(-1);
    }
    let rows = response
        .text()
        .await
        .expect("failed to get body")
        .split("\n")
        .skip(2)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let count = rows.len();
    if count <= 1 {
        return Ok(-1i64);
    }

    let path = &dotenv::var("DATA_DIR").expect("failed to get DATA_DIR");
    let path = Path::new(path).join(security);

    if !fs::exists(&path)? {
        fs::create_dir_all(&path)?;
    }
    let file_path = Path::new(&path.to_str().unwrap()).join(file_name);
    let mut file = fs::File::create(file_path)?;
    file.write_all(rows.join("\n").as_bytes())?;

    Ok((count - 1) as i64)
}

pub async fn get_candles_from_csv(path: &str) -> Vec<Candle> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(path)
        .expect("failed to read csv");

    rdr.deserialize::<Candle>()
        .map(|c| c.unwrap())
        .collect::<Vec<_>>()
}

pub async fn insert_candles(pool: &PgPool, security: &str) {
    let candles = get_candles_from_csv("data/iss_moex/MOEX/2025-03-03_1.csv").await;
    add_candles(pool, security, &candles).await;
}

pub async fn draw_candles(candles: Vec<Candle>, security: &str, file_name: &str) {
    let path = dotenv::var("GRAPHS_DIR").expect("failed to read GRAPHS_DIR");
    let dir = Path::new(&path).join(security);
    if !fs::exists(&dir).unwrap() {
        fs::create_dir_all(&dir).unwrap();
    }
    let file_name = file_name.replace(".csv", ".png");
    let file = Path::new(&dir).join(file_name);
    let root = BitMapBackend::new(&file, (1024, 768)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let to_date = candles.last().unwrap().end + Duration::minutes(10);
    let from_date = candles[0].end - Duration::minutes(10);

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption(security, ("sans-serif", 50.0).into_font())
        .build_cartesian_2d(RangedDateTime::from(from_date..to_date), 210f32..225f32)
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    let candles = candles.iter().map(|x| {
        CandleStick::new(
            x.begin,
            x.open,
            x.high,
            x.low,
            x.close,
            GREEN.filled(),
            RED.filled(),
            7,
        )
    });

    chart.draw_series(candles).unwrap();
    root.present().unwrap();
}
