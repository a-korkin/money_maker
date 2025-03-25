use reqwest;
use std::fs;

pub async fn say_hello() -> String {
    String::from("who wants to be a rich? me!")
}

pub async fn download(url: &str) {
    // let _res = reqwest::get(url)
    //     .await
    //     .expect("failed to download file")
    //     .text()
    //     .await
    //     .expect("failed to get csv");
    let path = "downloads/iss_moex";
    if !fs::exists(path).unwrap() {
        fs::create_dir_all(path).expect("failed to create dir");
    }
}
