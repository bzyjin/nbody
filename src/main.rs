use macroquad::prelude::*;

mod physics;
mod simulation;
mod space;

const CANVAS_W: i32 = 1280;
const CANVAS_H: i32 = 960;

fn window_conf() -> Conf {
    Conf {
        window_title: "n-body simulation (test)".to_string(),
        window_width: CANVAS_W,
        window_height: CANVAS_H,
        window_resizable: true,
        sample_count: 0,
        icon: None,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut sim = simulation::Simulation::init();

    loop {
        // Start after 1 second
        if get_time() > 1. {
            sim.update();
        }

        sim.render();

        next_frame().await
    }
}
