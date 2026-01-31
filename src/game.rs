use crate::audio::Audio;
use crate::bitmap::{Bitmap, Font};
use glam::*;

const GRAVITY: f32 = 400.0;
const FALLING_SPEED: f32 = 800.0;
const MOVEMENT_SPEED_X: f32 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    S,
    Space,
    LeftBracket,
    RightBracket,

    Count,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
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

#[derive(Debug, Clone)]
struct Aabb {
    min: Vec2,
    max: Vec2,
}

impl Aabb {
    fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
    fn overlaps(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x > other.min.x
            && self.min.y <= other.max.y
            && self.max.y > other.min.y
    }
}

fn draw_aabb(screen: &mut Bitmap, aabb: &Aabb, camera_pos: Vec2) {
    let min = world_space_to_screen_space(aabb.min, camera_pos);
    let max = world_space_to_screen_space(aabb.max, camera_pos);
    screen.draw_rectangle(
        min.x as i32,
        min.y as i32,
        max.x as i32,
        max.y as i32,
        false,
        0x00ff00,
    );
}

#[derive(Debug, Clone)]
struct MaskObject {
    position: Vec2,
    aabb: Aabb,
    color: crate::bitmap::ColorChannel,
    sprite: Bitmap,
    visible: bool,
}
impl MaskObject {
    fn aabb_world_space(&self) -> Aabb {
        Aabb {
            min: self.aabb.min + self.position,
            max: self.aabb.max + self.position,
        }
    }
}

impl TileMap {
    fn world_to_tile_index(&self, position: Vec2) -> IVec2 {
        (position / self.tile_size as f32).as_ivec2()
    }
    fn tile_index_to_world_coord(&self, tile_index: IVec2) -> Vec2 {
        (tile_index * self.tile_size as i32).as_vec2()
    }
    fn round_world_coord_to_tile(&self, position: Vec2) -> Vec2 {
        (position / self.tile_size as f32).floor()
    }

    fn sample_world_pos(&self, position: Vec2) -> u32 {
        let tile_pos = (position / self.tile_size as f32).as_ivec2();
        if tile_pos.x < 0
            || tile_pos.y < 0
            || tile_pos.x >= self.width as i32
            || tile_pos.y >= self.height as i32
        {
            0
        } else {
            self.tiles[(tile_pos.x + tile_pos.y * self.width as i32) as usize]
        }
    }

    fn draw(&self, tile_set: &TileSet, target: &mut Bitmap, camera: Vec2) {
        let screen_size = vec2(target.width as f32, target.height as f32);
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

    fn store_to_file(&self, path: &str) {
        let mut data = String::default();
        for y in 0..self.height {
            for x in 0..self.width {
                data += &format!("{},", self.tiles[(y * self.width + x) as usize]);
            }
            data.pop();
            data.push('\n');
        }
        std::fs::write(path, data).unwrap();
    }
}

struct EditorState {
    selected_tile: u32,
}
impl Default for EditorState {
    fn default() -> Self {
        Self { selected_tile: 0 }
    }
}

#[derive(Debug)]
struct Player {
    position: Vec2,
    velocity: Vec2,
    aabb: Aabb,
}

impl Player {
    fn aabb_world_space(&self) -> Aabb {
        Aabb {
            min: self.aabb.min + self.position,
            max: self.aabb.max + self.position,
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
    key_pressed: [bool; Key::Count as usize],
    key_released: [bool; Key::Count as usize],
    mouse_state: [bool; MouseButton::Count as usize], // is mouse currently pressed
    mouse_pressed: [bool; MouseButton::Count as usize], // was mouse just pressed
    mouse_released: [bool; MouseButton::Count as usize], // was mouse just release

    editor_state: EditorState,

    test_sprite: Bitmap,

    mouse_x: f32,
    mouse_y: f32,

    mask_game_objects: Vec<MaskObject>,

    player: Player,
    player_inventory: Vec<MaskObject>,
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

fn screen_to_world_space(pos_on_screen: Vec2, camera_pos: Vec2) -> Vec2 {
    pos_on_screen + camera_pos
}
fn world_space_to_screen_space(pos_in_world: Vec2, camera_pos: Vec2) -> Vec2 {
    pos_in_world + camera_pos
}

impl Game {
    pub fn new() -> Self {
        // Read level file
        let level_layout_file =
            std::fs::read_to_string("assets/level0.txt").expect("Could not load level file :(");
        let mut accumulator = String::new();
        let mut row_content: Vec<u32> = Vec::new();
        let mut layout: Vec<Vec<u32>> = Vec::new();
        let mut tile_count_x = 0;
        for char in level_layout_file.chars() {
            if char == ',' {
                let tile_index: u32 = accumulator
                    .parse::<u32>()
                    .expect(&format!("Could not parse! :({})", &accumulator));
                row_content.push(tile_index);
                accumulator = String::new();
            } else if char == '\r' {
                continue;
            } else if char == '\n' {
                layout.push(row_content.clone());
                tile_count_x = tile_count_x.max(row_content.len() as u32);
                row_content.clear();
            } else {
                accumulator.push(char);
            }
        }

        // Create flat tile vector
        let mut tile_indices: Vec<u32> = Vec::new();
        let tile_count_y = layout.len() as u32;
        for mut row in layout {
            let row_size = row.len();
            if row_size < tile_count_x as usize {
                for i in 0..(tile_count_x as usize - row_size) {
                    // pad for equal size
                    row.push(0);
                }
            }
            tile_indices.append(&mut row);
        }

        let colors = [
            0xffff0000, 0xff00ff00, 0xff0000ff, 0xffffff00, 0xffff00ff, 0xff00ffff,
        ];

        let tiles = colors
            .iter()
            .map(|&color| {
                let mut tile = Bitmap::new(8, 8);
                tile.clear(color);
                tile
            })
            .collect::<Vec<_>>();

        let tile_set = TileSet { tiles };

        let tile_map = TileMap {
            tile_size: 8,
            width: tile_count_x,
            height: tile_count_y,
            tiles: tile_indices,
        };

        let player_start_pos = vec2(200.0, 0.0);
        let player_sprite = Bitmap::load("assets/test_sprite.png");
        let sprite_width = player_sprite.width as f32;
        let sprite_height = player_sprite.height as f32;

        // Game objects
        let white_mask_pos = vec2(100.0, 0.0);
        let white_mask_sprite = Bitmap::load("assets/test_mask.png");
        let white_mask_sprite_width = white_mask_sprite.width as f32;
        let white_mask_sprite_height = white_mask_sprite.height as f32;

        let white_mask = MaskObject {
            position: white_mask_pos,
            aabb: Aabb {
                min: Vec2::ZERO,
                max: vec2(white_mask_sprite_width, white_mask_sprite_height),
            },
            color: crate::bitmap::WHITE,
            sprite: white_mask_sprite,
            visible: true,
        };

        Self {
            // audio: Some(Audio::new()),
            audio: None,
            font: Font::new_default(),

            test_sprite: player_sprite,

            camera: vec2(0.0, 0.0),
            key_state: [false; Key::Count as usize],
            key_pressed: [false; Key::Count as usize],
            key_released: [false; Key::Count as usize],

            editor_state: EditorState::default(),

            mouse_state: [false; MouseButton::Count as usize],
            mouse_pressed: [false; MouseButton::Count as usize],
            mouse_released: [false; MouseButton::Count as usize],

            tile_set,
            tile_map,

            mouse_x: 0.0,
            mouse_y: 0.0,

            // Add game objects
            mask_game_objects: vec![white_mask],

            player: Player {
                position: player_start_pos,
                velocity: Vec2::ZERO,
                aabb: Aabb {
                    min: vec2(1.0, 2.0),
                    max: vec2(15.0, 16.0),
                },
            },
            player_inventory: Vec::new(),

            time: 0.0,

            color_mask: crate::bitmap::RED,
            editor_mode: false,
        }
    }

    pub(crate) fn on_mouse_moved(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }
    pub(crate) fn on_mouse_button_down(&mut self, button: MouseButton, _x: f32, _y: f32) {
        self.mouse_state[button as usize] = true;
        self.mouse_pressed[button as usize] = true;
    }
    pub(crate) fn on_mouse_button_up(&mut self, button: MouseButton, _x: f32, _y: f32) {
        self.mouse_state[button as usize] = false;
        self.mouse_released[button as usize] = true;
    }
    pub(crate) fn on_key_down(&mut self, key: Key) {
        self.key_state[key as usize] = true;
        self.key_pressed[key as usize] = true;

        match key {
            Key::Space => self.editor_mode = !self.editor_mode,
            _ => {}
        }
    }
    pub(crate) fn on_key_up(&mut self, key: Key) {
        self.key_state[key as usize] = false;
        self.key_released[key as usize] = true;
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

        self.tile_map.draw(&self.tile_set, screen, self.camera);

        if self.editor_mode {
            if self.key_pressed[Key::S as usize] {
                self.tile_map.store_to_file("assets/level0.txt");
            }

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

            if self.key_pressed[Key::LeftBracket as usize] {
                if self.editor_state.selected_tile > 0 {
                    self.editor_state.selected_tile -= 1;
                }
            }
            if self.key_pressed[Key::RightBracket as usize] {
                if self.editor_state.selected_tile < (self.tile_set.tiles.len() - 1) as u32 {
                    self.editor_state.selected_tile += 1;
                }
            }

            screen.plot(self.mouse_x as i32, self.mouse_y as i32, 0xff00ff);

            if self.mouse_state[MouseButton::Left as usize] {
                let mouse_ws = screen_to_world_space(vec2(self.mouse_x, self.mouse_y), self.camera);
                let mouse_ws = mouse_ws.as_uvec2();
                let mouse_ts = mouse_ws / self.tile_map.tile_size;
                self.tile_map.tiles[(mouse_ts.x + mouse_ts.y * self.tile_map.width) as usize] =
                    self.editor_state.selected_tile + 1;
            }
            if self.mouse_state[MouseButton::Right as usize] {
                let mouse_ws = screen_to_world_space(vec2(self.mouse_x, self.mouse_y), self.camera);
                let mouse_ws = mouse_ws.as_uvec2();
                let mouse_ts = mouse_ws / self.tile_map.tile_size;
                self.tile_map.tiles[(mouse_ts.x + mouse_ts.y * self.tile_map.width) as usize] = 0;
            }

            // Place masks
        } else {
            // do game things here
            // let player_center = self.player.aabb_world_space().center();
            let aabb_ws = self.player.aabb_world_space();

            let samples_positions_below = [
                vec2(aabb_ws.min.x, aabb_ws.max.y + 1.0),
                vec2(aabb_ws.center().x, aabb_ws.max.y + 1.0),
                vec2(aabb_ws.max.x, aabb_ws.max.y + 1.0),
            ];
            let tiles_below = [
                self.tile_map.sample_world_pos(samples_positions_below[0]),
                self.tile_map.sample_world_pos(samples_positions_below[1]),
                self.tile_map.sample_world_pos(samples_positions_below[2]),
            ];

            let samples_positions_above = [
                vec2(aabb_ws.min.x, aabb_ws.min.y - 1.0),
                vec2(aabb_ws.center().x, aabb_ws.min.y - 1.0),
                vec2(aabb_ws.max.x, aabb_ws.min.y - 1.0),
            ];
            let tiles_above = [
                self.tile_map.sample_world_pos(samples_positions_above[0]),
                self.tile_map.sample_world_pos(samples_positions_above[1]),
                self.tile_map.sample_world_pos(samples_positions_above[2]),
            ];

            let samples_positions_left = [
                vec2(aabb_ws.min.x - 1.0, aabb_ws.min.y),
                vec2(aabb_ws.min.x - 1.0, aabb_ws.center().y),
                vec2(aabb_ws.min.x - 1.0, aabb_ws.max.y),
            ];
            let tiles_left = [
                self.tile_map.sample_world_pos(samples_positions_left[0]),
                self.tile_map.sample_world_pos(samples_positions_left[1]),
                self.tile_map.sample_world_pos(samples_positions_left[2]),
            ];

            let samples_positions_right = [
                vec2(aabb_ws.max.x + 1.0, aabb_ws.min.y),
                vec2(aabb_ws.max.x + 1.0, aabb_ws.center().y),
                vec2(aabb_ws.max.x + 1.0, aabb_ws.max.y),
            ];
            let tiles_right = [
                self.tile_map.sample_world_pos(samples_positions_right[0]),
                self.tile_map.sample_world_pos(samples_positions_right[1]),
                self.tile_map.sample_world_pos(samples_positions_right[2]),
            ];

            let tile_below = tiles_below.iter().any(|a| *a != 0);
            let tile_above = tiles_above.iter().any(|a| *a != 0);
            let tile_left = tiles_left.iter().any(|a| *a != 0);
            let tile_right = tiles_right.iter().any(|a| *a != 0);

            self.player.velocity.x = 0.0;
            if self.key_state[Key::Left as usize] {
                self.player.velocity.x -= MOVEMENT_SPEED_X;
            }
            if self.key_state[Key::Right as usize] {
                self.player.velocity.x += MOVEMENT_SPEED_X;
            }
            if self.key_pressed[Key::A as usize] {
                self.player.velocity.y = -100.0;
            }

            if tile_left {
                self.player.velocity.x = self.player.velocity.x.max(0.0);
            }
            if tile_right {
                self.player.velocity.x = self.player.velocity.x.min(0.0);
            }
            if tile_above {
                self.player.velocity.y = self.player.velocity.y.max(0.0);
            }
            if !tile_below {
                self.player.velocity.y += GRAVITY * delta_time;
            } else {
                self.player.velocity.y = self.player.velocity.y.min(0.0);
            }

            // self.player.velocity = self
            //     .player
            //     .velocity
            //     .clamp(vec2(-500.0, -500.0), vec2(500.0, 500.0));

            self.player.position += self.player.velocity * delta_time;
        }

        // let player_rel_pos = world_space_to_screen_space(self.player.position, self.camera);
        // self.test_sprite.draw_on(
        //     screen,
        //     player_rel_pos.x as i32,
        //     player_rel_pos.y as i32,
        //     self.color_mask,
        // );
        draw_aabb(screen, &self.player.aabb_world_space(), self.camera);

        // Loop over masks
        for mask in self.mask_game_objects.iter_mut() {
            if mask.visible {
                mask.sprite.draw_on(
                    screen,
                    mask.position.x as i32,
                    mask.position.y as i32,
                    crate::bitmap::WHITE,
                );

                // Add to collection
                if mask
                    .aabb_world_space()
                    .overlaps(&self.player.aabb_world_space())
                {
                    self.player_inventory.push(mask.clone());
                    mask.visible = false;
                }
            }
        }

        screen.draw_str(
            &self.font,
            &format!("time: {:.5} s", self.time),
            10,
            10,
            0xffff00,
        );

        screen.draw_str(
            &self.font,
            // &format!("player on ground: {}", self.player_on_ground),
            &format!("player speed: {}", self.player.velocity),
            10,
            30,
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
            &format!(
                "editor_mode: {}",
                if self.editor_mode { "true" } else { "false" }
            ),
            10,
            40,
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
        );

        // reset state
        self.mouse_pressed.fill(false);
        self.mouse_released.fill(false);
        self.key_pressed.fill(false);
        self.key_released.fill(false);
    }
}
