use crate::db::pg;
use crate::models::common::{Candle, Operation, OperationType};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use sqlx::postgres::PgPool;
use std::time::Duration;
use uuid::Uuid;

pub async fn run_strategy(pool: &PgPool) {
    let begin = NaiveDate::from_ymd_opt(2023, 1, 12)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let end = begin + Duration::from_secs(60 * 60 * 24 * 7);
    let mut balance: f32 = 100_000.0;
    let commission: f32 = 0.04;
    let security = "OZON";
    let duration = Duration::from_secs(60);
    let mut current = begin - duration;
    let attempt = Uuid::new_v4();
    let mut purchased: i32 = 0;
    let mut profit: f32 = 0.0;

    // находим средний объём торгов за год
    let avg = pg::get_average_volume_by_year(pool, security, begin.year()).await;
    println!("avg: {avg}");

    let mut last_operation: Option<Uuid> = None;
    while current <= end {
        current += duration;
        last_operation = st_1(
            pool,
            &security,
            current,
            &attempt,
            &commission,
            &mut balance,
            avg,
            last_operation,
            &mut purchased,
            &mut profit,
        )
        .await;
    }
}

async fn st_1(
    pool: &PgPool,
    security: &str,
    date: NaiveDateTime,
    attempt: &Uuid,
    commission: &f32,
    balance: &mut f32,
    avg: i32,
    prev: Option<Uuid>,
    purchased: &mut i32,
    profit: &mut f32,
) -> Option<Uuid> {
    if *purchased > 0 {
        // выходим close >= 0.5%
        let exit_points = pg::get_exit_points_1(pool, security, date, *profit).await;
        if let Some(exit_point) = exit_points.first() {
            let op_id = create_operation(
                pool, attempt, "sale", security, *purchased, &date, commission, balance, prev,
                exit_point,
            )
            .await;
            *purchased = 0;
            println!("profit: {profit}, balance: {balance}");
            return Some(op_id);
        }
        return prev;
    }
    // находим точку входа: volume > avg && open > close
    let entry_points = pg::get_entry_points_1(pool, security, date, avg).await;
    println!("time: {}, count ep: {}", date, entry_points.len());

    if let Some(entry_point) = entry_points.first() {
        let count = f32::floor(*balance / entry_point.close) as i32;

        if count == 0 {
            return prev;
        }
        let op_id = create_operation(
            pool,
            attempt,
            "purchase",
            security,
            count,
            &date,
            commission,
            balance,
            prev,
            &entry_point,
        )
        .await;

        *purchased += count;
        // профит в 0.5%
        *profit = (entry_point.close / 100.0) * 0.5 + entry_point.close;
        return Some(op_id);
    }
    return prev;
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
        OperationType::Buy => *balance - (count as f32 * candle.close),
        OperationType::Sold => *balance + (count as f32 * candle.close),
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
    *balance = sum_after;
    pg::add_operation(pool, &operation, prev).await;
    return operation.id;
}
