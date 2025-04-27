mod db;
mod models;
mod strategy;
mod terminal;
mod utils;

use chrono::prelude::*;
use chrono::Duration;
use clap::Parser;
use csv;
use db::pg::{add_candles, add_securities, add_trades, get_all_securities, init_db};
use dotenv;
use log::{error, info};
use models::common::{Candle, DateRange, Trade};
use plotters::prelude::*;
use reqwest;
use sqlx::postgres::PgPool;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use std::thread;
use std::time;
use utils::logger;

/// Money maker app
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// List of securities separated by comma
    #[arg(short, long, default_value = "all")]
    secs: String,

    /// Download csv files
    #[arg(short, long, default_value = "none")]
    download: String,

    /// Adding to DB
    #[arg(short, long)]
    add: bool,

    /// Run terminal
    #[arg(short, long)]
    terminal: bool,
}

pub async fn run() {
    logger::init().expect("failed to init logging");
    let pool = init_db().await;

    let args = Args::parse();
    if args.terminal {
        run_terminal(&pool).await;
        return;
    }

    // strategy::strategy::run_strategy(&pool).await;
    // return;

    let securities = match args.secs.as_str() {
        "all" => get_all_securities(&pool).await,
        _ => args
            .secs
            .split(",")
            .map(|s| s.trim().to_uppercase().to_owned())
            .collect::<Vec<String>>(),
    };

    let start: NaiveDateTime = dotenv::var("PERIOD_START")
        .expect("failed to get PERIOD_START")
        .parse::<NaiveDate>()
        .expect("failed parse to DateTime")
        .and_time(NaiveTime::default());

    let end: NaiveDateTime = dotenv::var("PERIOD_END")
        .expect("failed to get PERIOD_END")
        .parse::<NaiveDate>()
        .expect("failed parse to DateTime")
        .and_time(NaiveTime::default());

    let download_type = DownloadType::from(args.download.as_str());

    // match download_type {
    //     DownloadType::Candles => fetch_data(&securities, DownloadType::Candles, start, end).await,
    //     DownloadType::Trades => {
    //         fetch_data(&securities, DownloadType::Trades, start, end).await;
    //     }
    // }

    if args.add {
        add_securities(&pool, &securities).await;
        insert_entity(&pool, download_type, &securities).await;
    }
}

pub async fn run_terminal(pool: &PgPool) {
    terminal::terminal::run_terminal(pool).await;
}

pub enum DownloadType {
    Candles,
    Trades,
}

impl ToString for DownloadType {
    fn to_string(&self) -> String {
        match self {
            Self::Candles => String::from("candles"),
            Self::Trades => String::from("trades"),
        }
    }
}

impl From<&str> for DownloadType {
    fn from(value: &str) -> Self {
        match value {
            "c" | "candles" => Self::Candles,
            "t" | "trades" => Self::Trades,
            _ => unimplemented!(),
        }
    }
}

pub async fn fetch_data(
    securities: &Vec<String>,
    download_type: DownloadType,
    start: NaiveDateTime,
    end: NaiveDateTime,
) {
    let today = Local::now();
    let begin = today.time();
    let iss_moex = dotenv::var("ISS_MOEX").expect("failed to read ISS_MOEX");

    match download_type {
        DownloadType::Candles => {
            let date_range = DateRange(start, end);
            for date in date_range {
                let date = date.format("%Y-%m-%d");
                let interval: u8 = 1;

                for security in securities {
                    let mut start: u32 = 0;
                    let mut i = 1;
                    loop {
                        let url = format!(
                            "{iss_moex}/{security}/candles.csv?from={date}\
                &till={date}&interval={interval}&start={start}"
                        );
                        let file_name = &format!("{date}_{i}.csv");
                        match download(&download_type, &url, security, file_name).await {
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
            }
        }
        DownloadType::Trades => {
            for security in securities {
                let url = format!("{iss_moex}/{security}/trades.csv");
                let today = today.date_naive();
                let file_name = &format!("{today}.csv");
                match download(&download_type, &url, security, file_name).await {
                    Ok(added) => {
                        if added < 0 {
                            break;
                        }
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
    }
    let end = Local::now().time();
    info!("elapsed time: {}", elapsed_time(begin, end));
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

pub async fn download(
    download_type: &DownloadType,
    url: &str,
    security: &str,
    file_name: &str,
) -> std::io::Result<i64> {
    let response = reqwest::get(url).await.expect("failed to send request");
    if response.status() != 200 {
        error!("response status: {}", response.status());
        return Ok(-1);
    }

    let rows = match download_type {
        DownloadType::Trades => response
            .text()
            .await
            .expect("failed to get body")
            .split_once("dataversion")
            .unwrap()
            .0
            .split("\n")
            .skip(2)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<String>>(),
        _ => response
            .text()
            .await
            .expect("failed to get body")
            .split("\n")
            .skip(2)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<String>>(),
    };

    let count = rows.len();
    if count <= 1 {
        return Ok(-1i64);
    }

    let path = &dotenv::var("DATA_DIR").expect("failed to get DATA_DIR");
    let path = Path::new(path)
        .join(download_type.to_string())
        .join(security);

    if !fs::exists(&path)? {
        fs::create_dir_all(&path)?;
    }
    let file_path = Path::new(&path.to_str().unwrap()).join(file_name);
    println!("file_path: {:?}", file_path);
    let mut file = fs::File::create(file_path)?;
    file.write_all(rows.join("\n").as_bytes())?;

    match download_type {
        DownloadType::Trades => return Ok(-1i64),
        _ => return Ok((count - 1) as i64),
    }
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

pub async fn get_trades_from_csv(path: &str) -> Vec<Trade> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(path)
        .expect("failed to read csv");

    rdr.deserialize::<Trade>()
        .map(|c| c.unwrap())
        .collect::<Vec<_>>()
}

async fn insert_entity(pool: &PgPool, download_type: DownloadType, securities: &Vec<String>) {
    let start = Local::now().time();
    for security in securities {
        let data_dir = dotenv::var("DATA_DIR").expect("failed to get DATA_DIR");
        let path = Path::new(&data_dir)
            .join(download_type.to_string())
            .join(security)
            .to_str()
            .unwrap()
            .to_owned();
        for entry in fs::read_dir(path).unwrap() {
            let file = entry.unwrap();
            let file_type = file.file_type().unwrap();

            if file_type.is_file() {
                let mut added: u64 = 0;
                let file_name = file.file_name().to_str().unwrap().to_owned();
                match download_type {
                    DownloadType::Candles => {
                        let candles = get_candles_from_csv(
                            file.path().to_str().expect("failed to get filepath"),
                        )
                        .await;
                        added = add_candles(pool, security, &candles).await;
                    }
                    DownloadType::Trades => {
                        let trades = get_trades_from_csv(
                            file.path().to_str().expect("failed to get filepath"),
                        )
                        .await;
                        added = add_trades(pool, security, &trades).await;
                    }
                }
                info!(
                    "{} => {}/{}, count => {}",
                    security,
                    download_type.to_string(),
                    file_name,
                    added
                );
            }
        }
    }
    let end = Local::now().time();
    info!("elapsed time: {}", elapsed_time(start, end));
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
            let file_path = file
                .path()
                .to_str()
                .expect("failed to get filepath")
                .to_owned();
            let candles = get_candles_from_csv(&file_path).await;
            let file_name = file
                .file_name()
                .to_str()
                .expect("failed to get filename")
                .to_owned();
            draw_candles(candles, security, &file_name).await;
        }
    }

    Ok(())
}
