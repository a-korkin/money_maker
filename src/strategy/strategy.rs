use crate::db::pg;
use crate::models::common::{Candle, Operation, OperationType};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use sqlx::postgres::PgPool;
use std::time::Duration;
use uuid::Uuid;

pub async fn run_strategy(pool: &PgPool) {
    let begin = NaiveDate::from_ymd_opt(2023, 1, 12)
        .unwrap()
        .and_hms_opt(7, 0, 0)
        .unwrap();
    let end = begin + Duration::from_secs(60 * 60 * 24 * 7);
    let mut balance: f32 = 100_000.0;
    let commission: f32 = 0.04;
    let security = "OZON";
    let duration = Duration::from_secs(60);
    let mut current = begin - duration;
    let attempt = Uuid::new_v4();

    // находим средний объём торгов за год
    let avg = pg::get_average_volume_by_year(pool, security, begin.year()).await;

    let mut prev_op: Option<Uuid> = None;
    while current <= end {
        current += duration;
        prev_op = st_1(
            pool,
            &security,
            current,
            &attempt,
            &commission,
            &mut balance,
            avg,
            prev_op,
        )
        .await;
    }
}

async fn st_1(
    pool: &PgPool,
    security: &str,
    begin: NaiveDateTime,
    attempt: &Uuid,
    commission: &f32,
    balance: &mut f32,
    avg: i32,
    prev: Option<Uuid>,
) -> Option<Uuid> {
    // находим точку входа: volume > avg && open > close
    let entry_points = pg::get_entry_points_1(pool, security, begin, avg).await;
    // let prev: Option<Operation> = None;
    if let Some(entry_point) = entry_points.first() {
        let count = f32::floor(*balance / entry_point.close) as i32;

        let op_id = create_operation(
            pool,
            attempt,
            "purchase",
            security,
            count,
            &begin,
            commission,
            balance,
            prev,
            &entry_point,
        )
        .await;
        return Some(op_id);
    }
    // выходим close >= 0.5%
    // let profit: f32 = (entry_point.close / 100.0) * 0.5 + entry_point.close;
    // let exit_points = pg::get_exit_points_1(pool, security, entry_point.end, profit).await;
    // if let Some(exit_point) = exit_points.first() {
    //     create_operation(
    //         pool, attempt, "sale", security, count, &begin, commission, balance, None, exit_point,
    //     )
    //     .await;
    // }
    return None;
}

async fn create_operation(
    pool: &PgPool,
    attempt: &Uuid,
    op_type: &str,
    security: &str,
    count: i32,
    time_at: &NaiveDateTime,
    commission: &f32,
    balance: &mut f32,
    prev: Option<Uuid>,
    candle: &Candle,
) -> Uuid {
    let id = Uuid::new_v4();
    let operation_type = OperationType::from(op_type);
    let sum_after: f32 = match operation_type {
        OperationType::Purchase => *balance - (count as f32 * candle.close),
        OperationType::Sale => *balance + (count as f32 * candle.close),
    };
    let operation = Operation {
        id,
        attempt: *attempt,
        operation_type,
        security: security.to_string(),
        count,
        price: candle.close,
        commission: *commission,
        time_at: *time_at,
        sum_before: *balance,
        sum_after,
    };
    pg::add_operation(pool, &operation, prev).await;
    return operation.id;
}
