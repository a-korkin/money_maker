use crate::models::common::TradeInfo;
use chrono::NaiveDate;
use sqlx;
use sqlx::postgres::PgPool;

pub async fn get_trade_info(pool: &PgPool, security: &str, date: &NaiveDate) -> Vec<TradeInfo> {
    let sql = r#"
    select 
        c.begin_t as begin, round(avg(t.price)::decimal, 2)::float4 as avg_price, 
        sum(t.quantity)::int4 as sum_quantity, t.buysell, 
        c.open::float4 as open, c.close::float4 as close, 
        c.high::float4 as high, c.low::float4 as low
    from public.candles as c
    inner join public.securities as s on s.id = c.security_id
    inner join public.trades as t on t.security_id = c.security_id 
        and c.begin_t = date_bin('1 min', t.trade_datetime, t.trade_datetime::date)
    where s.code = $1 
        and c.begin_t::date = $2
    group by c.begin_t, c.open, c.close, c.high, c.low, t.buysell
    order by c.begin_t, t.buysell;
        "#;

    let result: Vec<TradeInfo> = sqlx::query_as(sql)
        .bind(security)
        .bind(date)
        .fetch_all(pool)
        .await
        .expect("Failed to get trade_info");

    return result;
}
