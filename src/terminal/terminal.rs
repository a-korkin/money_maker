use crate::db::pg;
use crate::models::common::Candle;
use chrono::{NaiveDate, NaiveDateTime, Timelike};
use raylib::prelude::*;
use sqlx::PgPool;

const H: f32 = 480.0;
const W: f32 = 896.0;
const CANDLE_W: f32 = 12.0;

pub async fn run_terminal(pool: &PgPool) {
    let date = NaiveDate::from_ymd_opt(2025, 3, 25)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let mut candles = pg::get_candles(pool, "MOEX", date).await;
    candles.sort_by(|a, b| a.begin.cmp(&b.begin));

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

    let min_y = f32::floor(min_low);
    let max_y = f32::ceil(max_high);
    let plot_pos_start = Vector2::new(40.0, 20.0);
    let plot_pos_end = Vector2::new(W - 20.0, 240.0 - 20.0);
    let step_y = (plot_pos_end.y - plot_pos_start.y) / (max_y - min_y);

    let (mut rl, thread) = raylib::init()
        .size(W as i32, H as i32)
        .title("Trading terminal")
        .build();

    rl.set_target_fps(60);
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::WHITE);

        draw_axis(
            &mut d,
            &plot_pos_start,
            &plot_pos_end,
            min_y,
            max_y,
            step_y,
            Period::Hour,
            &candles,
        );
    }
}

enum Period {
    Hour,
}

fn draw_axis(
    d: &mut RaylibDrawHandle,
    start_pos: &Vector2,
    end_pos: &Vector2,
    _min_y: f32,
    max_y: f32,
    step_y: f32,
    _period: Period,
    candles: &Vec<Candle>,
) {
    // y-axis
    d.draw_line_v(
        start_pos,
        Vector2::new(start_pos.x, end_pos.y + 1_f32),
        Color::BLACK,
    );

    let mut cur_y = start_pos.y;
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

    let center = (end_pos.x - start_pos.x) / 2.0;
    let half = f32::ceil(candles.len() as f32 / 2.0);
    let first_indx_pos: f32 = center - (half * CANDLE_W);

    let mut right = center;
    let mut left = center;
    let mut i = 0;
    while right <= end_pos.x {
        let scale = if i % 4 == 0 { 5_f32 } else { 3_f32 };
        d.draw_line_v(
            Vector2::new(right, end_pos.y + scale),
            Vector2::new(right, end_pos.y - scale),
            Color::BLACK,
        );
        if left >= start_pos.x {
            d.draw_line_v(
                Vector2::new(left, end_pos.y + scale),
                Vector2::new(left, end_pos.y - scale),
                Color::BLACK,
            );
        }

        right += CANDLE_W;
        left -= CANDLE_W;
        i += 1;
    }

    for (i, candle) in candles.iter().enumerate() {
        let x = first_indx_pos + (i as f32 * CANDLE_W);
        draw_candle(d, candle, x, start_pos, step_y, max_y);
        let hour = candle.begin.hour();
        let offset = match hour {
            0..=9 => 15.0,
            10..=19 => 14.0,
            _ => 13.0,
        };
        if hour % 3 == 0 {
            d.draw_text_ex(
                d.get_font_default(),
                &hour.to_string(),
                Vector2::new(x + offset, end_pos.y + 8.0),
                10.0,
                1.0,
                Color::BLACK,
            );
        }
    }
}

fn convert_coords(start_pos: Vector2, step_y: f32, max_y: f32, in_value_y: f32) -> f32 {
    (max_y - in_value_y) * step_y + start_pos.y
}

fn draw_candle(
    d: &mut RaylibDrawHandle,
    candle: &Candle,
    idx_pos: f32,
    start_pos: &Vector2,
    step_y: f32,
    max_y: f32,
) {
    let max = f32::max(candle.close, candle.open);
    let min = f32::min(candle.close, candle.open);
    let color = if candle.close > candle.open {
        Color::GREEN
    } else {
        Color::RED
    };
    let pos = Vector2::new(idx_pos, convert_coords(*start_pos, step_y, max_y, max));
    let size = Vector2::new(CANDLE_W, (max - min) * step_y);
    d.draw_rectangle_v(pos, size, color);
    let high = Vector2::new(
        idx_pos + CANDLE_W / 2.0,
        convert_coords(*start_pos, step_y, max_y, candle.high),
    );
    let low = Vector2::new(
        idx_pos + CANDLE_W / 2.0,
        convert_coords(*start_pos, step_y, max_y, candle.low),
    );
    d.draw_line_v(high, low, color);
}
