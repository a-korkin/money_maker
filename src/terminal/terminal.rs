use std::time::Duration;

use crate::db::pg;
use crate::models::common::Candle;
use chrono::Datelike;
use chrono::{NaiveDate, NaiveDateTime, Timelike};
use raylib::prelude::GuiControlProperty::*;
use raylib::prelude::GuiTextAlignment::*;
use raylib::prelude::*;
use sqlx::PgPool;

const H: f32 = 320.0; //480.0;
const W: f32 = 1280.0;
const CANDLE_W: f32 = 12.0;
const COUNT_Y: f32 = 10.0;
const DATE_TIME_FMT: &str = "%Y-%m-%d %H:%M:%S";

#[allow(dead_code)]
struct DrawCoords {
    start_pos: Vector2,
    end_pos: Vector2,
    step_y: f32,
    min_y: f32,
    max_y: f32,
}

enum Period {
    Hour,
}

pub async fn run_terminal(pool: &PgPool) {
    let begin = NaiveDate::from_ymd_opt(2025, 3, 10)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let end = begin + Duration::from_secs(60 * 60 * 24 * 10);
    let securities = pg::get_securities_str(pool).await;
    let secs: Vec<&str> = securities.split(";").collect();
    let mut selected_security = secs[0];

    let (mut candles, mut coords) = fetch_data(pool, selected_security, begin, end).await;

    // ui
    let alpha = 1.0;
    let mut securities_edit = false;
    let mut securities_active: i32 = 0;
    let mut begin_str: String = begin.format(DATE_TIME_FMT).to_string();
    let mut begin_edit = false;
    let mut end_str: String = end.format(DATE_TIME_FMT).to_string();
    let mut end_edit = false;

    let (mut rl, thread) = raylib::init()
        .size(W as i32, H as i32)
        .title("Trading terminal")
        .build();

    rl.set_target_fps(60);
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::WHITE);
        d.gui_set_alpha(alpha);

        if securities_edit {
            d.gui_lock();
        }

        d.draw_text_ex(
            d.get_font_default(),
            "BEGIN",
            Vector2::new(25.0, 90.0),
            10.0,
            1.0,
            Color::BLACK,
        );
        if d.gui_text_box(
            Rectangle::new(25.0, 100.0, 125.0, 30.0),
            &mut begin_str,
            begin_edit,
        ) {
            begin_edit = !begin_edit;
        }

        d.draw_text_ex(
            d.get_font_default(),
            "END",
            Vector2::new(25.0, 135.0),
            10.0,
            1.0,
            Color::BLACK,
        );
        if d.gui_text_box(
            Rectangle::new(25.0, 145.0, 125.0, 30.0),
            &mut end_str,
            end_edit,
        ) {
            end_edit = !end_edit;
        }

        d.gui_unlock();
        d.gui_set_style(
            GuiControl::DROPDOWNBOX,
            TEXT_ALIGNMENT,
            TEXT_ALIGN_CENTER as i32,
        );
        if d.gui_dropdown_box(
            Rectangle::new(25.0, 25.0, 125.0, 30.0),
            &securities,
            &mut securities_active,
            securities_edit,
        ) {
            securities_edit = !securities_edit;
            if secs[securities_active as usize] != selected_security {
                selected_security = secs[securities_active as usize];
                (candles, coords) = fetch_data(pool, selected_security, begin, end).await;
            }
        }

        draw_axis(&mut d, &coords, Period::Hour);
        draw_candles(&mut d, &coords, &candles);
    }
}

async fn fetch_data<'a>(
    pool: &'a PgPool,
    security: &'a str,
    begin: NaiveDateTime,
    end: NaiveDateTime,
) -> (Vec<Candle>, DrawCoords) {
    let start_pos = Vector2::new(300.0, 20.0);
    let end_pos = Vector2::new(W - 20.0, 240.0 - 20.0);

    let limit = ((end_pos.x - start_pos.x) / CANDLE_W) as i32 - 1;

    let candles = pg::get_candles(pool, &security, begin, end, limit).await;
    let mut min_low: f32 = candles.first().unwrap().low;
    let mut max_high: f32 = 0_f32;

    for candle in candles.iter() {
        if candle.low < min_low {
            min_low = candle.low;
        }
        if candle.high > max_high {
            max_high = candle.high;
        }
    }

    let min_y = f32::floor(min_low);
    let max_y = f32::ceil(max_high);
    let step_y = (end_pos.y - start_pos.y) / (max_y - min_y);

    let coords = DrawCoords {
        start_pos,
        end_pos,
        step_y,
        min_y,
        max_y,
    };

    return (candles, coords);
}

fn draw_axis(d: &mut RaylibDrawHandle, coords: &DrawCoords, _period: Period) {
    let center = (coords.end_pos.x - coords.start_pos.x) / 2.0;
    // y-axis
    d.draw_line_v(
        coords.start_pos,
        Vector2::new(coords.start_pos.x, coords.end_pos.y + 1_f32),
        Color::BLACK,
    );

    let mut cur_y = coords.start_pos.y;
    let step = (coords.end_pos.y - coords.start_pos.y) / COUNT_Y;
    let add = (coords.max_y - coords.min_y) / 10.0;
    let mut label: f32 = coords.max_y;
    while cur_y <= coords.end_pos.y {
        d.draw_line_v(
            Vector2::new(coords.start_pos.x, cur_y),
            Vector2::new(coords.start_pos.x + 5_f32, cur_y),
            Color::BLACK,
        );
        d.draw_line_v(
            Vector2::new(coords.start_pos.x, cur_y),
            Vector2::new(coords.start_pos.x - 6_f32, cur_y),
            Color::BLACK,
        );
        let offset = match label {
            0.0..1000.0 => 40.0,
            1000.0..10_000.0 => 45.0,
            _ => 50.0,
        };
        d.draw_text_ex(
            d.get_font_default(),
            &format!("{:.2}", label),
            Vector2::new(coords.start_pos.x - offset, cur_y - 5_f32),
            10.0,
            1.0,
            Color::BLACK,
        );
        cur_y += step;
        label -= add;
    }

    // x-axis
    d.draw_line_v(
        Vector2::new(coords.start_pos.x, coords.end_pos.y),
        coords.end_pos,
        Color::BLACK,
    );

    let mut right = center;
    let mut left = center;
    let mut i = 0;
    while right <= coords.end_pos.x {
        let scale = if i % 4 == 0 { 5_f32 } else { 3_f32 };
        d.draw_line_v(
            Vector2::new(right, coords.end_pos.y + scale),
            Vector2::new(right, coords.end_pos.y - scale),
            Color::BLACK,
        );
        if left >= coords.start_pos.x {
            d.draw_line_v(
                Vector2::new(left, coords.end_pos.y + scale),
                Vector2::new(left, coords.end_pos.y - scale),
                Color::BLACK,
            );
        }

        right += CANDLE_W;
        left -= CANDLE_W;
        i += 1;
    }
}

fn convert_coords(start_pos: Vector2, step_y: f32, max_y: f32, in_value_y: f32) -> f32 {
    (max_y - in_value_y) * step_y + start_pos.y
}

fn draw_candles(d: &mut RaylibDrawHandle, coords: &DrawCoords, candles: &Vec<Candle>) {
    let mut day: u32 = 0;
    for (i, candle) in candles.iter().enumerate() {
        let x = coords.start_pos.x + (i as f32 * CANDLE_W);
        draw_candle(
            d,
            candle,
            x + CANDLE_W,
            coords.start_pos,
            coords.step_y,
            coords.max_y,
        );

        // print time labels of x-axis
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
                Vector2::new(x + offset, coords.end_pos.y + 8.0),
                10.0,
                1.0,
                Color::BLACK,
            );
        }
        let day_label = candle.begin.format("%Y-%m-%d").to_string();
        let current_day = candle.begin.day();
        if current_day != day {
            day = current_day;
            d.draw_text_ex(
                d.get_font_default(),
                &day_label,
                Vector2::new(x - 14.0, coords.end_pos.y + 20.0),
                10.0,
                1.0,
                Color::BLACK,
            );
        }
    }
}

fn draw_candle(
    d: &mut RaylibDrawHandle,
    candle: &Candle,
    idx_pos: f32,
    start_pos: Vector2,
    step_y: f32,
    max_y: f32,
) {
    let max = f32::max(candle.close, candle.open);
    let min = f32::min(candle.close, candle.open);
    let color = if candle.close >= candle.open {
        Color::GREEN
    } else {
        Color::RED
    };
    let pos = Vector2::new(idx_pos, convert_coords(start_pos, step_y, max_y, max));
    let size = Vector2::new(CANDLE_W, (max - min) * step_y);
    d.draw_rectangle_v(pos, size, color);
    let high = Vector2::new(
        idx_pos + CANDLE_W / 2.0,
        convert_coords(start_pos, step_y, max_y, candle.high),
    );
    let low = Vector2::new(
        idx_pos + CANDLE_W / 2.0,
        convert_coords(start_pos, step_y, max_y, candle.low),
    );
    d.draw_line_v(high, low, color);
}
