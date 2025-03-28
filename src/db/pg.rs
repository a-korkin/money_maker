use crate::models::common::{Candle, ToSql};
use dotenv;
use sqlx::postgres::PgPool;

#[allow(dead_code)]
pub async fn init_db() -> PgPool {
    let db_url = dotenv::var("DATABASE_URL").expect("failed to DATABASE_URL");
    let pool = PgPool::connect(&db_url)
        .await
        .expect("failed to connect to postgres");
    pool
}

#[allow(dead_code)]
pub async fn add_securities(pool: &PgPool, securities: &Vec<String>) {
    let sql = r#"
    insert into public.securities(code)
    select a.code
    from 
    (
        select unnest($1) as code
    ) as a
    left join public.securities as b on a.code = b.code
    where b.id is null
        "#;

    let _ = sqlx::query(sql)
        .bind(securities)
        .fetch_all(pool)
        .await
        .expect("failed to insert securities");
}

#[allow(dead_code)]
pub async fn add_candles(pool: &PgPool, security: &str, candles: &Vec<Candle>) -> u64 {
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

    let res = sqlx::query(&sql)
        .execute(pool)
        .await
        .expect("failed to insert candles");
    res.rows_affected()
}
