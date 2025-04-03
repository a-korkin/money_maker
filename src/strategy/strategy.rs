use crate::db::pg;
use crate::models::common::{Operation, OperationType};
use chrono::NaiveDate;
use sqlx::postgres::PgPool;
use uuid::Uuid;

pub async fn create_operation(pool: &PgPool) {
    let mut prev: Option<Box<Operation>> = None;
    for i in 0..5 {
        let id = Uuid::new_v4();
        let operation = Operation {
            id,
            attempt: Uuid::new_v4(),
            operation_type: OperationType::from("purchase"),
            security: "MOEX".to_owned(),
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
