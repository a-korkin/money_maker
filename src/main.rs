use money_maker::download;
use std::io;
use tokio;

#[tokio::main]
async fn main() -> io::Result<()> {
    let url = "https://iss.moex.com/iss/engines/stock/markets/shares/securities/MOEX/candles.csv?from=2025-03-03&till=2025-03-03&interval=10";
    download(url).await;
    Ok(())
}
