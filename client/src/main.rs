#![allow(dead_code)]

extern crate core_compat;
extern crate geometry;

extern crate sdl2;
extern crate rusttype;
#[macro_use]
extern crate lazy_static;

mod error;
mod game;
mod sdl;
mod resource_manager;

use sdl::Sdl;

use std::time::Instant;


const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

fn main() {

    // Setup SDL2
    let mut sdl = Sdl::new(WINDOW_WIDTH, WINDOW_HEIGHT).unwrap();
    sdl.init_game_controllers().unwrap();

    // Setup initial game state
    let mut game = game::Game::new();

    let map_number = 3;
    game.state.map = map_number;
    game.load_map(map_number, &mut sdl).unwrap();

    'main: loop {

        let start_time = Instant::now();

        // Processes events
        sdl.handle_events(&mut game);

        // Update
        let old_map = game.state.map;
        if game.input.should_quit {
            break 'main;
        }
        game.update();

        // change maps?
        let new_map = game.state.map;
        if old_map != new_map {
            game.load_map(new_map, &mut sdl).unwrap();
        }

        // Render
        let ft = frame_time(&start_time);
        sdl.render(&mut game, ft);

        // worst frame limiter ever
        let dur = std::time::Duration::from_millis(100);
        std::thread::sleep(dur);
    }
}

fn frame_time(start: &Instant) -> f32 {
    let elapsed = start.elapsed();
    let sec_ms = (elapsed.as_secs() * 1000) as f32;
    let nano_ms = (elapsed.subsec_nanos() as f32) / 1_000_000.0;
    sec_ms + nano_ms
}
