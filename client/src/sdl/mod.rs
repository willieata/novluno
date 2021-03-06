extern crate geometry;

use std::cell::RefCell;
use std::borrow::BorrowMut;

use sdl2;
use sdl2::rect::Rect;
use sdl2::event::Event;
use sdl2::event::WindowEvent;
use sdl2::keyboard::Keycode;

use rusttype;
use rusttype::PositionedGlyph;

use geometry::rectangle::Rectangle;
use geometry::point::Point;

use core_compat::entity::sprite_type::SpriteType;
use core_compat::entity::rmd_type::RmdType;

use error::Error;
use resource_manager::list_manager::ListType;
use game::Game;
use game::input::Controller;
use game::input::MAX_CONTROLLERS as MAX_CTL;
use ::{WINDOW_HEIGHT, WINDOW_WIDTH};

// setup Rusttype
lazy_static! {
    static ref FONT: rusttype::Font<'static> = {
        let font_data = include_bytes!("../../static/noto_font/NotoMono-Regular.ttf");
        let font_collection = rusttype::FontCollection::from_bytes(font_data as &[u8]);
        font_collection.into_font().unwrap()
    };
}

pub struct Sdl {
    pub context: sdl2::Sdl,
    // video/rendering
    pub video: sdl2::VideoSubsystem,
    pub canvas: sdl2::render::Canvas<sdl2::video::Window>,
    pub texture_creator: sdl2::render::TextureCreator<sdl2::video::WindowContext>,
    // audio
    pub audio: sdl2::AudioSubsystem,
    pub audio_spec: sdl2::audio::AudioSpecDesired,
    // event handlers
    pub event_pump: RefCell<sdl2::EventPump>,
    // controllers
    pub controller: sdl2::GameControllerSubsystem,
    pub controllers: RefCell<[Option<sdl2::controller::GameController>; MAX_CTL]>,
    pub controller_count: u32,
    // debug
    pub do_debug_output: bool,
}

impl Sdl {
    pub fn new(width: u32, height: u32) -> Result<Sdl, Error> {
        // -- load SDL2 contexts
        let context = sdl2::init()?;
        let video = context.video()?;
        let window = video.window("Novluno", width, height)
            .position_centered()
            .opengl()
            .build()?;
        let canvas = window.into_canvas()
            .accelerated()
            .present_vsync()
            .build()?;
        let texture_creator = canvas.texture_creator();
        let controller = context.game_controller()?;
        let controllers = RefCell::new([None, None, None, None]);
        let event_pump = RefCell::new(context.event_pump()?);
        let audio = context.audio()?;
        let audio_spec = sdl2::audio::AudioSpecDesired {
            freq: Some(44100),
            channels: Some(2),
            samples: Some(4),
        };

        // -- Create SDL state object
        let sdl = Sdl {
            context,
            video,
            canvas,
            texture_creator,
            event_pump,
            audio,
            audio_spec,
            controller,
            controllers,
            controller_count: 0,
            do_debug_output: true,
        };
        Ok(sdl)
    }

    pub fn init_game_controllers(&mut self) -> Result<(), Error> {
        let num_joy = self.controller.num_joysticks()?;
        if self.controller_count != num_joy {
            let max = MAX_CTL as u32;
            let max = if num_joy < max { num_joy } else { max };
            for index in 1..max {
                println!("Found Controller index: {:?}", index);
            }
            self.controller_count = num_joy;
        }
        Ok(())
    }

    pub fn add_game_controller(&self, index: u32) -> Result<(), Error> {
        let mut controllers = self.controllers.borrow_mut();
        if index < MAX_CTL as u32 && index > 0 {
            let ctrl_list = controllers.borrow_mut();
            let new_ctrl = self.controller.open(index as u32)?;
            if let Some(spot) = ctrl_list.get_mut(index as usize) {
                *spot = Some(new_ctrl);
            }
        }
        println!("added controller: {}", index);
        Ok(())
    }

    pub fn remove_game_controller(&self, index: i32) -> Result<(), Error> {
        let mut controllers = self.controllers.borrow_mut();
        if index < MAX_CTL as i32 && index > 0 {
            let ctrl_list = controllers.borrow_mut();
            if let Some(spot) = ctrl_list.get_mut(index as usize) {
                *spot = None;
            }
        }
        println!("removed controller: {}", index);
        Ok(())
    }

    pub fn handle_events(
        &mut self,
        game: &mut Game
    ) {
        let mut event_pump = self.event_pump.borrow_mut();
        let mut last_event: Option<Event> = None;
        while let Some(new_event) = event_pump.poll_event() {
            if last_event.is_none() || new_event != last_event.unwrap() {
                match new_event {
                    Event::Quit { .. }
                    | Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                    => {
                        game.input.should_quit = true;
                    }
                    Event::KeyDown { keycode: Some(key), repeat, .. }
                    => {
                        let is_down = true;
                        if !repeat {
                            process_keycode(key, is_down, game.get_mut_keyboard());
                        }
                    }
                    Event::KeyUp { keycode: Some(key), repeat, .. }
                    => {
                        let is_down = false;
                        if !repeat {
                            process_keycode(key, is_down, game.get_mut_keyboard());
                        }
                    }
                    Event::Window { win_event: w_event, .. } => {
                        match w_event {
                            WindowEvent::Enter => (),
                            WindowEvent::Leave => (),
                            WindowEvent::SizeChanged(x, y) => {
                                game.input.should_resize = Some((x, y));
                                println!("Window size change: ({},{})", x, y);
                            }
                            _ => (),
                        }
                    }
                    Event::MouseMotion { x, y, .. } => {
                        game.input.mouse_x = x;
                        game.input.mouse_y = y;
                    },
                    Event::MouseButtonDown { mouse_btn: btn, .. } => {
                        let is_down = true;
                        match btn {
                            sdl2::mouse::MouseButton::Left => {
                                game.input.mouse_left.key_press(is_down);
                            },
                            sdl2::mouse::MouseButton::Middle => {
                                game.input.mouse_middle.key_press(is_down);
                            },
                            sdl2::mouse::MouseButton::Right => {
                                game.input.mouse_right.key_press(is_down);
                            },
                            _ => {}
                        }
                    },
                    Event::MouseButtonUp { mouse_btn: btn, .. } => {
                        let is_down = false;
                        match btn {
                            sdl2::mouse::MouseButton::Left => {
                                game.input.mouse_left.key_press(is_down);
                            },
                            sdl2::mouse::MouseButton::Middle => {
                                game.input.mouse_middle.key_press(is_down);
                            },
                            sdl2::mouse::MouseButton::Right => {
                                game.input.mouse_right.key_press(is_down);
                            },
                            _ => {}
                        }
                    },
                    Event::ControllerDeviceAdded { which: index, .. } => {
                        println!("{:?}: {:?}", new_event, index);
                        self.add_game_controller(index).unwrap();
                    }
                    Event::ControllerDeviceRemoved { which: index, .. } => {
                        println!("{:?}: {:?}", new_event, index);
                        self.remove_game_controller(index).unwrap();
                    }
                    Event::JoyDeviceAdded { .. } => (),
                    _ => {
                        println!("{:?}", new_event);
                    }
                }
            }
            last_event = Some(new_event);
        } // end while new SDL event
    }


    pub fn render(&mut self, game: &mut Game, _dt: f32) {
        // start frame
        self.canvas.set_draw_color(sdl2::pixels::Color::RGB(75, 100, 255));

        // draw background color
        self.canvas.clear();

        // render game map
        self.render_map_tiles(game);
        self.render_map_objects(game);

        // draw text
        // let text = format!("{}, {}", game.input.mouse_x, game.input.mouse_y);
        // self.render_text_line(&text, game.input.mouse_x, game.input.mouse_y);

        // finish frame
        self.canvas.present();
    }


    fn render_text_line(&mut self, text: &str, x: i32, y: i32) {
        let bpp = 4; // bytes per pixel
        let height: f32 = 24.0;
        let scale = rusttype::Scale{ x: height, y: height };
        let start = rusttype::point(0.0, FONT.v_metrics(scale).ascent);
        let glyphs: Vec<PositionedGlyph> = FONT.layout(&text, scale, start).collect();
        let width = glyphs.iter()
                                .rev()
                                .filter_map( |glyph| {
                                    glyph.pixel_bounding_box()
                                        .map( |b_box| {
                                            b_box.min.x as f32
                                            + glyph.unpositioned()
                                                   .h_metrics()
                                                   .advance_width
                                        })
                                }).next()
                                .unwrap_or(height * 2.0).ceil() as usize;

        // NOTE: this is a little weird to have to cap the integer height of
        //       the texture to fit the (possibly) non-integer glyph height...
        //       but ohh well... *shrug*
        let height = height.ceil() as usize;

        let mut texture = self.texture_creator.create_texture(
            Some(sdl2::pixels::PixelFormatEnum::RGBA8888),
            sdl2::render::TextureAccess::Streaming,
            width as u32,
            height as u32).unwrap();
        texture.set_blend_mode(sdl2::render::BlendMode::Blend);
        texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            for glyph in glyphs {
                if let Some(b_box) = glyph.pixel_bounding_box() {
                    glyph.draw( |x, y, v| {
                        // `v` is the pixel coverage of the glyph (aka alpha)
                        let alpha = (v * 255.0) as u8;
                        let x = x as i32 + b_box.min.x;
                        let y = y as i32 + b_box.min.y;

                        // the glyph coord could still be out of the texture
                        // bounds so we need to check it
                        if x >= 0 && x < width  as i32
                        && y >= 0 && y < height as i32 {
                            let y_off: usize = y as usize * pitch;
                            let x_off: usize = x as usize * bpp;
                            let offset = y_off + x_off;
                            buffer[offset+0] = alpha;
                            buffer[offset+1] = 255 as u8;
                            buffer[offset+2] = 255 as u8;
                            buffer[offset+3] = 255 as u8;
                        }
                    })
                }
            }
        }).unwrap();

        let src_rect = Rect::new( 0, 0, width as u32, height as u32);
        let dst_rect = Rect::new( x, y, width as u32, height as u32);

        let _ = self.canvas.copy(&texture, src_rect, dst_rect);
    }

    pub fn render_map_tiles(&mut self, game: &mut Game) {
        let tle_list = game.list_manager.get_list(ListType::Tile).unwrap();
        let map = game.map_manager.get_map(game.state.map).unwrap();
        let tile_stride = map.size_x() as i32;
        let tile_height = 24i32;
        let tile_width = 48i32;
        let mut tile_x = 0i32;
        let mut tile_y = 0i32;

        let view_bounds = Rectangle::new_from_points(
            (-100 - game.state.map_off_x, -100 - game.state.map_off_y),
            ( 100 + WINDOW_WIDTH as i32, 100 + WINDOW_HEIGHT as i32)
        );

        for map_tile in map.tiles().iter() {
            let tile_offset = Point::new ( tile_x * tile_width, tile_y * tile_height );
            let mouse_offset = Point::new(game.input.mouse_x, game.input.mouse_y);

            // skip tiles which out out of view
            if view_bounds.contains_point(&tile_offset) == false {
                next_tile(&mut tile_x, &mut tile_y, tile_stride);
                continue;
            }

            // draw map tile
            let tle_entry = map_tile.tle_rmd_entry;
            if tle_entry.file() != 0 {
                let file = tle_entry.file() as usize;
                let index = tle_entry.index() as usize;
                if let Ok(rmd) = game.data_manager.get_data(RmdType::Tile, file) {
                    if let Some(entry) = rmd.get_entry(index) {
                        for img in entry.images() {
                            for id in img.image_id.iter() {
                                let item = tle_list.get_item(*id as usize).unwrap();
                                let sprite = game.sprite_manager.get_sprite_entry(&item.entry, SpriteType::Tile, self).unwrap();
                                let _w = (img.source_x2 - img.source_x1) as u32;
                                let _h = (img.source_y2 - img.source_y1) as u32;
                                let src_rect = Rect::new( img.source_x1, img.source_y1, _w, _h);
                                let mut dst_rect = Rect::new( 0, 0, tile_width as u32, tile_height as u32);
                                dst_rect.offset(tile_offset.x, tile_offset.y);
                                dst_rect.offset(game.state.map_off_x, game.state.map_off_y);

                                // render
                                let _ = self.canvas.copy(&sprite.texture, src_rect, dst_rect);

                                // debug render
                                {
                                    let _rect = Rectangle::new_from_points((dst_rect.x(), dst_rect.y()), (dst_rect.width() as i32, dst_rect.height() as i32));
                                    if _rect.contains_point(&mouse_offset) {
                                        match map_tile.collision {
                                            0 => self.canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 10, 10)),
                                            _ => self.canvas.set_draw_color(sdl2::pixels::Color::RGB(10, 255, 10)),
                                        }
                                        let _ = self.canvas.draw_rect(dst_rect);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // update tile positions
            next_tile(&mut tile_x, &mut tile_y, tile_stride);
        }
    }

    pub fn render_map_objects(&mut self, game: &mut Game) {

        let obj_list = game.list_manager.get_list(ListType::Object).unwrap();
        let map = game.map_manager.get_map(game.state.map).unwrap();
        let tile_stride = map.size_x() as i32;
        let tile_height = 24i32;
        let tile_width = 48i32;
        let mut tile_x = 0i32;
        let mut tile_y = 0i32;

        let view_bounds = Rectangle::new_from_points(
            (-100 - game.state.map_off_x, -100 - game.state.map_off_y),
            ( 100 + WINDOW_WIDTH as i32, 100 + WINDOW_HEIGHT as i32)
        );

        for map_tile in map.tiles().iter() {
            let tile_offset = Point::new ( tile_x * tile_width, tile_y * tile_height );
            let mouse_offset = Point::new(game.input.mouse_x, game.input.mouse_y);

            // skip tiles which out out of view
            let tile_rect = Rectangle::new_from_points((tile_offset.x, tile_offset.y), (tile_width, tile_height));

            if view_bounds.contains_point(&tile_offset) == false
            // || tile_rect.contains_point(&mouse_offset) == false
            {
                next_tile(&mut tile_x, &mut tile_y, tile_stride);
                continue;
            }

            // draw tile objects
            let obj_entry = map_tile.obj_rmd_entry;
            if obj_entry.file() != 0 {
                let file = obj_entry.file() as usize;
                let index = obj_entry.index() as usize;
                if let Ok(rmd) = game.data_manager.get_data(RmdType::Object, file) {
                    if let Some(entry) = rmd.get_entry(index) {
                        for img in entry.images() {
                            for id in img.image_id.iter() {
                                // get the sprite
                                let _id : usize = *id as usize;
                                let item = obj_list.get_item(_id).unwrap();
                                let sprite = game.sprite_manager.get_sprite_entry(&item.entry, SpriteType::Object, self).unwrap();

                                // calculate the sprite's image offsets
                                let img_rect = Rect::new(0, 0, sprite.sprite.x_dim as u32, sprite.sprite.y_dim as u32);
                                let img_x_1_off = img.source_x1 - sprite.sprite.x_off;
                                let img_y_1_off = img.source_y1 - sprite.sprite.y_off;
                                let _src_pts = [ (img_x_1_off, img_y_1_off).into(), (img.source_x2 - sprite.sprite.x_off, img.source_y2 - sprite.sprite.y_off).into() ];
                                let mut _x_diff = 0;
                                let mut _y_diff = 0;
                                let mut src_rect = Rect::from_enclose_points(&_src_pts, None).unwrap();
                                if let Some(rect) = src_rect.intersection(img_rect) {
                                    if img_x_1_off < 0 { _x_diff = -img_x_1_off; }
                                    if img_y_1_off < 0 { _y_diff = -img_y_1_off; }
                                    src_rect = rect;
                                }

                                // actually move the destination rectangle into position
                                let mut dst_rect = Rect::new(_x_diff, _y_diff, src_rect.width(), src_rect.height());
                                dst_rect.offset(game.state.map_off_x, game.state.map_off_y);
                                dst_rect.offset(tile_offset.x, tile_offset.y);
                                dst_rect.offset(img.dest_x, img.dest_y);

                                // render
                                let _ = self.canvas.copy(&sprite.texture, src_rect, dst_rect);

                                // debug renders
                                {
                                    // TODO: make an into or something...
                                    let d_rect = Rectangle::new_from_points((dst_rect.x(), dst_rect.y()), (dst_rect.width() as i32, dst_rect.height() as i32));
                                    if d_rect.contains_point(&mouse_offset) {
                                        self.canvas.set_draw_color(sdl2::pixels::Color::RGB(10, 10, 255));
                                        let _ = self.canvas.draw_rect(dst_rect);
                                    }
                                    let tile_rect = Rectangle::new_from_points((tile_offset.x, tile_offset.y), (tile_width, tile_height));
                                    if tile_rect.contains_point(&mouse_offset) {
                                        let mut t_rect = Rect::new( tile_offset.x, tile_offset.y, tile_width as u32, tile_height as u32);
                                        self.canvas.set_draw_color(sdl2::pixels::Color::RGB(100, 10, 10));
                                        let _ = self.canvas.draw_rect(t_rect);
                                    }
                                }
                            }
                        }
                    }
                }
            } // end if obj_entry != 0

            // update tile positions
            next_tile(&mut tile_x, &mut tile_y, tile_stride);
        }
    }
}

/// Helper function to move to the next map tile in order.
/// TODO: Probably fold into `map` entity iterator?
fn next_tile(x: &mut i32, y: &mut i32, stride: i32) {
    *x += 1;
    if *x >= stride {
        *x  = 0;
        *y += 1;
    }
}

fn process_keycode(
    key: sdl2::keyboard::Keycode,
    is_down: bool,
    input: &mut Controller
) {
    match key {
        Keycode::W => input.move_up.key_press(is_down),
        Keycode::A => input.move_left.key_press(is_down),
        Keycode::S => input.move_down.key_press(is_down),
        Keycode::D => input.move_right.key_press(is_down),
        Keycode::Q => input.left_shoulder.key_press(is_down),
        Keycode::E => input.right_shoulder.key_press(is_down),
        Keycode::Up => input.action_up.key_press(is_down),
        Keycode::Down => input.action_down.key_press(is_down),
        Keycode::Right => input.action_right.key_press(is_down),
        Keycode::Left => input.action_left.key_press(is_down),
        Keycode::F => (),
        Keycode::Space => (),
        _ => (),
    }
}
