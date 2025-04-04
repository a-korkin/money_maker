use crate::db::pg;
use crate::models::common::{DateRange, Operation, OperationType};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use sqlx::postgres::PgPool;
use std::time::Duration;
use uuid::Uuid;

pub async fn create_operation(pool: &PgPool, op_type: &str, security: &str) {
    let mut prev: Option<Box<Operation>> = None;
    for i in 0..5 {
        let id = Uuid::new_v4();
        let operation = Operation {
            id,
            attempt: Uuid::new_v4(),
            operation_type: OperationType::from(op_type),
            security: security.to_string(),
            count: i + 1,
            price: 412.32,
            commission: 0.17,
            time_at: NaiveDate::from_ymd_opt(2025, 3, 10)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            sum_before: 100_000.0,
            sum_after: 100_000.0 - (412.32 * i as f32 + 1_f32),
            prev,
        };
        pg::add_operation(pool, &operation).await;
        prev = Some(Box::from(operation));
    }
}

pub async fn run_strategy(_pool: &PgPool) {
    let begin = NaiveDate::from_ymd_opt(2023, 1, 12)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let end = begin + Duration::from_secs(60 * 60 * 24 * 7);
    let _balance: f32 = 100_000.0;
    let _commission: f32 = 0.04;

    let mut current = begin;
    while current <= end {
        println!("current: {}", current);
        current += Duration::from_secs(60 * 60 * 12);
    }

    // for d in DateRange(begin, begin + Duration::from_secs(60 * 60 * 24)) {
    //     println!("date: {d}");
    // }

    // pub struct DateRange(pub DateTime<Utc>, pub DateTime<Utc>);
    // st_1(pool, "OZON", date).await;
}

pub async fn st_1(pool: &PgPool, security: &str, begin: NaiveDateTime) -> bool {
    // находим средний объём торгов за год
    let avg = pg::get_average_volume_by_year(pool, security, begin.year()).await;

    // находим точку входа: volume > avg && open > close
    let entry_points = pg::get_entry_points_1(pool, security, begin, avg).await;
    if let Some(entry_point) = entry_points.first() {
        // выходим close >= 0.5%
        let profit: f32 = (entry_point.close / 100.0) * 0.5 + entry_point.close;
        let exit_points = pg::get_exit_points_1(pool, security, entry_point.end, profit).await;
        if let Some(exit_point) = exit_points.first() {
            println!("start: {} end: {}", entry_point.close, exit_point.close);
            return true;
        } else {
            return false;
        }
    } else {
        return false;
    }
}
