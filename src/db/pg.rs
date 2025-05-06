use crate::models::common::{
    Attempt, AvgPeriod, Candle, Frame, Operation, SecuritiesStr, ToSql, Trade, TradeView,
};
use chrono::NaiveDateTime;
use dotenv;
use sqlx::postgres::PgPool;
use sqlx::types::Uuid;

pub async fn init_db() -> PgPool {
    let db_url = dotenv::var("DATABASE_URL").expect("failed to DATABASE_URL");
    let pool = PgPool::connect(&db_url)
        .await
        .expect("failed to connect to postgres");
    pool
}

pub async fn add_securities(pool: &PgPool, securities: &Vec<String>) {
    let sql = r#"
    insert into public.securities(code)
    select a.code
    from 
    (
        select unnest($1) as code
    ) as a
    left join public.securities as b on a.code = b.code
    where b.id is null;
        "#;

    let _ = sqlx::query(sql)
        .bind(securities)
        .fetch_all(pool)
        .await
        .expect("failed to insert securities");
}

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
    values{};
        "#,
        candles_str
    );

    let res = sqlx::query(&sql)
        .execute(pool)
        .await
        .expect("failed to insert candles");
    res.rows_affected()
}

pub async fn add_trades(pool: &PgPool, security: &str, trades: &Vec<Trade>) -> u64 {
    let sec: (Uuid,) = sqlx::query_as("select id from public.securities where code = $1")
        .bind(security)
        .fetch_one(pool)
        .await
        .expect("failed to get security id");
    let trades_str = trades
        .iter()
        .filter(|a| a.board_id == "TQBR")
        .map(|t| format!("('{}', {})", sec.0, t.for_insert()))
        .collect::<Vec<String>>()
        .join(",\n");

    let sql = format!(
        r#"
    insert into public.trades(security_id, trade_no, trade_datetime, price, quantity, value, buysell)
    values{};
        "#,
        trades_str
    );

    let result = sqlx::query(&sql)
        .execute(pool)
        .await
        .expect("failed to insert trades");

    result.rows_affected()
}

pub async fn get_trades_view(
    pool: &PgPool,
    security: &str,
    begin: NaiveDateTime,
    end: NaiveDateTime,
    frame: &Frame,
    limit: i32,
) -> Vec<TradeView> {
    let join_str = match frame {
        Frame::M1 => "a.trade_period = b::timestamp",
        Frame::H1 => "a.trade_period = b::timestamp",
        Frame::D1 => "a.trade_period::date = b::date",
    };
    let sql = format!(
        r#"
    select 
        b::timestamp as trade_period, string_agg(coalesce(a.buysell, 'N'), '') as buysell, --coalesce(a.buysell, 'N') as buysell, 
        coalesce(avg(a.price), 0.0)::float4 as price_all, 
        coalesce(sum(a.quantity), 0) as quantity_all, 
        coalesce(sum(a.value), 0.0) as value_all,
        coalesce(avg(a.price) filter (where a.buysell = 'B'), 0.0)::float4 as price_buy, 
        coalesce(sum(a.quantity) filter (where a.buysell = 'B'), 0) as quantity_buy, 
        coalesce(sum(a.value) filter (where a.buysell = 'B'), 0.0) as value_buy,
        coalesce(avg(a.price) filter (where a.buysell = 'S'), 0.0)::float4 as price_sell, 
        coalesce(sum(a.quantity) filter (where a.buysell = 'S'), 0) as quantity_sell, 
        coalesce(sum(a.value) filter (where a.buysell = 'S'), 0.0) as value_sell
    from generate_series($3::timestamp, $4::timestamp, $1::interval) as b
    left join
    (
        select 
            date_bin($1::interval, t.trade_datetime, t.trade_datetime::date) as trade_period,
            t.price, t.quantity, t.value, t.buysell
        from public.trades as t
        inner join public.securities as s on s.id = t.security_id
        where s.code = $2
            and trade_datetime >= $3::timestamp
            and trade_datetime <= $4::timestamp
    ) as a on {}
    group by b --, a.buysell
    order by b
    limit $5;
        "#,
        join_str
    );

    let frame_str = match frame {
        Frame::M1 => "1 min",
        Frame::H1 => "1 hour",
        Frame::D1 => "1 day",
    };
    let result: Vec<TradeView> = sqlx::query_as(&sql)
        .bind(frame_str)
        .bind(security)
        .bind(begin)
        .bind(end)
        .bind(limit)
        .fetch_all(pool)
        .await
        .expect("failed to fetch trades");

    result
}

pub async fn get_securities_str(pool: &PgPool) -> String {
    let result: SecuritiesStr = sqlx::query_as(
        r#"
    select string_agg(code, ';')::text
    from public.securities;
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap();

    result.0
}

pub async fn get_all_securities(pool: &PgPool) -> Vec<String> {
    let result: Vec<SecuritiesStr> = sqlx::query_as(
        r#"
    select code from public.securities;
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap();

    return result
        .iter()
        .map(|a| a.0.to_owned())
        .collect::<Vec<String>>();
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
    limit $4;
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
    limit $4;
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
    limit $4;
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

pub async fn add_attempt(pool: &PgPool, attempt: &Attempt) {
    let sql = r#"
    insert into public.attempts(id, created_at, profit, commission)
    values($1, now(), $2, $3);
        "#;

    let _ = sqlx::query(sql)
        .bind(attempt.id)
        .bind(attempt.profit)
        .bind(attempt.commission)
        .execute(pool)
        .await
        .unwrap();
}

pub async fn add_operation(pool: &PgPool, operation: &Operation, prev_uuid: Option<Uuid>) {
    let sql = r#"
    insert into public.operations(
        id, attempt_id, operation_type, security_id, count,
        price, commission, time_at, sum_before, sum_after, prev)
    select $1, $2, $3, s.id, $5, $6, $7, $8, $9, $10, $11
    from public.securities as s
    where s.code = $4;
        "#;

    let _ = sqlx::query(sql)
        .bind(operation.id)
        .bind(operation.attempt)
        .bind(operation.operation_type.to_string())
        .bind(&operation.security)
        .bind(operation.count)
        .bind(operation.price)
        .bind(operation.commission)
        .bind(operation.time_at)
        .bind(operation.sum_before)
        .bind(operation.sum_after)
        .bind(prev_uuid)
        .execute(pool)
        .await
        .unwrap();
}

pub async fn get_average_volume(
    pool: &PgPool,
    security: &str,
    period: AvgPeriod,
    time_interval: i32,
) -> i32 {
    let sql = match period {
        AvgPeriod::Year => {
            r#"
    select avg(c.volume)::integer
    from public.candles as c
    inner join public.securities as s on s.id = c.security_id
    where to_char(c.begin_t, 'yyyy')::integer = $1
        and s.code = $2
    group by c.security_id;
        "#
        }
        AvgPeriod::Month => {
            r#"
    select avg(c.volume)::integer, to_char(c.begin_t, 'yyyyMM')::integer as per
    from public.candles as c
    inner join public.securities as s on s.id = c.security_id
    where to_char(c.begin_t, 'yyyyMM')::integer = $1
        and s.code = $2
    group by c.security_id, per;
        "#
        }
    };

    let result = sqlx::query_as::<_, (i32,)>(sql)
        .bind(time_interval)
        .bind(security)
        .fetch_one(pool)
        .await
        .unwrap();

    return result.0;
}
