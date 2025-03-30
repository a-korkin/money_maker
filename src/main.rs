use money_maker::run_terminal;
use std::io::Result;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    // run().await;

    run_terminal().await;
    Ok(())
}
