use crate::db::pg;
use crate::models::common::{Candle, Frame, TradeView};
use chrono::Datelike;
use chrono::{NaiveDate, NaiveDateTime, Timelike};
use raylib::prelude::GuiControlProperty::*;
use raylib::prelude::GuiTextAlignment::*;
use raylib::prelude::*;
use regex::Regex;
use sqlx::PgPool;
use std::time::Duration;

const H: f32 = 640.0;
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

#[allow(dead_code)]
struct UiElements<'a> {
    securities: &'a str,
    selected_security: &'a str,
    secs: Vec<&'a str>,
    securities_edit: bool,
    securities_active: i32,
    begin_str: String,
    begin_edit: bool,
    end_str: String,
    end_edit: bool,
}

pub async fn run_terminal(pool: &PgPool) {
    let mut begin = NaiveDate::from_ymd_opt(2025, 4, 25)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let mut end = begin + Duration::from_secs(60 * 60 * 24 * 10);

    let securities = pg::get_securities_str(pool).await;
    let secs: Vec<&str> = securities.split(";").collect();
    let selected_security = secs[0];

    let frames_str = "m1;h1;d1";
    let frames = &frames_str.split(";").collect::<Vec<&str>>();
    let mut frame_active: i32 = 1;
    let mut current_frame = frames[frame_active as usize];
    let mut frame_edit: bool = false;

    let (mut candles, mut coords) = fetch_data(
        pool,
        selected_security,
        begin,
        end,
        &Frame::from(current_frame),
    )
    .await;

    let b = NaiveDateTime::parse_from_str("2025-04-26 10:00:00", DATE_TIME_FMT).unwrap();
    let trades = pg::get_trades(pool, selected_security, b, end).await;

    // ui
    let alpha = 1.0;
    let mut ui = UiElements {
        securities: &securities,
        selected_security,
        secs,
        securities_edit: false,
        securities_active: 0,
        begin_str: format!("{} ", begin.format(DATE_TIME_FMT)),
        begin_edit: false,
        end_str: format!("{} ", end.format(DATE_TIME_FMT)),
        end_edit: false,
    };

    let (mut rl, thread) = raylib::init()
        .size(W as i32, H as i32)
        .title("Trading terminal")
        .build();

    rl.set_target_fps(60);
    let font = rl
        .load_font(&thread, "assets/fonts/SourceCodePro-Bold.ttf")
        // .load_font(&thread, "assets/fonts/JetBrainsMono-Bold.ttf")
        .expect("failed to load font");
    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::WHITE);
        d.gui_set_alpha(alpha);

        //draw ui
        if ui.securities_edit || frame_edit {
            d.gui_lock();
        }

        if draw_datepicker(
            &mut d,
            Vector2::new(25.0, 90.0),
            &mut ui.begin_str,
            &mut ui.begin_edit,
            "BEGIN",
            &mut begin,
        ) {
            (candles, coords) = fetch_data(
                pool,
                ui.selected_security,
                begin,
                end,
                &Frame::from(current_frame),
            )
            .await;
        }

        if draw_datepicker(
            &mut d,
            Vector2::new(25.0, 135.0),
            &mut ui.end_str,
            &mut ui.end_edit,
            "END",
            &mut end,
        ) {
            (candles, coords) = fetch_data(
                pool,
                ui.selected_security,
                begin,
                end,
                &Frame::from(current_frame),
            )
            .await;
        }

        if draw_dropdown(
            &mut d,
            ui.securities,
            &mut ui.securities_active,
            &mut ui.securities_edit,
            Rectangle::new(25.0, 25.0, 80.0, 30.0),
        ) {
            ui.securities_edit = !ui.securities_edit;
            if ui.secs[ui.securities_active as usize] != ui.selected_security {
                ui.selected_security = ui.secs[ui.securities_active as usize];
                (candles, coords) = fetch_data(
                    pool,
                    ui.selected_security,
                    begin,
                    end,
                    &Frame::from(current_frame),
                )
                .await;
            }
        }

        if draw_dropdown(
            &mut d,
            frames_str,
            &mut frame_active,
            &mut frame_edit,
            Rectangle::new(110.0, 25.0, 80.0, 30.0),
        ) {
            frame_edit = !frame_edit;
            if frames[frame_active as usize] != current_frame {
                current_frame = frames[frame_active as usize];
                (candles, coords) = fetch_data(
                    pool,
                    ui.selected_security,
                    begin,
                    end,
                    &Frame::from(current_frame),
                )
                .await;
            }
        }

        draw_axis(&mut d, &font, &coords);
        draw_candles(&mut d, &coords, &mut candles, &Frame::from(current_frame));
        draw_trades(&mut d, &font, &candles, &trades, &coords);
    }
}

async fn fetch_data<'a>(
    pool: &'a PgPool,
    security: &'a str,
    begin: NaiveDateTime,
    end: NaiveDateTime,
    frame: &Frame,
) -> (Vec<Candle>, DrawCoords) {
    let start_pos = Vector2::new(300.0, 20.0);
    let end_pos = Vector2::new(W - 20.0, 240.0 - 20.0);

    let limit = ((end_pos.x - start_pos.x) / CANDLE_W) as i32 - 1;

    let candles = pg::get_candles(pool, &security, begin, end, limit, frame).await;
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

fn draw_axis(d: &mut RaylibDrawHandle, font: &Font, coords: &DrawCoords) {
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
            font, //d.get_font_default(),
            &format!("{:.2}", label),
            Vector2::new(coords.start_pos.x - offset, cur_y - 5_f32),
            15.0,
            0.0,
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

fn draw_candles(
    d: &mut RaylibDrawHandle,
    coords: &DrawCoords,
    candles: &mut Vec<Candle>,
    frame: &Frame,
) {
    let mut day: u32 = 0;
    let mut month: u32 = 0;
    for (i, candle) in candles.into_iter().enumerate() {
        let x = coords.start_pos.x + (i as f32 * CANDLE_W);
        draw_candle(
            d,
            candle,
            x + CANDLE_W,
            coords.start_pos,
            coords.step_y,
            coords.max_y,
        );

        // print time labels on x-axis
        match frame {
            Frame::M1 => {
                draw_frames_m1(d, candle.begin, &mut day, Vector2::new(x, coords.end_pos.y))
            }
            Frame::H1 => {
                draw_frames_h1(d, candle.begin, &mut day, Vector2::new(x, coords.end_pos.y))
            }
            Frame::D1 => draw_frame_d1(
                d,
                candle.begin,
                &mut month,
                Vector2::new(x, coords.end_pos.y),
            ),
        }
    }
}

fn draw_candle(
    d: &mut RaylibDrawHandle,
    candle: &mut Candle,
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
    candle.position_x = pos.x;
    candle.position_y = pos.y;
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

fn draw_frames_m1(
    d: &mut RaylibDrawHandle,
    date: NaiveDateTime,
    hour: &mut u32,
    position: Vector2,
) {
    let minute = date.minute();
    let offset = match minute {
        0..=9 => 12.0,
        10..=19 => 14.0,
        _ => 12.0,
    };

    if minute % 2 == 0 {
        d.draw_text_ex(
            d.get_font_default(),
            &format!("{:02}", minute),
            Vector2::new(position.x + offset, position.y + 8.0),
            10.0,
            1.0,
            Color::BLACK,
        );
    }

    let current_hour = date.hour();
    if current_hour != *hour && current_hour != 6 {
        *hour = current_hour;

        let offset = match current_hour {
            0..=9 => 12.0,
            10..=19 => 14.0,
            _ => 12.0,
        };
        d.draw_text_ex(
            d.get_font_default(),
            &format!("{:02}", current_hour),
            Vector2::new(position.x + offset, position.y + 20.0),
            10.0,
            1.0,
            Color::BLACK,
        );
    }
}

fn draw_frame_d1(
    d: &mut RaylibDrawHandle,
    date: NaiveDateTime,
    month: &mut u32,
    position: Vector2,
) {
    let day = date.day();
    let offset = match day {
        1..=9 => 16.0,
        10..=19 => 14.0,
        _ => 12.0,
    };

    if day == 1 || day % 2 == 0 {
        d.draw_text_ex(
            d.get_font_default(),
            &day.to_string(),
            Vector2::new(position.x + offset, position.y + 8.0),
            10.0,
            1.0,
            Color::BLACK,
        );
    }

    let current_month = date.month();
    if current_month != *month {
        *month = current_month;
        d.draw_text_ex(
            d.get_font_default(),
            &date.format("%Y-%m").to_string(),
            Vector2::new(position.x - 8.0, position.y + 20.0),
            10.0,
            1.0,
            Color::BLACK,
        );
    }
}

fn draw_frames_h1(d: &mut RaylibDrawHandle, date: NaiveDateTime, day: &mut u32, position: Vector2) {
    let hour = date.hour();
    let offset = match hour {
        0..=9 => 15.0,
        10..=19 => 14.0,
        _ => 13.0,
    };
    if hour % 3 == 0 {
        d.draw_text_ex(
            d.get_font_default(),
            &hour.to_string(),
            Vector2::new(position.x + offset, position.y + 8.0),
            10.0,
            1.0,
            Color::BLACK,
        );
    }
    let current_day = date.day();
    if current_day != *day {
        *day = current_day;
        d.draw_text_ex(
            d.get_font_default(),
            &date.format("%Y-%m-%d").to_string(),
            Vector2::new(position.x - 14.0, position.y + 20.0),
            10.0,
            1.0,
            Color::BLACK,
        );
    }
}

#[allow(dead_code)]
async fn draw_ui<'a>(
    d: &mut RaylibDrawHandle<'a>,
    ui: &mut UiElements<'a>,
    pool: &'a PgPool,
    candles: &mut Vec<Candle>,
    coords: &mut DrawCoords,
    begin: NaiveDateTime,
    end: NaiveDateTime,
    frame: &Frame,
) {
    if ui.securities_edit {
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
        &mut ui.begin_str,
        ui.begin_edit,
    ) {
        ui.begin_edit = !ui.begin_edit;
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
        &mut ui.end_str,
        ui.end_edit,
    ) {
        ui.end_edit = !ui.end_edit;
    }

    d.gui_unlock();
    d.gui_set_style(
        GuiControl::DROPDOWNBOX,
        TEXT_ALIGNMENT,
        TEXT_ALIGN_CENTER as i32,
    );
    if d.gui_dropdown_box(
        Rectangle::new(25.0, 25.0, 125.0, 30.0),
        ui.securities,
        &mut ui.securities_active,
        ui.securities_edit,
    ) {
        ui.securities_edit = !ui.securities_edit;
        if ui.secs[ui.securities_active as usize] != ui.selected_security {
            ui.selected_security = ui.secs[ui.securities_active as usize];
            (*candles, *coords) = fetch_data(pool, ui.selected_security, begin, end, frame).await;
        }
    }
}

fn draw_datepicker(
    d: &mut RaylibDrawHandle,
    position: Vector2,
    ui_str: &mut String,
    ui_edit: &mut bool,
    label: &str,
    date: &mut NaiveDateTime,
) -> bool {
    d.draw_text_ex(
        d.get_font_default(),
        label,
        position,
        10.0,
        1.0,
        Color::BLACK,
    );
    if d.gui_text_box(
        Rectangle::new(position.x, position.y + 10.0, 125.0, 30.0),
        ui_str,
        *ui_edit,
    ) {
        *ui_edit = !*ui_edit;
        let re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}").unwrap();
        if re.is_match(ui_str) {
            match NaiveDateTime::parse_from_str(&ui_str[..19], DATE_TIME_FMT) {
                Ok(d) => {
                    *date = d;
                    return true;
                }
                Err(e) => {
                    println!("===================================");
                    println!("[ERROR]: {e}, value: {}", ui_str);
                }
            }
        }
    }

    false
}

fn draw_dropdown(
    d: &mut RaylibDrawHandle,
    list: &str,
    active: &mut i32,
    edit: &mut bool,
    position: Rectangle,
) -> bool {
    d.gui_unlock();
    d.gui_set_style(
        GuiControl::DROPDOWNBOX,
        TEXT_ALIGNMENT,
        TEXT_ALIGN_CENTER as i32,
    );
    d.gui_dropdown_box(position, list, active, *edit)
}

fn draw_trade(d: &mut RaylibDrawHandle, rect: &Rectangle, font: &Font, trade: &TradeView) {
    let color = match trade.buysell.to_uppercase().as_str() {
        "B" => Color::GREEN,
        "S" => Color::RED,
        _ => Color::BLUE,
    };
    d.draw_rectangle_rec(rect, color);
    d.draw_text_ex(
        font,
        &format!("{}: {}", trade.quantity, trade.price),
        Vector2::new(rect.x + 2.0, rect.y + 2.0),
        15.0,
        0.0,
        Color::BLACK,
    );
}

fn draw_trades(
    d: &mut RaylibDrawHandle,
    font: &Font,
    candles: &Vec<Candle>,
    trades: &Vec<TradeView>,
    coords: &DrawCoords,
) {
    let rect = Rectangle::new(300.0, 300.0, 80.0, 120.0);
    d.draw_rectangle_lines_ex(rect, 1.0, Color::BLACK);

    if d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
        let mouse_position = d.get_mouse_position();

        if mouse_position.x >= coords.start_pos.x
            && mouse_position.y >= coords.start_pos.y
            && mouse_position.x <= coords.end_pos.x
            && mouse_position.y <= coords.end_pos.y
        {
            for candle in candles {
                if mouse_position.x >= candle.position_x
                    && mouse_position.x <= candle.position_x + CANDLE_W
                {
                    println!("{:?}", candle);
                }
            }
        }
    }

    let height = 20.0;
    let mut current_y = 300.0;
    for trade in trades {
        draw_trade(
            d,
            &Rectangle::new(300.0, current_y, 80.0, height),
            font,
            trade,
        );
        current_y += height;
    }
}
