use app::db::pg::init_db;
use std::io::Result;
use terminal::run_terminal;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    let pool = init_db().await;
    run_terminal(&pool).await;
    Ok(())
}
