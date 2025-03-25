use reqwest;
use std::fs;
use std::io::prelude::*;
use std::path::Path;

pub async fn say_hello() -> String {
    String::from("who wants to be a rich? me!")
}

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
