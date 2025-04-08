use crate::db::pg;
use crate::models::common::{Attempt, AvgPeriod, Candle, Frame, Operation, OperationType, Packet};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use sqlx::postgres::PgPool;
// use std::time::Duration;
use uuid::Uuid;

pub async fn run_strategy(pool: &PgPool) {
    let begin = NaiveDate::from_ymd_opt(2024, 6, 10)
        .unwrap()
        .and_hms_opt(10, 0, 0)
        .unwrap();
    // let end = begin + Duration::from_secs(60 * 60 * 24 * 31);
    let end = NaiveDate::from_ymd_opt(2024, 6, 10)
        .unwrap()
        .and_hms_opt(11, 0, 0)
        .unwrap();
    // let mut wallet = Wallet { balance: 100_000.0 };

    let packets: Vec<Packet> = vec![
        Packet::new("OZON", 1, 100_000.0),
        // Packet::new("LKOH", 1, 100_000.0),
        // Packet::new("SBER", 10, 100_000.0),
    ];

    for mut packet in packets {
        // strategy_1(pool, &mut packet, begin, end).await;
        // strategy_2(pool, &mut packet, begin, end).await;
        strategy_3(pool, &mut packet, begin, end).await;
    }
}

async fn strategy_3(pool: &PgPool, packet: &mut Packet, begin: NaiveDateTime, end: NaiveDateTime) {
    let candles = pg::get_candles(pool, &packet.security, begin, end, 200_000, &Frame::M1).await;
    let mut volume_up: f32 = 0.0;
    let mut volume_down: f32 = 0.0;
    let mut volume_all: f32 = 0.0;

    for candle in candles {
        if candle.close > candle.open {
            volume_up += candle.volume;
        }
        if candle.close < candle.open {
            volume_down += candle.volume;
        }
        volume_all += candle.volume;

        let red_line = (volume_up - volume_down) / (volume_all / 100.0);

        println!(
            "time: {}, volume_up: {}, volume_down: {}, volume_all: {}, red_line: {}",
            candle.begin.time(),
            volume_up,
            volume_down,
            volume_all,
            red_line
        );
    }
}

async fn strategy_2(pool: &PgPool, packet: &mut Packet, begin: NaiveDateTime, end: NaiveDateTime) {
    let mut last_operation: Option<Uuid> = None;
    let candles = pg::get_candles(pool, &packet.security, begin, end, 200_000, &Frame::M1).await;
    let attempt = Attempt {
        id: Uuid::new_v4(),
        profit: 1.5,
        commission: 0.04,
    };
    pg::add_attempt(pool, &attempt).await;

    let mut prev_avg = 100;
    let mut current_date = candles.first().unwrap().begin.date();
    let mut i: i32 = 0;
    let mut vol: f32 = 0.0;

    for candle in candles {
        last_operation =
            strategy_2_logic(pool, packet, &candle, &attempt, prev_avg, last_operation).await;
        if current_date != candle.begin.date() {
            current_date = candle.begin.date();
            prev_avg = (vol / i as f32) as i32;
        } else {
            i += 1;
            vol += candle.volume;
        }
    }
}

async fn strategy_2_logic(
    pool: &PgPool,
    packet: &mut Packet,
    candle: &Candle,
    attempt: &Attempt, // wallet: &mut Wallet,
    avg: i32,
    prev: Option<Uuid>,
) -> Option<Uuid> {
    if packet.purchased > 0 {
        // выходим
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
    // находим точку входа
    // if candle.volume as i32 > avg && candle.open > candle.close {
    if candle.begin.hour() >= 13
        && candle.begin.hour() < 19
        && candle.volume as i32 >= avg * 5
        && candle.open > candle.close
    {
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

async fn strategy_1(pool: &PgPool, packet: &mut Packet, begin: NaiveDateTime, end: NaiveDateTime) {
    // находим средний объём торгов за год
    let avg = pg::get_average_volume(pool, &packet.security, AvgPeriod::Year, begin.year()).await;
    let mut last_operation: Option<Uuid> = None;
    let candles = pg::get_candles(pool, &packet.security, begin, end, 200_000, &Frame::M1).await;
    let attempt = Attempt {
        id: Uuid::new_v4(),
        profit: 1.5,
        commission: 0.04,
    };
    pg::add_attempt(pool, &attempt).await;

    for candle in &candles {
        last_operation = strategy_1_logic(
            pool,
            packet,
            &candle,
            &attempt, //&mut wallet,
            avg,
            last_operation,
        )
        .await;
    }
}

async fn strategy_1_logic(
    pool: &PgPool,
    packet: &mut Packet,
    candle: &Candle,
    attempt: &Attempt, // wallet: &mut Wallet,
    avg: i32,
    prev: Option<Uuid>,
) -> Option<Uuid> {
    if packet.purchased > 0 {
        // выходим
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
    // находим точку входа
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
