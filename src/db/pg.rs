use crate::models::common::{Candle, Frame, SecuritiesStr, ToSql};
use chrono::NaiveDateTime;
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

pub async fn get_securities_str(pool: &PgPool) -> String {
    let result: SecuritiesStr = sqlx::query_as(
        r#"
    select string_agg(code, ';')::text
    from public.securities
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    result.0
}

pub async fn get_candles(
    pool: &PgPool,
    security: &str,
    begin: NaiveDateTime,
    end: NaiveDateTime,
    limit: i32,
    frame: &Frame,
) -> Vec<Candle> {
    let sql = match frame {
        Frame::M1 => {
            r#"
    select 
        c.open::float4 as open, 
        c.close::float4 as close, 
        c.high::float4 as high, 
        c.low::float4 as low, 
        c.value::float4 as value, 
        c.volume::float4 as volume, 
        c.begin_t as begin, c.end_t as end
    from public.candles as c
    inner join public.securities as s on s.id = c.security_id
    where s.code = $1
        and c.begin_t >= $2
        and c.end_t <= $3
    order by c.begin_t
    limit $4
        "#
        }
        Frame::H1 => {
            r#"
    select a.open, a.close, a.high, a.low, a.value, a.volume, a.begin, a.end
    from
    (
        select 
            (array_agg(open order by c.begin_t))[1]::float4 as open, 
            (array_agg(close order by c.end_t desc))[1]::float4 as close, 
            max(c.high)::float4 as high, min(c.low)::float4 as low, 
            sum(c.value)::float4 as value, 
            sum(c.volume)::float4 as volume, 
            min(c.begin_t) as begin, max(c.end_t) as end,
            c.begin_t::date as cdate, extract(hour from date_trunc('hour', c.begin_t)) as hour
        from public.candles as c
        inner join public.securities as s on s.id = c.security_id
        where s.code = $1
            and c.begin_t::date >= $2
            and c.end_t::date <= $3
        group by cdate, hour
    ) as a
    order by a.begin
    limit $4
        "#
        }
        Frame::D1 => {
            r#"
    select a.open, a.close, a.high, a.low, a.value, a.volume, a.begin, a.end
    from
    (
        select 
            (array_agg(open order by c.begin_t))[1]::float4 as open, 
            (array_agg(close order by c.end_t desc))[1]::float4 as close, 
            max(c.high)::float4 as high, min(c.low)::float4 as low, 
            sum(c.value)::float4 as value, 
            sum(c.volume)::float4 as volume, 
            min(c.begin_t) as begin, max(c.end_t) as end,
            c.begin_t::date as cdate
        from public.candles as c
        inner join public.securities as s on s.id = c.security_id
        where s.code = $1
            and c.begin_t::date >= $2
            and c.end_t::date <= $3
        group by cdate
    ) as a
    order by a.begin
    limit $4
        "#
        }
    };

    let result: Vec<Candle> = sqlx::query_as(sql)
        .bind(security)
        .bind(begin)
        .bind(end)
        .bind(limit)
        .fetch_all(pool)
        .await
        .unwrap();

    result
}
