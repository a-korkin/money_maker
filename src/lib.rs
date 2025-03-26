use chrono::{Duration, NaiveDateTime};
use csv;
use dotenv;
use plotters::prelude::*;
use reqwest;
use serde::Deserialize;
use std::fs;
use std::io::prelude::*;
use std::path::Path;

pub mod unix_timestamp {
    use chrono::NaiveDateTime;
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let time: String = Deserialize::deserialize(deserializer)?;
        let dt = NaiveDateTime::parse_from_str(&time, "%Y-%m-%d %H:%M:%S")
            .map_err(serde::de::Error::custom)?;
        Ok(dt)
    }
}

#[derive(Debug, Deserialize)]
pub struct Candle {
    pub open: f32,
    pub close: f32,
    pub high: f32,
    pub low: f32,
    pub value: f32,
    pub volume: f32,
    #[serde(with = "unix_timestamp")]
    pub begin: NaiveDateTime,
    #[serde(with = "unix_timestamp")]
    pub end: NaiveDateTime,
}

pub async fn download(url: &str, security: &str) -> std::io::Result<usize> {
    let response = reqwest::get(url)
        .await
        .expect("failed to download file")
        .text()
        .await
        .expect("failed to get body")
        .split("\n")
        .skip(2)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let count: usize = response.len();
    if count <= 1 {
        return Ok(count);
    }

    let path = &dotenv::var("DATA_DIR").expect("failed to get DATA_DIR");
    let path = Path::new(path).join(security);

    if !fs::exists(&path)? {
        fs::create_dir_all(&path)?;
    }
    let file_name = "moex2.csv";
    let file_path = Path::new(&path.to_str().unwrap()).join(file_name);
    let mut file = fs::File::create(file_path)?;
    file.write_all(response.join("\n").as_bytes())?;

    Ok(count)
}

pub async fn read_csv(path: &str) -> Vec<Candle> {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(path)
        .expect("failed to read csv");

    rdr.deserialize::<Candle>()
        .map(|c| c.unwrap())
        .collect::<Vec<_>>()
}

pub async fn draw_candles(candles: Vec<Candle>) {
    let dir = "graphs";
    if !fs::exists(dir).unwrap() {
        fs::create_dir_all(dir).unwrap();
    }
    let root = BitMapBackend::new("graphs/stock.png", (1024, 768)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    let to_date = candles.last().unwrap().end + Duration::minutes(10);
    let from_date = candles[0].end - Duration::minutes(10);

    let mut chart = ChartBuilder::on(&root)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .caption("MSFT", ("sans-serif", 50.0).into_font())
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
