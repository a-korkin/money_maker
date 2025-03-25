use chrono::NaiveDateTime;
use csv;
use reqwest;
use serde::Deserialize;
use std::fs;
use std::io::prelude::*;
use std::path::Path;

pub async fn download(url: &str) -> std::io::Result<()> {
    let res = reqwest::get(url)
        .await
        .expect("failed to download file")
        .text()
        .await
        .expect("failed to get csv");

    let path = "data/iss_moex";
    if !fs::exists(path).unwrap() {
        fs::create_dir_all(path)?;
    }
    let file_name = "moex.csv";
    let file_path = Path::new(path).join(file_name);
    let mut file = fs::File::create(file_path)?;
    file.write_all(res.as_bytes())?;

    Ok(())
}

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

pub fn read_csv(path: &str) {
    let mut rdr = csv::ReaderBuilder::new()
        .delimiter(b';')
        .from_path(path)
        .expect("failed to read csv");

    // for record in rdr.records() {
    //     let record = record.expect("failed to read record");
    //     println!("{:?}", record);
    // }

    // let mut rdr = csv::Reader::from_path(path).unwrap();

    for result in rdr.deserialize::<Candle>() {
        println!("{:?}", result);
        // let candle: Candle = result.unwrap();
        // println!("{:?}", candle);
    }
}
