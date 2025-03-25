use chrono::{Duration, NaiveDateTime};
use csv;
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
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub value: f64,
    pub volume: f64,
    #[serde(with = "unix_timestamp")]
    pub begin: NaiveDateTime,
    #[serde(with = "unix_timestamp")]
    pub end: NaiveDateTime,
}

pub async fn download(url: &str) -> std::io::Result<()> {
    let res = reqwest::get(url)
        .await
        .expect("failed to download file")
        .text()
        .await
        .expect("failed to get csv");

    let path = "data/iss_moex";
    if !fs::exists(path)? {
        fs::create_dir_all(path)?;
    }
    let file_name = "moex.csv";
    let file_path = Path::new(path).join(file_name);
    let mut file = fs::File::create(file_path)?;
    file.write_all(res.as_bytes())?;

    Ok(())
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

    chart
        .configure_mesh() // .light_line_style(WHITE)
        .draw()
        .unwrap();

    let candles = candles.iter().map(|x| {
        CandleStick::new(
            x.begin,
            x.open as f32,
            x.high as f32,
            x.low as f32,
            x.close as f32,
            GREEN.filled(),
            RED.filled(),
            7,
        )
    });

    chart.draw_series(candles).unwrap();
    root.present().unwrap();
}
