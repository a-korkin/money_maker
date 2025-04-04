use crate::db::pg;
use crate::models::common::{Operation, OperationType};
use chrono::{Datelike, NaiveDate, NaiveDateTime};
use sqlx::postgres::PgPool;
use uuid::Uuid;

pub async fn create_operation(pool: &PgPool) {
    let date = NaiveDate::from_ymd_opt(2023, 1, 12)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    st_1(pool, "OZON", date).await;
    // let mut prev: Option<Box<Operation>> = None;
    // for i in 0..5 {
    //     let id = Uuid::new_v4();
    //     let operation = Operation {
    //         id,
    //         attempt: Uuid::new_v4(),
    //         operation_type: OperationType::from("purchase"),
    //         security: "MOEX".to_owned(),
    //         count: i + 1,
    //         price: 412.32,
    //         commission: 0.17,
    //         time_at: NaiveDate::from_ymd_opt(2025, 3, 10)
    //             .unwrap()
    //             .and_hms_opt(0, 0, 0)
    //             .unwrap(),
    //         sum_before: 100_000.0,
    //         sum_after: 100_000.0 - (412.32 * i as f32 + 1_f32),
    //         prev,
    //     };
    //     pg::add_operation(pool, &operation).await;
    //     prev = Some(Box::from(operation));
    // }
}

pub async fn st_1(pool: &PgPool, security: &str, begin: NaiveDateTime) {
    // находим средний объём торгов за год
    let avg = pg::get_average_volume_by_year(pool, security, begin.year()).await;

    // находим точку входа: volume > avg && open > close
    let entry_points = pg::get_entry_points_1(pool, security, begin, avg).await;
    let entry_point = entry_points.first().unwrap();
    // выходим close >= 0.5%
    let profit: f32 = (entry_point.close / 100.0) * 0.5 + entry_point.close;
    let exit_points = pg::get_exit_points_1(pool, security, entry_point.end, profit).await;
    let exit_point = exit_points.first().unwrap();
    println!("start: {} end: {}", entry_point.close, exit_point.close);
}
