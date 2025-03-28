use crate::models::common::{Candle, ToSql};
use dotenv;
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

pub async fn add_candles(pool: &PgPool, security: &str, candles: &Vec<Candle>) {
    let row: (String,) = sqlx::query_as("select id::text from public.securities where code = $1")
        .bind(security)
        .fetch_one(pool)
        .await
        .expect("failed to get security id");

    let candles_str = candles
        .iter()
        .map(|c| format!("('{}'::uuid, {})", row.0, c.for_insert()))
        .collect::<Vec<String>>()
        .join(",\n");
    let sql = format!(
        r#"
    insert into public.candles(security_id, open, close, high, low, value, volume, begin_t, end_t)
    values{}
        "#,
        candles_str
    );

    let _ = sqlx::query(&sql)
        .execute(pool)
        .await
        .expect("failed to insert candles");
}
