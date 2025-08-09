mod db;
mod models;
mod strategy;
mod terminal;
mod utils;

use anyhow::{Context, Result};
use chrono::prelude::*;
use chrono::{Duration, NaiveDate};
use clap::Parser;
use csv;
use db::pg::{
    add_candles, add_securities, add_trades, get_all_securities, get_candles, init_db,
    remove_dooblicates_candles, remove_dooblicates_trades,
};
use dotenv;
use log::info;
use models::common::{Candle, Frame, Trade, TradeInfo};
use plotters::prelude::*;
use sqlx::postgres::PgPool;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration as time_duration;
use utils::logger;

/// Money maker app
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// List of securities separated by comma
    #[arg(short, long, default_value = "all")]
    secs: String,

    /// Kind csv files
    #[arg(short, long, default_value = "none")]
    kind: String,

    /// Adding to DB
    #[arg(short, long)]
    add: bool,

    /// Run terminal
    #[arg(short, long)]
    terminal: bool,

    /// Show candles
    #[arg(short, long)]
    display: bool,
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

    if args.kind.as_str() != "none" {
        let kind = Kind::from(args.kind.as_str());
        if args.add {
            add_securities(&pool, &securities).await;
            insert_entity(&pool, kind, &securities)
                .await
                .expect("failed to insert entity");
        }
    }

    if args.display {
        // let date_format: &str = "%Y-%m-%d %H:%M:%S";
        // let begin = NaiveDateTime::parse_from_str("2025-04-26 00:00:00", date_format)
        //     .expect("failed to convert datetime");
        // let end = begin + time_duration::from_secs(60 * 60 * 24 * 1);
        // let candles = get_candles(&pool, "MOEX", begin, end, 1000, &Frame::M1).await;
        // display(&candles);
        let date = NaiveDate::parse_from_str("2025-05-13", "%Y-%m-%d").expect("failed to date");
        let info = strategy::strategy::trade_info(&pool, "AFLT", &date).await;
        for i in info {
            pretty_print_info(&i);
        }
    }
}

fn pretty_print_info(info: &TradeInfo) {
    let color = match (info.open, info.close) {
        (o, c) if o > c => "31",
        (o, c) if o < c => "32",
        _ => "37",
    };
    let max = f32::max(info.open, info.close);
    let min = f32::min(info.open, info.close);
    let percent = (max / (min / 100.0)) - 100.0;
    println!(
        "\x1b[{color}m{}\topen: {:.2}\tclose: {:.2}\tpercent: {:.2}\tquantity: {:.2}\x1b[0m",
        info.begin, info.open, info.close, percent, info.sum_quantity,
    );
}

fn pretty_print_candle(candle: &Candle) {
    let color = match (candle.open, candle.close) {
        (o, c) if o > c => "31",
        (o, c) if o < c => "32",
        _ => "37",
    };
    let max = f32::max(candle.open, candle.close);
    let min = f32::min(candle.open, candle.close);
    let percent = (max / (min / 100.0)) - 100.0;
    println!(
        "\x1b[{color}m{}\topen: {:.2}\tclose: {:.2}\tpercent: {:.2}\x1b[0m",
        candle.begin, candle.open, candle.close, percent
    );
}

fn display(candles: &Vec<Candle>) {
    for candle in candles {
        pretty_print_candle(candle);
    }
}

pub async fn run_terminal(pool: &PgPool) {
    terminal::terminal::run_terminal(pool).await;
}

pub enum Kind {
    Candles,
    Trades,
}

impl ToString for Kind {
    fn to_string(&self) -> String {
        match self {
            Self::Candles => String::from("candles"),
            Self::Trades => String::from("trades"),
        }
    }
}

impl From<&str> for Kind {
    fn from(value: &str) -> Self {
        match value {
            "c" | "candles" => Self::Candles,
            "t" | "trades" => Self::Trades,
            _ => unimplemented!(),
        }
    }
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

pub async fn get_candles_from_csv(path: &str) -> Result<Vec<Candle>> {
    let rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(path)
        .context("failed to read csv");

    let result = rdr?
        .deserialize::<Candle>()
        .map(|c| c.expect("failed to parse elem"))
        .collect::<Vec<_>>();

    Ok(result)
}

pub async fn get_trades_from_csv(path: &str) -> Result<Vec<Trade>> {
    let rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(path)
        .context("failed to read csv");

    let result = rdr?
        .deserialize::<Trade>()
        .map(|c| c.expect("failed to parse elem"))
        .collect::<Vec<_>>();

    Ok(result)
}

async fn insert_entity(pool: &PgPool, kind: Kind, securities: &Vec<String>) -> Result<()> {
    let start = Local::now().time();
    for security in securities {
        let data_dir = dotenv::var("DATA_DIR").expect("failed to get DATA_DIR");
        let path = Path::new(&data_dir)
            .join(kind.to_string())
            .join(security)
            .to_str()
            .unwrap()
            .to_owned();
        if !fs::exists(&path).expect(&format!("directory not exists: {}", &path)) {
            continue;
        }
        for entry in fs::read_dir(path).unwrap() {
            let file = entry.unwrap();
            let file_type = file.file_type().unwrap();

            if file_type.is_file() {
                let file_name = file.file_name().to_str().unwrap().to_owned();
                match kind {
                    Kind::Candles => {
                        let candles = get_candles_from_csv(
                            file.path().to_str().expect("failed to get filepath"),
                        )
                        .await?;
                        let added = add_candles(pool, security, &candles).await;
                        info!(
                            "{} => {}/{}, count => {}",
                            security,
                            kind.to_string(),
                            file_name,
                            added
                        );
                    }
                    Kind::Trades => {
                        let trades = get_trades_from_csv(
                            file.path().to_str().expect("failed to get filepath"),
                        )
                        .await?;
                        let added = add_trades(pool, security, &trades).await;
                        info!(
                            "{} => {}/{}, count => {}",
                            security,
                            kind.to_string(),
                            file_name,
                            added
                        );
                    }
                }
            }
        }
    }
    match kind {
        Kind::Trades => remove_dooblicates_trades(pool).await,
        Kind::Candles => remove_dooblicates_candles(pool).await,
    }
    let end = Local::now().time();
    info!("elapsed time: {}", elapsed_time(start, end));

    Ok(())
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

pub async fn draw_graphs(security: &str) -> Result<()> {
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
            let candles = get_candles_from_csv(&file_path).await?;
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
