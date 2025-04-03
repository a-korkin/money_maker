use money_maker::run;
use std::io::Result;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    run().await;
    Ok(())
}
