use std::fs;
use std::io::prelude::*;
use std::path::Path;

use chrono::{Days, Local};
use dotenv::dotenv;
use reqwest;
use tokio;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let securities = dotenv::var("SECURITIES")
        .expect("failed to get SECURITIES")
        .split_whitespace()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    for security in securities.iter() {
        run(&security, &DownloadType::Trades).await;
        run(&security, &DownloadType::Candles).await;
    }
}

#[derive(Debug)]
enum DownloadType {
    Trades,
    Candles,
}

impl ToString for DownloadType {
    fn to_string(&self) -> String {
        match self {
            Self::Trades => String::from("trades"),
            Self::Candles => String::from("candles"),
        }
    }
}

async fn run(security: &str, download_type: &DownloadType) {
    println!("{:?}: {}", download_type, security);

    let mut ok = true;
    let mut i = 1;
    let base_url = dotenv::var("BASE_URL").expect("failed to get BASE_URL");
    let today = Local::now().date_naive();
    let yesterday = today
        .checked_sub_days(Days::new(1))
        .expect("failed to get yesterday")
        .format("%Y-%m-%d");

    let suffix = match download_type {
        DownloadType::Trades => format!("boards/TQBR/securities/{security}/trades.csv"),
        DownloadType::Candles => {
            format!("securities/{security}/candles.csv?from={yesterday}&interval=1")
        }
    };

    let mut url = format!("{base_url}/{suffix}");
    let mut start = 0;

    while ok {
        let rows = get_rows(&url, download_type).await;
        start += rows.len() - 1;

        ok = rows.len() > 1;

        if !ok {
            break;
        }

        let data_dir = dotenv::var("DATA_DIR").expect("failed to get DATA_DIR");
        let path = Path::new(&data_dir)
            .join(download_type.to_string())
            .join(security);
        if !fs::exists(&path).expect("failed to get path") {
            fs::create_dir_all(&path).expect("failed to create path");
        }

        let file_name = format!("{}_{:02}.csv", today, i);
        let file_path = Path::new(&path.to_str().unwrap()).join(&file_name);
        let mut file = fs::File::create(file_path).expect("failed to create file");
        file.write_all(rows.join("\n").as_bytes())
            .expect("failed write to file");

        match download_type {
            DownloadType::Trades => {
                let mut rows = rows.iter().skip(1).collect::<Vec<_>>();
                rows.sort();

                let last_row = rows.last().expect("failed to get last row");
                let tradeno = last_row
                    .split(";")
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .first()
                    .expect("failed to get tradeno")
                    .to_owned();

                url = format!(
"{base_url}/boards/TQBR/securities/{security}/trades.csv?tradeno={tradeno}&next_trade=1");
            }
            DownloadType::Candles => {
                url = format!(
"{base_url}/securities/{security}/candles.csv?from={yesterday}&interval=1&start={start}"
                );
            }
        }

        i += 1;
    }
}

async fn get_rows(url: &str, download_type: &DownloadType) -> Vec<String> {
    let rows = match download_type {
        DownloadType::Trades => reqwest::get(url)
            .await
            .expect("failed to send request")
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
        DownloadType::Candles => reqwest::get(url)
            .await
            .expect("failed to send request")
            .text()
            .await
            .expect("failed to get body")
            .split("\n")
            .skip(2)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<String>>(),
    };
    return rows;
}
