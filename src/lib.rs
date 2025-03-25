use chrono::NaiveDateTime;
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

pub async fn read_csv(path: &str) {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(path)
        .expect("failed to read csv");

    for result in rdr.deserialize::<Candle>() {
        println!("{:?}", result);
    }
}

pub async fn draw_ex() {
    let dir = "graphs";
    if !fs::exists(dir).unwrap() {
        fs::create_dir_all(dir).unwrap();
    }
    let root = BitMapBackend::new("graphs/0.png", (640, 480)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let mut chart = ChartBuilder::on(&root)
        .caption("y=x^2", ("sans-serif", 30).into_font())
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(-1f32..1f32, -0.1f32..1f32)
        .unwrap();

    chart.configure_mesh().draw().unwrap();

    chart
        .draw_series(LineSeries::new(
            (-50..50).map(|x| x as f32 / 50.0).map(|x| (x, x * x)),
            &RED,
        ))
        .unwrap()
        .label("y = x^2")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.5))
        .border_style(&BLACK)
        .draw()
        .unwrap();

    root.present().unwrap();
}
