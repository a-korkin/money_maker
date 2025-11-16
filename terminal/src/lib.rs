use app::db::pg;
use app::models::common::{Candle, Frame, TradeView};
use chrono::{Datelike, NaiveDate, NaiveDateTime, Timelike};
use raylib::prelude::GuiControlProperty::*;
use raylib::prelude::GuiTextAlignment::*;
use raylib::prelude::*;
use regex::Regex;
use sqlx::PgPool;
use std::i64;
use std::time::Duration;

const H: f32 = 640.0;
const W: f32 = 1280.0;
const CANDLE_W: f32 = 12.0;
const COUNT_Y: f32 = 10.0;
const DATE_TIME_FMT: &str = "%Y-%m-%d %H:%M:%S";
const DATE_FMT: &str = "%Y-%m-%d";
const TRADES_DELTA_Y: f32 = 300.0;
const BACKGROUND_COLOR: Color = Color::new(23, 35, 46, 0);

#[allow(dead_code)]
#[derive(Debug)]
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
    let start_info = pg::get_start_info(pool).await;

    let securities = pg::get_securities_str(pool).await;
    let secs: Vec<&str> = securities.split(";").collect();
    let selected_security = &start_info.security_code; //secs[0];

    let frames_str = "m1;m15;h1;d1";
    let frames = &frames_str.split(";").collect::<Vec<&str>>();
    let mut frame_active: i32 = 1;
    let mut current_frame = frames[frame_active as usize];
    let mut frame_edit: bool = false;

    let mut date_active: i32 = 0;
    let mut date_edit: bool = false;
    let mut dates = &start_info.dates.split(";").collect::<Vec<&str>>();
    let mut current_date = dates[date_active as usize];
    let mut begin =
        NaiveDateTime::parse_from_str(&format!("{} 00:00:00", current_date), DATE_TIME_FMT)
            .unwrap();
    let mut end = begin + Duration::from_secs(60 * 60 * 24 * 1);

    let data = fetch_data(
        pool,
        selected_security,
        begin,
        end,
        &Frame::from(current_frame),
    )
    .await;

    if !data.is_some() {
        println!("Has no data for: {:#}, {}", begin, selected_security);
        return;
    }

    let (mut candles, mut coords) = data.unwrap();

    let mut trades = pg::get_trades_view(
        pool,
        selected_security,
        begin,
        end,
        &Frame::from(current_frame),
        candles.len() as i32,
    )
    .await;

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
        .load_font(&thread, "terminal/assets/fonts/SourceCodePro-Bold.ttf")
        .expect("failed to load font");
    let mut info = String::from("");
    let mut current_candle = candles.first().unwrap().clone();

    rl.gui_set_font(&font);
    rl.gui_set_style(GuiControl::DEFAULT, GuiDefaultProperty::TEXT_SIZE, 15);

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);
        d.clear_background(BACKGROUND_COLOR);
        d.gui_set_alpha(alpha);

        //draw ui
        if ui.securities_edit || frame_edit {
            d.gui_lock();
        }

        // if draw_datepicker(
        //     &mut d,
        //     Vector2::new(25.0, 90.0),
        //     &mut ui.begin_str,
        //     &mut ui.begin_edit,
        //     "BEGIN",
        //     &mut begin,
        //     &font,
        // ) {
        //     (candles, coords) = fetch_data(
        //         pool,
        //         ui.selected_security,
        //         begin,
        //         end,
        //         &Frame::from(current_frame),
        //     )
        //     .await
        //     .unwrap();
        // }

        // if draw_datepicker(
        //     &mut d,
        //     Vector2::new(25.0, 135.0),
        //     &mut ui.end_str,
        //     &mut ui.end_edit,
        //     "END",
        //     &mut end,
        //     &font,
        // ) {
        //     (candles, coords) = fetch_data(
        //         pool,
        //         ui.selected_security,
        //         begin,
        //         end,
        //         &Frame::from(current_frame),
        //     )
        //     .await
        //     .unwrap();
        // }

        if draw_dropdown(
            &mut d,
            ui.securities,
            &mut ui.securities_active,
            &mut ui.securities_edit,
            Rectangle::new(25.0, 25.0, 80.0, 30.0),
            false,
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
                .await
                .unwrap();
            }
        }

        if draw_dropdown(
            &mut d,
            frames_str,
            &mut frame_active,
            &mut frame_edit,
            Rectangle::new(110.0, 25.0, 80.0, 30.0),
            false,
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
                .await
                .unwrap();
            }
        }

        let bounds = Rectangle::new(25.0, 200.0, 130.0, 30.0);
        if draw_dropdown(
            &mut d,
            &start_info.dates,
            &mut date_active,
            &mut date_edit,
            bounds,
            true,
        ) {
            date_edit = !date_edit;
            // if date_edit {
            let scrollPanelBounds: Rectangle =
                Rectangle::new(bounds.x, bounds.y + bounds.height, bounds.width, 150.0);

            let content = Rectangle::new(bounds.x, bounds.y + bounds.height, bounds.width, 200.0);
            let scroll = Vector2::new(bounds.x, bounds.y);
            let view = Rectangle::new(bounds.x, bounds.y + bounds.height, bounds.width, 150.0);
            let _ = d.gui_scroll_panel(scrollPanelBounds, &start_info.dates, content, scroll, view);
            // }
            if dates[date_active as usize] != current_date {
                current_date = dates[date_active as usize];
                begin = NaiveDateTime::parse_from_str(
                    &format!("{} 00:00:00", current_date),
                    DATE_TIME_FMT,
                )
                .unwrap();
                end = begin + Duration::from_secs(60 * 60 * 24 * 1);
                (candles, coords) = fetch_data(
                    pool,
                    ui.selected_security,
                    begin,
                    end,
                    &Frame::from(current_frame),
                )
                .await
                .unwrap();
            }
            // let position = Rectangle::new(35.0, 200.0, 130.0, 30.0);
            // let mut bounds = position;
            // bounds.height = 500.0;
            // let scroll = Vector2::new(bounds.x, bounds.y);
            // d.gui_scroll_panel(bounds, "", bounds, scroll, bounds);
        }

        // candles
        draw_axis(&mut d, &font, &coords);
        draw_graphs(
            &mut d,
            &coords,
            &mut candles,
            &Frame::from(current_frame),
            &font,
            &current_candle,
        );

        // trades
        draw_trades(&mut d, &font, &trades, &coords, &Frame::from(current_frame));

        if mouse_click(&mut d, &coords, &candles, &mut current_candle, &mut info) {
            // trades = pg::get_trades_view(
            //     pool,
            //     selected_security,
            //     current_candle.begin,
            //     current_candle.end,
            //     &Frame::from(current_frame),
            //     candles.len() as i32,
            // )
            // .await;
        }

        draw_info(&mut d, &coords, &font, &info);
        // draw_arrow(&mut d, &mut current_candle);
    }
}

async fn fetch_data<'a>(
    pool: &'a PgPool,
    security: &'a str,
    begin: NaiveDateTime,
    end: NaiveDateTime,
    frame: &Frame,
) -> Option<(Vec<Candle>, DrawCoords)> {
    let start_pos = Vector2::new(300.0, 20.0);
    let end_pos = Vector2::new(W - 20.0, 240.0 - 20.0);
    let limit = ((end_pos.x - start_pos.x) / CANDLE_W) as i32 - 1;
    let candles = pg::get_candles(pool, &security, begin, end, limit, frame).await;
    if candles.is_empty() {
        return None;
    }
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

    return Some((candles, coords));
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
            0.0..1000.0 => 50.0,
            1000.0..10_000.0 => 52.0,
            _ => 50.0,
        };
        d.draw_text_ex(
            font,
            &format!("{:.2}", label),
            Vector2::new(coords.start_pos.x - offset, cur_y - 8_f32),
            15.0,
            0.0,
            Color::WHEAT,
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

fn convert_coords_y(start: f32, step: f32, max: f32, value: f32) -> f32 {
    (max - value) * step + start
}

fn draw_graphs(
    d: &mut RaylibDrawHandle,
    coords: &DrawCoords,
    candles: &mut Vec<Candle>,
    frame: &Frame,
    font: &Font,
    current_candle: &Candle,
) {
    let y = coords.end_pos.y;
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
            current_candle,
        );

        // print time labels on x-axis
        match frame {
            Frame::M1 => draw_frames_m1(d, candle.begin, &mut day, Vector2::new(x, y), font),
            Frame::M15 => draw_frames_m1(d, candle.begin, &mut day, Vector2::new(x, y), font),
            Frame::H1 => draw_frames_h1(d, candle.begin, &mut day, Vector2::new(x, y), font),
            Frame::D1 => draw_frames_d1(d, candle.begin, &mut month, Vector2::new(x, y), font),
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
    current_candle: &Candle,
) {
    let max = f32::max(candle.close, candle.open);
    let min = f32::min(candle.close, candle.open);
    let color = if candle.begin == current_candle.begin {
        Color::WHEAT
    } else if candle.close >= candle.open {
        Color::GREEN
    } else {
        Color::RED
    };
    let pos = Vector2::new(idx_pos, convert_coords_y(start_pos.y, step_y, max_y, max));
    let size = Vector2::new(CANDLE_W, (max - min) * step_y);
    d.draw_rectangle_v(pos, size, color);
    candle.position_x = Some(pos.x);
    candle.position_y = Some(pos.y);
    let high = Vector2::new(
        idx_pos + CANDLE_W / 2.0,
        convert_coords_y(start_pos.y, step_y, max_y, candle.high),
    );
    let low = Vector2::new(
        idx_pos + CANDLE_W / 2.0,
        convert_coords_y(start_pos.y, step_y, max_y, candle.low),
    );
    d.draw_line_v(high, low, color);
}

fn draw_frames_m1(
    d: &mut RaylibDrawHandle,
    date: NaiveDateTime,
    hour: &mut u32,
    position: Vector2,
    font: &Font,
) {
    let minute = date.minute();
    let offset = 10.5;

    if minute % 2 == 0 {
        d.draw_text_ex(
            font,
            &format!("{:02}", minute),
            Vector2::new(position.x + offset, position.y + 8.0),
            15.0,
            0.0,
            Color::WHEAT,
        );
    }

    let current_hour = date.hour();
    if current_hour != *hour && current_hour != 6 {
        *hour = current_hour;
        d.draw_text_ex(
            font,
            &format!("{:02}", current_hour),
            Vector2::new(position.x + offset, position.y + 20.0),
            15.0,
            0.0,
            Color::WHEAT,
        );
    }
}

fn draw_frames_d1(
    d: &mut RaylibDrawHandle,
    date: NaiveDateTime,
    month: &mut u32,
    position: Vector2,
    font: &Font,
) {
    let day = date.day();
    let offset = match day {
        1..=9 => 16.0,
        10..=19 => 14.0,
        _ => 12.0,
    };

    if day == 1 || day % 2 == 0 {
        d.draw_text_ex(
            font,
            &day.to_string(),
            Vector2::new(position.x + offset, position.y + 8.0),
            15.0,
            1.0,
            Color::WHEAT,
        );
    }

    let current_month = date.month();
    if current_month != *month {
        *month = current_month;
        d.draw_text_ex(
            font,
            &date.format("%Y-%m").to_string(),
            Vector2::new(position.x - 8.0, position.y + 20.0),
            15.0,
            1.0,
            Color::WHEAT,
        );
    }
}

fn draw_frames_h1(
    d: &mut RaylibDrawHandle,
    date: NaiveDateTime,
    day: &mut u32,
    position: Vector2,
    font: &Font,
) {
    let hour = date.hour();
    let offset = match hour {
        0..=9 => 15.0,
        10..=19 => 14.0,
        _ => 13.0,
    };
    if hour % 3 == 0 {
        d.draw_text_ex(
            font,
            &hour.to_string(),
            Vector2::new(position.x + offset, position.y + 8.0),
            15.0,
            1.0,
            Color::WHEAT,
        );
    }
    let current_day = date.day();
    if current_day != *day {
        *day = current_day;
        d.draw_text_ex(
            font,
            &date.format("%Y-%m-%d").to_string(),
            Vector2::new(position.x - 14.0, position.y + 20.0),
            15.0,
            1.0,
            Color::WHEAT,
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
        Color::WHEAT,
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
        Color::WHEAT,
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
            (*candles, *coords) = fetch_data(pool, ui.selected_security, begin, end, frame)
                .await
                .unwrap();
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
    font: &Font,
) -> bool {
    d.draw_text_ex(
        font,
        label,
        Vector2::new(position.x, position.y - 5.0),
        15.0,
        0.0,
        Color::WHEAT,
    );
    if d.gui_text_box(
        Rectangle::new(position.x, position.y + 10.0, 186.0, 30.0),
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
    scrolled: bool,
) -> bool {
    d.gui_unlock();
    d.gui_set_style(
        GuiControl::DROPDOWNBOX,
        TEXT_ALIGNMENT,
        TEXT_ALIGN_CENTER as i32,
    );
    let result = d.gui_dropdown_box(position, list, active, *edit);
    result
}

fn draw_trades(
    d: &mut RaylibDrawHandle,
    font: &Font,
    trades: &Vec<TradeView>,
    coords: &DrawCoords,
    frame: &Frame,
) {
    let end_y = coords.end_pos.y + TRADES_DELTA_Y;
    let start_y = coords.start_pos.y + TRADES_DELTA_Y;
    let mut min_y = i64::MAX;
    let mut max_y = 0_i64;

    for trade in trades {
        let min = i64::min(trade.quantity_buy, trade.quantity_sell);
        let max = i64::max(trade.quantity_buy, trade.quantity_sell);

        if min < min_y {
            min_y = min;
        }
        if max > max_y {
            max_y = max;
        }
    }

    // draw y-axis
    d.draw_line_v(
        Vector2::new(coords.start_pos.x, start_y),
        Vector2::new(coords.start_pos.x, end_y + 1_f32),
        Color::BLACK,
    );

    let mut cur_y = start_y;
    let step = (end_y - start_y) / COUNT_Y;
    let add = ((max_y as f64 - min_y as f64) / 10.0) as i64;
    let mut label: i64 = max_y;
    while cur_y <= end_y {
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
            0..1000 => 40.0,
            1000..10_000 => 45.0,
            _ => 50.0,
        };
        d.draw_text_ex(
            font,
            &format!("{:.2}", label),
            Vector2::new(coords.start_pos.x - offset, cur_y - 8_f32),
            15.0,
            0.0,
            Color::WHEAT,
        );
        cur_y += step;
        label -= add;
    }

    // draw x-axis
    let center = (coords.end_pos.x - coords.start_pos.x) / 2.0;
    d.draw_line_v(
        Vector2::new(coords.start_pos.x, end_y),
        Vector2::new(coords.end_pos.x, end_y),
        Color::BLACK,
    );

    let mut right = center;
    let mut left = center;
    let mut i = 0;
    while right <= coords.end_pos.x {
        let scale = if i % 4 == 0 { 5_f32 } else { 3_f32 };
        d.draw_line_v(
            Vector2::new(right, end_y + scale),
            Vector2::new(right, end_y - scale),
            Color::BLACK,
        );
        if left >= coords.start_pos.x {
            d.draw_line_v(
                Vector2::new(left, end_y + scale),
                Vector2::new(left, end_y - scale),
                Color::BLACK,
            );
        }

        right += CANDLE_W;
        left -= CANDLE_W;
        i += 1;
    }

    let y = coords.end_pos.y + TRADES_DELTA_Y;
    let mut day: u32 = 0;
    let mut month: u32 = 0;

    let step_y = (end_y - start_y) / (max_y - min_y) as f32;
    for (i, trade) in trades.into_iter().enumerate() {
        let x = coords.start_pos.x + (i as f32 * CANDLE_W);

        // buy
        let position = Vector2::new(
            x + CANDLE_W,
            convert_coords_y(start_y, step_y, max_y as f32, trade.quantity_buy as f32),
        );
        let size = Vector2::new(CANDLE_W / 2.0, trade.quantity_buy as f32 * step_y);
        let color = Color::GREEN;
        d.draw_rectangle_v(position, size, color);

        // sell
        let position = Vector2::new(
            x + CANDLE_W + CANDLE_W / 2.0,
            convert_coords_y(start_y, step_y, max_y as f32, trade.quantity_sell as f32),
        );
        let size = Vector2::new(CANDLE_W / 2.0, trade.quantity_sell as f32 * step_y);
        let color = Color::RED;
        d.draw_rectangle_v(position, size, color);

        // print time labels on x-axis
        match frame {
            Frame::M1 => draw_frames_m1(d, trade.trade_period, &mut day, Vector2::new(x, y), font),
            Frame::M15 => draw_frames_m1(d, trade.trade_period, &mut day, Vector2::new(x, y), font),
            Frame::H1 => draw_frames_h1(d, trade.trade_period, &mut day, Vector2::new(x, y), font),
            Frame::D1 => {
                draw_frames_d1(d, trade.trade_period, &mut month, Vector2::new(x, y), font)
            }
        }
    }
}

fn draw_arrow(d: &mut RaylibDrawHandle, current_candle: &mut Candle) {
    if let Some(x) = current_candle.position_x {
        if let Some(y) = current_candle.position_y {
            let diff_x = 0.8;
            let diff_y = 15.0;
            let start_pos = Vector2::new(x + CANDLE_W / 2.0 - 1.5, 4.0);
            let end_pos = Vector2::new(x + CANDLE_W / 2.0 - diff_x, y - diff_y);
            let thick = 2.0;

            // main
            d.draw_line_ex(start_pos, end_pos, thick, Color::WHEAT);

            // left
            let start_pos = Vector2::new(x + CANDLE_W / 2.0 - 3.0, y - 22.0);
            let end_pos = Vector2::new(x + CANDLE_W / 2.0 - diff_x, y - diff_y);
            d.draw_line_ex(start_pos, end_pos, thick, Color::WHEAT);

            // right
            let start_pos = Vector2::new(x + CANDLE_W / 2.0 + 1.0, y - 22.0);
            let end_pos = Vector2::new(x + CANDLE_W / 2.0 - diff_x, y - diff_y);
            d.draw_line_ex(start_pos, end_pos, thick, Color::WHEAT);
        }
    }
}

fn draw_info(d: &mut RaylibDrawHandle, coords: &DrawCoords, font: &Font, info: &str) {
    d.draw_text_ex(
        font,
        info,
        Vector2::new(25.0, coords.start_pos.y + 220.0),
        15.0,
        0.0,
        Color::WHEAT,
    );
}

fn mouse_click(
    d: &mut RaylibDrawHandle,
    coords: &DrawCoords,
    candles: &Vec<Candle>,
    current_candle: &mut Candle,
    info: &mut String,
) -> bool {
    if d.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
        let mouse_position = d.get_mouse_position();
        if mouse_position.x >= coords.start_pos.x
            && mouse_position.y >= coords.start_pos.y
            && mouse_position.x <= coords.end_pos.x
            && mouse_position.y <= coords.end_pos.y
        {
            for candle in candles {
                if mouse_position.x >= candle.position_x.unwrap_or(0.0)
                    && mouse_position.x <= candle.position_x.unwrap_or(0.0) + CANDLE_W
                {
                    *current_candle = candle.clone();
                    *info = current_candle.to_info();
                    return true;
                }
            }
        }
    }
    false
}
