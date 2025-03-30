use raylib::prelude::*;

const W_HEIGHT: i32 = 480;
const W_WIDTH: i32 = 640;
const PADDING: i32 = 20;

pub fn run_terminal() {
    let (mut rl, thread) = raylib::init()
        .size(W_WIDTH, W_HEIGHT)
        .title("Trading terminal")
        .build();

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::WHITE);

        draw_axis(&mut d);
    }
}

fn draw_axis(d: &mut RaylibDrawHandle) {
    d.draw_line(PADDING, PADDING, PADDING, W_HEIGHT - PADDING, Color::BLACK);
    d.draw_line(
        PADDING,
        W_HEIGHT - PADDING,
        W_WIDTH - PADDING,
        W_HEIGHT - PADDING,
        Color::BLACK,
    );
    let step = 10;
    for i in (PADDING..W_HEIGHT - PADDING).step_by(step) {
        d.draw_line(PADDING - 5, i, PADDING + 5, i, Color::BLACK);
    }
    for i in (PADDING + step as i32..W_WIDTH - PADDING + step as i32).step_by(step) {
        d.draw_line(
            i,
            W_HEIGHT - PADDING - 5,
            i,
            W_HEIGHT - PADDING + 5,
            Color::BLACK,
        );
    }
}
