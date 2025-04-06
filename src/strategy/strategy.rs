use crate::db::pg;
use crate::models::common::{Attempt, Candle, Frame, Operation, OperationType};
use chrono::{Datelike, NaiveDate};
use sqlx::postgres::PgPool;
// use std::time::Duration;
use uuid::Uuid;

pub async fn run_strategy(pool: &PgPool) {
    let begin = NaiveDate::from_ymd_opt(2023, 1, 1)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    // let end = begin + Duration::from_secs(60 * 60 * 24 * 31);
    let end = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    let mut balance: f32 = 100_000.0;
    let commission: f32 = 0.04;
    let security = "OZON";
    let mut purchased: i32 = 0;
    let mut profit: f32 = 0.0;

    // находим средний объём торгов за год
    let avg = pg::get_average_volume_by_year(pool, security, begin.year()).await;
    let mut last_operation: Option<Uuid> = None;
    let candles = pg::get_candles(pool, security, begin, end, 200_000, &Frame::M1).await;
    let attempt = Attempt {
        id: Uuid::new_v4(),
        profit: 0.5,
    };
    pg::add_attempt(pool, &attempt).await;

    for candle in candles {
        last_operation = st_1(
            pool,
            &security,
            &candle,
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
    candle: &Candle,
    attempt: &Attempt,
    commission: &f32,
    balance: &mut f32,
    avg: i32,
    prev: Option<Uuid>,
    purchased: &mut i32,
    profit: &mut f32,
) -> Option<Uuid> {
    // println!("period: {}", candle.begin);
    if *purchased > 0 {
        // выходим close >= 0.5%
        if candle.close >= *profit {
            let commission: f32 = ((*purchased as f32 * candle.close) / 100.0) * *commission;
            let op_id = create_operation(
                pool,
                attempt,
                "sold",
                security,
                *purchased,
                &commission,
                balance,
                prev,
                candle,
            )
            .await;
            *purchased = 0;
            return Some(op_id);
        }
        return prev;
    }
    // находим точку входа: volume > avg && open > close
    if candle.volume as i32 > avg && candle.open > candle.close {
        let mut count = f32::floor(*balance / candle.close) as i32;
        let commission: f32 = ((count as f32 * candle.close) / 100.0) * *commission;
        while (count as f32 * candle.close) + commission > *balance {
            count -= 1;
        }

        if count == 0 {
            return prev;
        }
        let op_id = create_operation(
            pool,
            attempt,
            "buy",
            security,
            count,
            &commission,
            balance,
            prev,
            candle,
        )
        .await;

        *purchased += count;
        // профит в 0.5%
        *profit = (candle.close / 100.0) * attempt.profit + candle.close;
        return Some(op_id);
    }
    return prev;
}

async fn create_operation(
    pool: &PgPool,
    attempt: &Attempt,
    op_type: &str,
    security: &str,
    count: i32,
    commission: &f32,
    balance: &mut f32,
    prev: Option<Uuid>,
    candle: &Candle,
) -> Uuid {
    let id = Uuid::new_v4();
    let operation_type = OperationType::from(op_type);
    let mut sum_after: f32 = match operation_type {
        OperationType::Buy => *balance - (count as f32 * candle.close),
        OperationType::Sold => *balance + (count as f32 * candle.close),
    };
    sum_after = sum_after - *commission;
    let operation = Operation {
        id,
        attempt: attempt.id,
        operation_type,
        security: security.to_string(),
        count,
        price: candle.close,
        commission: *commission,
        time_at: candle.begin,
        sum_before: *balance,
        sum_after,
    };
    *balance = sum_after;
    pg::add_operation(pool, &operation, prev).await;
    return operation.id;
}
