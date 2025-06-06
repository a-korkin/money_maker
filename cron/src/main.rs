use std::fs;
use std::io::prelude::*;
use std::path::Path;

use chrono::Local;
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
        run(&security).await;
    }
}

async fn run(security: &str) {
    let mut ok = true;
    let mut i = 1;
    let base_url = "https://iss.moex.com/iss/engines/stock/markets/shares\
                    /boards/TQBR/securities/";

    let mut url = format!("{base_url}/{security}/trades.csv");
    while ok {
        let rows = get_rows(&url).await;
        ok = rows.len() > 1;

        if !ok {
            break;
        }

        let data_dir = dotenv::var("DATA_DIR").expect("failed to get SAVE_PATH");
        let path = Path::new(&data_dir).join("iss_moex").join(security);
        if !fs::exists(&path).expect("failed to get path") {
            fs::create_dir_all(&path).expect("failed to create path");
        }

        let today = Local::now().date_naive();
        let file_name = format!("{}_{:02}.csv", today, i);
        let file_path = Path::new(&path.to_str().unwrap()).join(&file_name);
        let mut file = fs::File::create(file_path).expect("failed to create file");
        file.write_all(rows.join("\n").as_bytes())
            .expect("failed write to file");

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

        url = format!("{base_url}/{security}/trades.csv?tradeno={tradeno}&next_trade=1");

        i += 1;
    }
}

async fn get_rows(url: &str) -> Vec<String> {
    reqwest::get(url)
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
        .collect::<Vec<String>>()
}
