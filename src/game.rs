use crate::audio::Audio;
use crate::bitmap::{Bitmap, Font};
use glam::*;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    Space,

    Count,
}

struct TileSet {
    tiles: Vec<Bitmap>,
}

struct TileMap {
    tile_size: u32,

    width: u32,
    height: u32,
    tiles: Vec<u32>,
}

#[derive(Debug)]
struct Aabb {
    min: Vec2,
    max: Vec2,
}

impl TileMap {
    fn draw(&self, tile_set: &TileSet, target: &mut Bitmap, camera: Vec2) {
        let screen_size = vec2(target.width as f32, target.height as f32);
        let camera = camera - screen_size * 0.5;
        let bounds = Aabb {
            min: camera,
            max: camera + screen_size,
        };
        let tile_sizef = self.tile_size as f32;

        let bounds_in_tiles = Aabb {
            min: bounds.min / tile_sizef,
            max: bounds.max / tile_sizef,
        };
        let tile_min_x = bounds_in_tiles.min.x.max(0.0) as usize;
        let tile_min_y = bounds_in_tiles.min.y.max(0.0) as usize;

        let tile_max_x = bounds_in_tiles.max.x.ceil().clamp(0.0, self.width as f32) as usize;
        let tile_max_y = bounds_in_tiles.max.y.ceil().clamp(0.0, self.height as f32) as usize;

        let tile_count_x = tile_max_x - tile_min_x;
        let tile_count_y = tile_max_y - tile_min_y;

        for y in 0..tile_count_y {
            for x in 0..tile_count_x {
                let tx = (tile_min_x + x) as u32;
                let ty = (tile_min_y + y) as u32;

                let sx = x as i32 * self.tile_size as i32;
                let sy = y as i32 * self.tile_size as i32;

                let tile_index = self.tiles[(ty * self.width + tx) as usize];
                if tile_index != 0 {
                    let tile = &tile_set.tiles[(tile_index - 1) as usize];
                    tile.draw_on(
                        target,
                        sx - camera.x as i32 + tile_min_x as i32 * self.tile_size as i32,
                        sy - camera.y as i32 + tile_min_y as i32 * self.tile_size as i32,
                        crate::bitmap::WHITE,
                    );
                }
            }
        }
    }
}

pub struct Game {
    audio: Option<Audio>,
    font: Font,

    tile_set: TileSet,
    tile_map: TileMap,

    camera: Vec2,

    key_state: [bool; Key::Count as usize],

    test_sprite: Bitmap,

    mouse_x: f32,
    mouse_y: f32,

    player_x: i32,
    player_y: i32,

    time: f32,

    editor_mode: bool,

    color_mask: crate::bitmap::ColorChannel,
}

fn wang_hash(seed: u32) -> u32 {
    let seed = (seed ^ 61) ^ (seed >> 16);
    let seed = seed.wrapping_mul(9);
    let seed = seed ^ (seed >> 4);
    let seed = seed.wrapping_mul(0x27d4eb2d);
    seed ^ (seed >> 15)
}

impl Game {
    pub fn new() -> Self {

        // Read level file
        let level_layout_file= std::fs::read_to_string("assets/level1.txt").expect("Could not load level file :(");
        let mut accumulator = String::new();
        let mut row_content: Vec<u32> = Vec::new();
        let mut layout: Vec<Vec<u32>> = Vec::new();
        for char in level_layout_file.chars() {
            if char == ',' {
                dbg!(&accumulator);
                let tile_index: u32 = accumulator
                    .parse::<u32>()
                    .expect(&format!("Could not parse! :({})", &accumulator));
                row_content.push(tile_index);
                accumulator = String::new();
            } else if char == '\r' {
                continue;
            }
            else if char == '\n' {
                layout.push(row_content.clone());
                row_content.clear();
            } else {
                accumulator.push(char);
            }
        }

        let mut tile = Bitmap::new(16, 16);
        tile.clear(0xffff7fff);

        let tile_set = TileSet { tiles: vec![tile] };

        let tile_count_x = 512;
        let tile_count_y = 512;

        let tiles = (0..tile_count_x * tile_count_y)
            .map(|i| wang_hash(i) & 1)
            .collect::<Vec<_>>();

        let tile_map = TileMap {
            tile_size: 16,
            width: tile_count_x,
            height: tile_count_y,
            tiles,
        };

        Self {
            // audio: Some(Audio::new()),
            audio: None,
            font: Font::new_default(),

            test_sprite: Bitmap::load("assets/test_sprite.png"),

            camera: vec2(0.0, 0.0),
            key_state: [false; Key::Count as usize],

            tile_set,
            tile_map,

            mouse_x: 0.0,
            mouse_y: 0.0,

            player_x: 200,
            player_y: 200,

            time: 0.0,

            color_mask: crate::bitmap::RED,
            editor_mode: false,
        }
    }

    pub(crate) fn on_mouse_moved(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }
    pub(crate) fn on_mouse_button_down(&mut self, _button: super::MouseButton, _x: f32, _y: f32) {}
    pub(crate) fn on_mouse_button_up(&mut self, _button: super::MouseButton, _x: f32, _y: f32) {}
    pub(crate) fn on_key_down(&mut self, key: Key) {
        self.key_state[key as usize] = true;

        match key {
            Key::Up => self.player_y -= 10,
            Key::Down => self.player_y += 10,
            Key::Left => self.player_x -= 10,
            Key::Right => self.player_x += 10,
            Key::A => {}
            Key::B => {}
            Key::Space => self.editor_mode = !self.editor_mode,
            _ => {}
        }
    }
    pub(crate) fn on_key_up(&mut self, key: Key) {
        self.key_state[key as usize] = false;
    }

    pub fn set_color_mask(&mut self, color_channel: crate::bitmap::ColorChannel) {
        self.color_mask = color_channel;
    }

    pub fn add_color_mask(&mut self, color_channel: crate::bitmap::ColorChannel) {
        self.color_mask |= color_channel;
    }

    pub fn remove_color_mask(&mut self, color_channel: crate::bitmap::ColorChannel) {
        self.color_mask ^= color_channel;
    }

    pub fn tick(&mut self, delta_time: f32, screen: &mut Bitmap) {
        self.time += delta_time;

        screen.clear(0);

        if self.editor_mode {
            if self.key_state[Key::Left as usize] {
                self.camera.x -= delta_time * 50.0;
            }
            if self.key_state[Key::Right as usize] {
                self.camera.x += delta_time * 50.0;
            }
            if self.key_state[Key::Up as usize] {
                self.camera.y -= delta_time * 50.0;
            }
            if self.key_state[Key::Down as usize] {
                self.camera.y += delta_time * 50.0;
            }

            screen.plot(self.mouse_x as i32, self.mouse_y as i32, 0xff00ff);
        } else {
            // do game things here
        }

        self.tile_map.draw(&self.tile_set, screen, self.camera);
        self.test_sprite
            .draw_on(screen, self.player_x, self.player_y, self.color_mask);

        screen.draw_str(
            &self.font,
            &format!("time: {:.5} s", self.time),
            10,
            10,
            0xffff00,
        );
        screen.draw_str(
            &self.font,
            &format!("delta time: {:.5} s", delta_time),
            10,
            20,
            0xffff00,
        );

        screen.draw_str(
            &self.font,
            &format!("camera: {:?}", self.camera),
            10,
            30,
            0xffff00,
        );
        screen.draw_str(
            &self.font,
            &format!(
                "editor_mode: {}",
                if self.editor_mode { "true" } else { "false" }
            ),
            10,
            40,
            0xffff00,
        )
    }
}
