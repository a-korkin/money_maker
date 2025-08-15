use crate::db::pg;
use crate::db::repo;
use crate::models::common::{Attempt, AvgPeriod, Candle, Frame, Operation, OperationType, Packet};
use crate::models::common::{TradeInfo, TradeType};
use chrono::NaiveTime;
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use sqlx::postgres::PgPool;
use std::time::Duration;
use uuid::Uuid;

pub fn pretty_print_info(info: &TradeInfo) {
    let color = match info.get_type() {
        TradeType::Sell => "31",
        TradeType::Buy => "32",
    };
    let max = f32::max(info.open, info.close);
    let min = f32::min(info.open, info.close);
    let percent = f32::abs((max / (min / 100.0)) - 100.0);
    println!(
        "\x1b[{color}m{}\topen: {:.2}\tclose: {:.2}\tpercent: {:.2}\tquantity: {:.2}\tbuysell: {}\x1b[0m",
        info.begin, info.open, info.close, percent, info.sum_quantity, info.buysell,
    );
}

pub fn pretty_print_candle(candle: &Candle) {
    let color = match (candle.open, candle.close) {
        (o, c) if o > c => "31",
        (o, c) if o < c => "32",
        _ => "37",
    };
    let max = f32::max(candle.open, candle.close);
    let min = f32::min(candle.open, candle.close);
    let percent = (max / (min / 100.0)) - 100.0;
    println!(
        "\x1b[{color}m{}\topen: {:.2}\tclose: {:.2}\tpercent: {:.2}\x1b[0m",
        candle.begin, candle.open, candle.close, percent
    );
}

#[allow(dead_code)]
pub async fn trade_info(pool: &PgPool, security: &str, date: &NaiveDate) -> Vec<TradeInfo> {
    let info = repo::get_trade_info(pool, security, date).await;
    info
}

pub async fn best_choice(pool: &PgPool, security: &str, date: &NaiveDate) {
    let time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    let begin = NaiveDateTime::new(*date, time);
    let end = begin + Duration::from_secs(60 * 60 * 24 * 1);
    let candles = pg::get_candles(pool, security, begin, end, 10000, &Frame::M1).await;
    let candles = candles
        .iter()
        .enumerate()
        .map(|(a, b)| (a, b))
        .collect::<Vec<_>>();
    let inner = candles.clone();

    let trade_info = repo::get_trade_info(pool, security, date).await;

    let mut count: usize = 0;
    let mut current_inner = 0;

    let divider = format!("{:-<125}", "");
    println!("{divider}");
    println!(
        "{:>8} | {:<8} | {} | {:>10} | {:>10} | {:>8} | {:>8} | {:>13} | {:>13} | {:>13}",
        "start",
        "end",
        "percent",
        "before",
        "after",
        "buy",
        "sell",
        "quant",
        "< 5 min quant",
        "> 5 min quant",
    );
    println!("{divider}");

    for (i, x) in candles {
        if i < current_inner {
            continue;
        }
        if inner.len() >= i + 60 {
            let hour_candles = &inner[i..i + 60];
            for (j, y) in hour_candles {
                let percent = (y.close / (x.close / 100.0)) - 100.0;
                if percent >= 0.3 {
                    let scope_trades = trade_info
                        .iter()
                        .filter(|a| {
                            a.begin >= x.begin - Duration::from_secs(60 * 5)
                                && a.begin <= x.begin + Duration::from_secs(60 * 5)
                        })
                        .collect::<Vec<_>>();

                    let trades = scope_trades
                        .iter()
                        .filter(|a| a.begin == x.begin)
                        .collect::<Vec<_>>();

                    let sum_buy_5m_before = scope_trades
                        .iter()
                        .filter(|a| {
                            a.begin >= x.begin - Duration::from_secs(60 * 5)
                                && a.begin < x.begin
                                && a.get_type() == TradeType::Buy
                        })
                        .fold(0, |acc, t| acc + t.sum_quantity);
                    let sum_sell_5m_before = scope_trades
                        .iter()
                        .filter(|a| {
                            a.begin >= x.begin - Duration::from_secs(60 * 5)
                                && a.begin < x.begin
                                && a.get_type() == TradeType::Sell
                        })
                        .fold(0, |acc, t| acc + t.sum_quantity);

                    let sum_buy_5m_after = scope_trades
                        .iter()
                        .filter(|a| {
                            a.begin <= x.begin + Duration::from_secs(60 * 5)
                                && a.begin > x.begin
                                && a.get_type() == TradeType::Buy
                        })
                        .fold(0, |acc, t| acc + t.sum_quantity);
                    let sum_sell_5m_after = scope_trades
                        .iter()
                        .filter(|a| {
                            a.begin <= x.begin + Duration::from_secs(60 * 5)
                                && a.begin > x.begin
                                && a.get_type() == TradeType::Sell
                        })
                        .fold(0, |acc, t| acc + t.sum_quantity);

                    let mut buy_quantity = 0;
                    let mut sell_quantity = 0;
                    if trades.len() > 0 {
                        match trades.iter().find(|a| a.get_type() == TradeType::Buy) {
                            Some(t) => {
                                buy_quantity = t.sum_quantity;
                            }
                            None => {
                                buy_quantity = 0;
                            }
                        }
                        match trades.iter().find(|a| a.get_type() == TradeType::Sell) {
                            Some(t) => {
                                sell_quantity = t.sum_quantity;
                            }
                            None => {
                                sell_quantity = 0;
                            }
                        }
                    }

                    let (prefix, suffix) = match percent >= 0.3 {
                        true => ("\x1b[1;32m", "\x1b[0m"),
                        false => ("", ""),
                    };

                    println!(
                        "{}{} | {} | {:>7.2} | {:>10.2} | {:>10.2} | {:>8} | {:>8} | {:>13.2} | {:>13.2} | {:>13.2}{}",
                        prefix,
                        x.begin.format("%H:%M:%S"),
                        y.begin.format("%H:%M:%S"),
                        percent,
                        x.close,
                        y.close,
                        buy_quantity,
                        sell_quantity,
                        if buy_quantity == 0 {
                            1.0f32
                        } else {
                            buy_quantity as f32
                        } / if sell_quantity == 0 {
                            1.0f32
                        } else {
                            sell_quantity as f32
                        },
                        sum_buy_5m_before as f32 / sum_sell_5m_before as f32,
                        sum_buy_5m_after as f32 / sum_sell_5m_after as f32,
                        suffix,
                    );
                    count += 1;
                    current_inner = *j;
                    break;
                }
            }
        }
    }
    println!("count: {count}");
}

#[allow(dead_code)]
pub async fn run_strategy(pool: &PgPool) {
    let begin = NaiveDate::from_ymd_opt(2024, 6, 10)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    // let end = begin + Duration::from_secs(60 * 60 * 24 * 31);
    let end = NaiveDate::from_ymd_opt(2024, 6, 11)
        .unwrap()
        .and_hms_opt(0, 0, 0)
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

#[allow(dead_code)]
async fn strategy_3(pool: &PgPool, packet: &mut Packet, begin: NaiveDateTime, end: NaiveDateTime) {
    let candles = pg::get_candles(pool, &packet.security, begin, end, 200_000, &Frame::M1).await;
    let mut volume_up: f32 = 0.0;
    let mut volume_down: f32 = 0.0;

    let attempt = Attempt {
        id: Uuid::new_v4(),
        profit: 0.25,
        commission: 0.04,
    };
    pg::add_attempt(pool, &attempt).await;
    let mut last_operation: Option<Uuid> = None;

    let mut current_date = candles.first().unwrap().begin.date();
    let candles_skip: Vec<Candle> = candles.clone();
    // объём для OZON > 8000
    let break_volume: u32 = 1000 * 9;
    let mut volume_ok: bool = false;
    // let mut streak_up: u8 = 0;
    // let mut streak_down: u8 = 0;

    for (candle, next) in std::iter::zip(candles, candles_skip.iter().skip(1)) {
        if candle.begin.date() != current_date {
            current_date = candle.begin.date();
            volume_up = 0.0;
            volume_down = 0.0;
            volume_ok = false;
            // streak_up = 0;
            // streak_down = 0;
            // volume_all = 0.0;
        }
        if candle.close > candle.open {
            volume_up += candle.volume;
            // streak_up += 1;
            if candle.volume as u32 >= break_volume {
                volume_ok = false;
            }
        }
        if candle.close < candle.open {
            volume_down += candle.volume;
            // streak_down += 1;
            if candle.volume as u32 >= break_volume {
                volume_ok = true;
            }
        }

        // let diff = volume_up - volume_down;
        // let red_line = (volume_up - volume_down) / ((volume_up + volume_down) / 100.0);
        let percent = (f32::max(candle.open, candle.close)
            / (f32::min(candle.open, candle.close) / 100.0))
            - 100.0;
        let percent = if candle.open > candle.close {
            -1f32 * percent
        } else {
            percent
        };

        // println!(
        //     "time: {}, volume: {}, volume_up: {}, volume_down: {}, profit: {}",
        //     candle.begin, candle.volume, volume_up, volume_down, packet.profit,
        // );

        // let sold: bool =
        //     (candle.close >= packet.profit || red_line < 5.0) && candle.begin.hour() > 12;
        // let buy: bool = red_line > 5.0
        //     && candle.close >= candle.open + (candle.open / 100.0) * 0.3
        //     && candle.begin.hour() < 19;

        let buy: bool = volume_ok
            && percent >= 0.0
            && percent <= 0.001
            && candle.begin.hour() != 17
            && candle.begin.hour() != 18;
        let sold: bool = candle.close >= packet.profit;

        if buy {
            let operation = if buy { "buy" } else { "sold" };
            println!(
                "time: {}, operation: {}, profit: {}, balance: {}, percent: {}",
                candle.begin, operation, packet.profit, packet.balance, percent,
            );
        }

        last_operation =
            strategy_logic(pool, packet, next, &attempt, last_operation, sold, buy).await;
    }
}

#[allow(dead_code)]
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
        let sold: bool = candle.close >= packet.profit;
        let buy: bool = candle.begin.hour() >= 13
            && candle.begin.hour() < 19
            && candle.volume as i32 >= prev_avg * 5
            && candle.open > candle.close;
        last_operation =
            strategy_logic(pool, packet, &candle, &attempt, last_operation, sold, buy).await;
        if current_date != candle.begin.date() {
            current_date = candle.begin.date();
            prev_avg = (vol / i as f32) as i32;
        } else {
            i += 1;
            vol += candle.volume;
        }
    }
}

#[allow(dead_code)]
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
        let sold: bool = candle.close >= packet.profit;
        let buy: bool = candle.volume as i32 > avg && candle.open > candle.close;

        last_operation =
            strategy_logic(pool, packet, &candle, &attempt, last_operation, sold, buy).await;
    }
}

async fn strategy_logic(
    pool: &PgPool,
    packet: &mut Packet,
    candle: &Candle,
    attempt: &Attempt,
    prev: Option<Uuid>,
    sold: bool,
    buy: bool,
) -> Option<Uuid> {
    if packet.purchased > 0 {
        // выходим
        if sold {
            let commission: f32 =
                ((packet.purchased as f32 * candle.open) / 100.0) * attempt.commission;
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
    if buy {
        let mut count = f32::floor(packet.balance / (candle.open)) as i32;
        let commission: f32 = ((count as f32 * candle.open) / 100.0) * attempt.commission;
        while (count as f32 * candle.open) + commission > packet.balance {
            count -= 1;
        }
        count = (count / packet.min_count) * packet.min_count;

        if count == 0 {
            return prev;
        }
        packet.purchased += count;
        packet.profit = (candle.open / 100.0) * attempt.profit + candle.open;
        let op_id = create_operation(pool, attempt, "buy", packet, &commission, prev, candle).await;

        return Some(op_id);
    }
    return prev;
}

async fn create_operation(
    pool: &PgPool,
    attempt: &Attempt,
    op_type: &str,
    packet: &mut Packet,
    commission: &f32,
    prev: Option<Uuid>,
    candle: &Candle,
) -> Uuid {
    let id = Uuid::new_v4();
    let operation_type = OperationType::from(op_type);
    let mut sum_after: f32 = match operation_type {
        OperationType::Buy => packet.balance - (packet.purchased as f32 * candle.open),
        OperationType::Sold => packet.balance + (packet.purchased as f32 * candle.open),
    };
    sum_after = sum_after - *commission;
    let operation = Operation {
        id,
        attempt: attempt.id,
        operation_type,
        security: packet.security.clone(),
        count: packet.purchased,
        price: candle.open,
        commission: *commission,
        time_at: candle.begin,
        sum_before: packet.balance,
        sum_after,
    };
    packet.balance = sum_after;
    pg::add_operation(pool, &operation, prev).await;
    return operation.id;
}
