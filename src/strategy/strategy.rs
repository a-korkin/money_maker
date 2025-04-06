use crate::db::pg;
use crate::models::common::{Attempt, AvgPeriod, Candle, Frame, Operation, OperationType};
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
    // let mut wallet = Wallet { balance: 100_000.0 };

    let packets: Vec<Packet> = vec![
        Packet::new("OZON", 1, 100_000.0),
        // Packet::new("LKOH", 1, 100_000.0),
        // Packet::new("SBER", 10, 100_000.0),
    ];

    let mut period = begin.format("%Y%m").to_string().parse::<i32>().unwrap();

    for mut packet in packets {
        // находим средний объём торгов за год
        // let avg =
        //     pg::get_average_volume(pool, &packet.security, AvgPeriod::Year, begin.year()).await;

        // находим средний объём торгов за текущий месяц
        let mut avg =
            pg::get_average_volume(pool, &packet.security, AvgPeriod::Month, period).await;
        let mut last_operation: Option<Uuid> = None;
        let candles =
            pg::get_candles(pool, &packet.security, begin, end, 200_000, &Frame::M1).await;
        let attempt = Attempt {
            id: Uuid::new_v4(),
            profit: 2.0, // профит, который необходим
            commission: 0.04,
        };
        pg::add_attempt(pool, &attempt).await;

        for candle in &candles {
            let current_period = candle
                .begin
                .format("%Y%m")
                .to_string()
                .parse::<i32>()
                .unwrap();
            if period != current_period {
                period = current_period;
                avg =
                    pg::get_average_volume(pool, &packet.security, AvgPeriod::Month, period).await;
            }
            last_operation = st_1(
                pool,
                &mut packet,
                &candle,
                &attempt, //&mut wallet,
                avg,
                last_operation,
            )
            .await;
        }
    }
}

#[allow(dead_code)]
struct Wallet {
    balance: f32,
}

struct Packet {
    security: String,
    min_count: i32,
    purchased: i32,
    profit: f32,
    balance: f32,
}

impl Packet {
    fn new(security: &str, min_count: i32, balance: f32) -> Self {
        Self {
            security: security.to_string(),
            min_count,
            purchased: 0,
            profit: 0.0,
            balance,
        }
    }
}

async fn st_1(
    pool: &PgPool,
    packet: &mut Packet,
    candle: &Candle,
    attempt: &Attempt, // wallet: &mut Wallet,
    avg: i32,
    prev: Option<Uuid>,
) -> Option<Uuid> {
    if packet.purchased > 0 {
        // выходим close >= 0.5%
        if candle.close >= packet.profit {
            let commission: f32 =
                ((packet.purchased as f32 * candle.close) / 100.0) * attempt.commission;
            let op_id = create_operation(
                pool,
                attempt,
                "sold",
                packet,
                &commission, //&mut wallet.balance,
                prev,
                candle,
            )
            .await;
            packet.purchased = 0;
            return Some(op_id);
        }
        return prev;
    }
    // находим точку входа: volume > avg && open > close
    if candle.volume as i32 > avg && candle.open > candle.close {
        let mut count = f32::floor(packet.balance / (candle.close)) as i32;
        let commission: f32 = ((count as f32 * candle.close) / 100.0) * attempt.commission;
        while (count as f32 * candle.close) + commission > packet.balance {
            count -= 1;
        }
        count = (count / packet.min_count) * packet.min_count;

        if count == 0 {
            return prev;
        }
        packet.purchased += count;
        packet.profit = (candle.close / 100.0) * attempt.profit + candle.close;
        let op_id = create_operation(
            pool,
            attempt,
            "buy",
            packet,
            &commission, //&mut wallet.balance,
            prev,
            candle,
        )
        .await;

        return Some(op_id);
    }
    return prev;
}

async fn create_operation(
    pool: &PgPool,
    attempt: &Attempt,
    op_type: &str,
    packet: &mut Packet,
    commission: &f32, //balance: &mut f32,
    prev: Option<Uuid>,
    candle: &Candle,
) -> Uuid {
    let id = Uuid::new_v4();
    let operation_type = OperationType::from(op_type);
    let mut sum_after: f32 = match operation_type {
        OperationType::Buy => packet.balance - (packet.purchased as f32 * candle.close),
        OperationType::Sold => packet.balance + (packet.purchased as f32 * candle.close),
    };
    sum_after = sum_after - *commission;
    let operation = Operation {
        id,
        attempt: attempt.id,
        operation_type,
        security: packet.security.clone(),
        count: packet.purchased,
        price: candle.close,
        commission: *commission,
        time_at: candle.begin,
        sum_before: packet.balance,
        sum_after,
    };
    packet.balance = sum_after;
    pg::add_operation(pool, &operation, prev).await;
    return operation.id;
}
