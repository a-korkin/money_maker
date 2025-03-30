use crate::db::pg;
use crate::models::common::Candle;
use chrono::{NaiveDate, NaiveDateTime};
use raylib::prelude::*;
use sqlx::PgPool;

const H: f32 = 480.0;
const W: f32 = 640.0;
const PAD: f32 = 20.0;

pub async fn run_terminal(pool: &PgPool) {
    let date = NaiveDate::from_ymd_opt(2025, 3, 25)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let candles = pg::get_candles(pool, "MOEX", date).await;

    let mut min_date: chrono::NaiveDateTime = candles.first().unwrap().begin;
    let mut max_date: NaiveDateTime = date;
    let mut min_low: f32 = candles.first().unwrap().low;
    let mut max_high: f32 = 0_f32;

    for candle in candles.iter() {
        if candle.begin < min_date {
            min_date = candle.begin;
        }
        if candle.end > max_date {
            max_date = candle.end;
        }
        if candle.low < min_low {
            min_low = candle.low;
        }
        if candle.high > max_high {
            max_high = candle.high;
        }
    }

    // println!("{:?},{:?},{:?},{:?}", min_date, max_date, min_low, max_high);

    // let times: Vec<(NaiveDateTime, NaiveDateTime)> =
    //     candles.iter().map(|m| (m.begin, m.end)).collect();
    //
    // for time in times.iter().step_by(10) {
    //     println!("time: {:?}", time);
    // }

    let min_y = f32::floor(min_low);
    let max_y = f32::ceil(max_high);
    let plot_pos_start = Vector2::new(40.0, 20.0);
    let plot_pos_end = Vector2::new(W - 20.0, H - 20.0);

    let (mut rl, thread) = raylib::init()
        .size(W as i32, H as i32)
        .title("Trading terminal")
        .build();

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::WHITE);

        draw_axis(
            &mut d,
            &plot_pos_start,
            &plot_pos_end,
            min_y,
            max_y,
            &candles,
        );
    }
}

fn draw_axis(
    d: &mut RaylibDrawHandle,
    start_pos: &Vector2,
    end_pos: &Vector2,
    min_y: f32,
    max_y: f32,
    _candles: &Vec<Candle>,
) {
    // y-axis
    d.draw_line_v(
        start_pos,
        Vector2::new(start_pos.x, end_pos.y),
        Color::BLACK,
    );

    let mut cur_y = start_pos.y;
    let step_y = (end_pos.y - start_pos.y) / (max_y - min_y);
    let mut label_y = max_y;
    while cur_y <= end_pos.y {
        d.draw_line_v(
            Vector2::new(start_pos.x, cur_y),
            Vector2::new(start_pos.x + 5_f32, cur_y),
            Color::BLACK,
        );
        d.draw_line_v(
            Vector2::new(start_pos.x, cur_y),
            Vector2::new(start_pos.x - 6_f32, cur_y),
            Color::BLACK,
        );

        d.draw_text_ex(
            d.get_font_default(),
            &label_y.to_string(),
            Vector2::new(start_pos.x - 25_f32, cur_y - 5_f32),
            10.0,
            1.0,
            Color::BLACK,
        );
        cur_y += step_y;
        label_y -= 1.0;
    }

    // x-axis
    d.draw_line_v(Vector2::new(start_pos.x, end_pos.y), end_pos, Color::BLACK);
}
