use dotenv;
use money_maker::get_candles_from_csv;
use sqlx::postgres::PgPool;

pub async fn init_db() -> PgPool {
    let db_url = dotenv::var("DATABASE_URL").expect("failed to DATABASE_URL");
    let pool = PgPool::connect(&db_url)
        .await
        .expect("failed to connect to postgres");
    pool
}

pub async fn add_securities(pool: &PgPool, securities: &Vec<&str>) {
    let sql = r#"
    insert into public.securities(code)
    select code 
    from unnest($1) as code
    returning id, code
        "#;
    let _ = sqlx::query(sql)
        .bind(securities)
        .fetch_all(pool)
        .await
        .expect("failed to insert securities");
}

pub async fn add_candles(pool: &PgPool) {
    let candles = get_candles_from_csv("data/iss_moex/MOEX/2025-03-03_1.csv").await;
    let security = "1592a4c0-e9cf-4c14-b985-460b958df3df";
    let cans = candles
        .iter()
        .map(|c| format!("('{}', {})", security, c.to_string()))
        .collect::<Vec<String>>()
        .join(",\n");
    let sql = format!(
        r#"
    insert into public.candles(security_id, open, close, high, low, value, volume, begin_t, end_t)
    values{}"#,
        cans
    );
    let _ = sqlx::query(&sql)
        .execute(pool)
        .await
        .expect("failed to insert candles");
}
