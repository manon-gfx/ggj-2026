pub mod sprite;

use crate::audio::Audio;
use crate::bitmap::{self, Bitmap, Font};
use crate::game::sprite::Sprite;
use bitflags::bitflags;
use glam::*;

const GRAVITY: f32 = 600.0;
const JUMP_IMPULSE: f32 = 150.0;
const JUMP_SUSTAIN: f32 = 350.0;
const MOVEMENT_ACCELERATION: f32 = 1500.0;
const MOVEMENT_SPEED_X: f32 = 100.0;
const FRICTION: f32 = 1500.0;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    A,
    S,
    Space,
    LeftBracket,
    RightBracket,

    // Mask activation/desactivation
    R,
    G,
    B,

    M,
    MusicC3,
    MusicCs3,
    MusicD3,
    MusicDs3,
    MusicE3,
    MusicF3,
    MusicFs3,
    MusicG3,
    MusicGs3,
    MusicA3,
    MusicAs3,
    MusicB3,
    MusicC4,
    MusicCs4,
    MusicD4,
    MusicDs4,
    MusicE4,

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
    tile_colors: Vec<bitmap::ColorChannel>,
    tile_types: Vec<TileFlags>,
}

struct TileMap {
    tile_size: u32,

    width: u32,
    height: u32,
    tiles: Vec<u32>,
}

struct PlayerInventory {
    tile_size: i32,
    width: i32,
    height: i32,
    position_on_screen: Vec2,
    background_color: u32,
    bag_sprite: Bitmap,
    masks: Vec<MaskObject>,
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
    fn point_intersects(&self, point: Vec2) -> bool {
        point.x >= self.min.x
            && point.x < self.max.x
            && point.y >= self.min.y
            && point.y < self.max.y
    }
}

fn draw_aabb(screen: &mut Bitmap, aabb: &Aabb, camera_pos: Vec2, color: u32) {
    let min = world_space_to_screen_space(aabb.min, camera_pos);
    let max = world_space_to_screen_space(aabb.max, camera_pos);
    screen.draw_rectangle(
        min.x as i32,
        min.y as i32,
        max.x as i32,
        max.y as i32,
        false,
        color,
    );
}

#[derive(Debug, Clone)]
struct MaskObject {
    position: Vec2,
    aabb: Aabb,
    color: crate::bitmap::ColorChannel,
    sprite_scene: Bitmap,
    sprite_inventory: Bitmap,
    visible: bool,
    activation_key: Key,
}
impl MaskObject {
    fn aabb_world_space(&self) -> Aabb {
        Aabb {
            min: self.aabb.min + self.position,
            max: self.aabb.max + self.position,
        }
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    struct TileFlags: u32 {
        const COLLISION = 0x1;
        const SPIKE = 0x2;
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
        (position / self.tile_size as f32).floor() * self.tile_size as f32
    }

    fn sample_world_pos(
        &self,
        position: Vec2,
        tile_colors: &Vec<bitmap::ColorChannel>,
        color_mask: &bitmap::ColorChannel,
    ) -> u32 {
        let color_mask = color_mask & 0xffffff;
        let tile_pos = self.world_to_tile_index(position);
        if tile_pos.x < 0
            || tile_pos.y < 0
            || tile_pos.x >= self.width as i32
            || tile_pos.y >= self.height as i32
        {
            0
        } else {
            let tile_index = self.tiles[(tile_pos.x + tile_pos.y * self.width as i32) as usize];
            //  If the tile is non-empty and non-white, and this color is masked out, treat as if there is no tile here
            if tile_index != 0 && tile_index != 1 && tile_index != 8 {
                if tile_colors[(tile_index - 1) as usize] & color_mask == 0 {
                    0
                } else {
                    tile_index
                }
            } else {
                tile_index
            }
        }
    }

    fn sample_tile_type_ws(
        &self,
        position: Vec2,
        tile_flags: &Vec<TileFlags>,
        tile_colors: &Vec<bitmap::ColorChannel>,
        color_mask: &bitmap::ColorChannel,
    ) -> TileFlags {
        let tile_index = self.sample_world_pos(position, tile_colors, &color_mask);
        if tile_index == 0 {
            TileFlags::empty()
        } else {
            tile_flags[(tile_index - 1) as usize]
        }
    }

    fn draw(
        &self,
        tile_set: &TileSet,
        target: &mut Bitmap,
        camera: Vec2,
        color_mask: &crate::bitmap::ColorChannel,
    ) {
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
                    // leave white tiles white
                    let tile = &tile_set.tiles[(tile_index - 1) as usize];
                    let color = &tile_set.tile_colors[(tile_index - 1) as usize];
                    let color_mask_rgb = color_mask & 0xffffff;
                    let color_rgb = color & 0xffffff;

                    let is_white_tile = (tile_index == 1) | (tile_index == 8);
                    let mut color_mask = color_mask;
                    if is_white_tile {
                        color_mask = &bitmap::WHITE;
                    }

                    tile.draw_tile(
                        target,
                        sx - camera.x as i32 + tile_min_x as i32 * self.tile_size as i32,
                        sy - camera.y as i32 + tile_min_y as i32 * self.tile_size as i32,
                        !is_white_tile && (color_rgb & color_mask_rgb == 0),
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
    idle_sprite: Sprite,
    walk_sprite: Sprite,
    jump_sprite: Sprite,
    death_sprite: Sprite,

    position: Vec2,
    velocity: Vec2,
    aabb: Aabb,
    on_ground: bool,
    is_jumping: bool,
    is_dead: bool,
}

impl Player {
    fn aabb_world_space(&self) -> Aabb {
        Aabb {
            min: self.aabb.min + self.position,
            max: self.aabb.max + self.position,
        }
    }

    fn tick(&mut self, delta_time: f32) {
        self.walk_sprite.tick(delta_time);

        if !self.on_ground {
            self.jump_sprite.tick(delta_time);
        } else {
            self.jump_sprite.t = 0.0;
        }

        if self.is_dead {
            self.death_sprite.tick(delta_time);
        }
    }

    fn draw(&self, screen: &mut Bitmap, camera: Vec2) {
        let scale = vec2(if self.velocity.x < 0.0 { -1.0 } else { 1.0 }, 1.0);
        let screen_pos = world_space_to_screen_space(self.position, camera);

        if self.is_dead {
            self.death_sprite.draw(screen, screen_pos, scale);
        } else if !self.on_ground {
            self.jump_sprite.draw(screen, screen_pos, scale);
        } else {
            if self.velocity.x.abs() < 0.001 {
                self.idle_sprite.draw(screen, screen_pos, scale);
            } else {
                self.walk_sprite.draw(screen, screen_pos, scale);
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct InputState {
    pub key_state: [bool; Key::Count as usize],
    pub key_pressed: [bool; Key::Count as usize],
    pub key_released: [bool; Key::Count as usize],

    pub mouse_state: [bool; MouseButton::Count as usize], // is mouse currently pressed
    pub mouse_pressed: [bool; MouseButton::Count as usize], // was mouse just pressed
    pub mouse_released: [bool; MouseButton::Count as usize], // was mouse just release
}
impl InputState {
    fn is_key_down(&self, key: Key) -> bool {
        self.key_state[key as usize]
    }
    fn is_key_pressed(&self, key: Key) -> bool {
        self.key_pressed[key as usize]
    }
    fn is_key_released(&self, key: Key) -> bool {
        self.key_released[key as usize]
    }

    fn is_mouse_down(&self, button: MouseButton) -> bool {
        self.mouse_state[button as usize]
    }
    fn is_mouse_pressed(&self, button: MouseButton) -> bool {
        self.mouse_pressed[button as usize]
    }
    fn is_mouse_released(&self, button: MouseButton) -> bool {
        self.mouse_released[button as usize]
    }

    // Call at the end of every frame
    fn reset(&mut self) {
        self.key_pressed.fill(false);
        self.key_released.fill(false);

        self.mouse_pressed.fill(false);
        self.mouse_released.fill(false);
    }
}

pub struct Game {
    audio: Option<Audio>,
    music_mode: bool,

    font: Font,

    tile_set: TileSet,
    tile_map: TileMap,

    camera: Vec2,

    input_state: InputState,

    editor_state: EditorState,

    test_sprite: Bitmap,

    mouse_x: f32,
    mouse_y: f32,

    mask_game_objects: Vec<MaskObject>,

    player: Player,
    player_inventory: PlayerInventory,
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
    pos_in_world - camera_pos
}

fn build_frame_list(
    sprite_sheet: &Bitmap,
    coords: &[(i32, i32)],
    size: (usize, usize),
) -> Vec<Bitmap> {
    coords
        .iter()
        .map(|(x, y)| {
            let mut bmp = Bitmap::new(size.0, size.1);
            sprite_sheet.draw_on(&mut bmp, -x, -y);
            bmp
        })
        .collect::<Vec<_>>()
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

        let tile_sheet = Bitmap::load("assets/level_tiles_8x8.png");
        let coords = [
            // Terrain Blocks
            (32, 0),
            (32, 32),
            (32, 64),
            (32, 96),
            (32, 128),
            (128, 0),
            (128, 32),
            (128, 64),
            (128, 96),
            (128, 128),
            // Spikes
            (32, 0 + 16),
            (32, 32 + 16),
            (32, 64 + 16),
            (32, 96 + 16),
            (32, 128 + 16),
            (128, 0 + 16),
            (128, 32 + 16),
            (128, 64 + 16),
            (128, 96 + 16),
            (128, 128 + 16),
        ];
        let tile_colors = vec![
            bitmap::BLACK,
            bitmap::RED,
            bitmap::BLUE,
            bitmap::GREEN,
            bitmap::YELLOW,
            bitmap::CYAN,
            bitmap::MAGENTA,
            bitmap::GREY,
            bitmap::ORANGE,
            bitmap::PURPLE,
            bitmap::BLACK,
            bitmap::RED,
            bitmap::BLUE,
            bitmap::GREEN,
            bitmap::YELLOW,
            bitmap::CYAN,
            bitmap::MAGENTA,
            bitmap::GREY,
            bitmap::ORANGE,
            bitmap::PURPLE,
        ];
        let tile_types = vec![
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
        ];

        let tiles = build_frame_list(&tile_sheet, &coords, (8, 8));

        let tile_set = TileSet {
            tiles,
            tile_types,
            tile_colors,
        };

        let tile_map = TileMap {
            tile_size: 8,
            width: tile_count_x,
            height: tile_count_y,
            tiles: tile_indices,
        };

        let player_start_pos = vec2(2200.0, 2110.0);
        let player_sprite = Bitmap::load("assets/test_sprite.png");
        let sprite_width = player_sprite.width as f32;
        let sprite_height = player_sprite.height as f32;

        // Inventory
        let bag_sprite = Bitmap::load("assets/sprites/bag.png");

        // Game objects for masks
        pub const MASK_SPRITE_SIZE: f32 = 16.0;
        let red_mask = MaskObject {
            position: vec2(2652.0, 2168.0),
            aabb: Aabb {
                min: Vec2::ZERO,
                max: vec2(MASK_SPRITE_SIZE, MASK_SPRITE_SIZE),
            },
            color: crate::bitmap::RED,
            sprite_scene: Bitmap::load("assets/sprites/red_mask_in_scene.png"),
            sprite_inventory: Bitmap::load("assets/sprites/red_mask_in_bag.png"),
            visible: true,
            activation_key: Key::R,
        };

        let green_mask = MaskObject {
            position: vec2(100.0, 0.0),
            aabb: Aabb {
                min: Vec2::ZERO,
                max: vec2(MASK_SPRITE_SIZE, MASK_SPRITE_SIZE),
            },
            color: crate::bitmap::GREEN,
            sprite_scene: Bitmap::load("assets/sprites/green_mask_in_scene.png"),
            sprite_inventory: Bitmap::load("assets/sprites/green_mask_in_bag.png"),
            visible: true,
            activation_key: Key::G,
        };

        let blue_mask = MaskObject {
            position: vec2(2219.0, 2144.0),
            aabb: Aabb {
                min: Vec2::ZERO,
                max: vec2(MASK_SPRITE_SIZE, MASK_SPRITE_SIZE),
            },
            color: crate::bitmap::BLUE,
            sprite_scene: Bitmap::load("assets/sprites/blue_mask_in_scene.png"),
            sprite_inventory: Bitmap::load("assets/sprites/blue_mask_in_bag.png"),
            visible: true,
            activation_key: Key::B,
        };

        let player_sprite_sheet = Bitmap::load("assets/sprite/spritesheet_animation.png");

        let walk_frames = [
            (0, 16),
            (16, 16),
            (32, 16),
            (48, 16),
            (64, 16),
            (80, 16),
            (96, 16),
            (112, 16),
        ];
        let jump_frames = [(0, 80), (16, 80)];

        let idle_sprite = Sprite {
            frames: build_frame_list(&player_sprite_sheet, &[(0, 0)], (16, 16)),
            frame_index: 0,
            t: 0.0,
            seconds_per_frame: 1.0 / 24.0,
        };
        let walk_sprite = Sprite {
            frames: build_frame_list(&player_sprite_sheet, &walk_frames, (16, 16)),
            frame_index: 0,
            t: 0.0,
            seconds_per_frame: 1.0 / 24.0,
        };
        let jump_sprite = Sprite {
            frames: build_frame_list(&player_sprite_sheet, &jump_frames, (16, 16)),
            frame_index: 0,
            t: 0.0,
            seconds_per_frame: 1.0 / 4.0,
        };
        let death_sprite = Sprite {
            frames: build_frame_list(&player_sprite_sheet, &[(0, 32)], (16, 16)),
            frame_index: 0,
            t: 0.0,
            seconds_per_frame: 1.0 / 4.0,
        };

        Self {
            // audio: Some(Audio::new()),
            audio: None,
            music_mode: false,
            font: Font::new_default(),

            test_sprite: player_sprite,

            camera: vec2(2000.0, 2000.0),

            input_state: InputState::default(),

            editor_state: EditorState::default(),

            tile_set,
            tile_map,

            mouse_x: 0.0,
            mouse_y: 0.0,

            // Add game objects
            mask_game_objects: vec![red_mask, green_mask, blue_mask],

            player: Player {
                idle_sprite,
                walk_sprite,
                jump_sprite,
                death_sprite,
                position: player_start_pos,
                velocity: Vec2::ZERO,
                aabb: Aabb {
                    min: vec2(3.0, 5.0),
                    max: vec2(12.0, 15.0),
                },
                on_ground: false,
                is_jumping: false,
                is_dead: false,
            },
            player_inventory: PlayerInventory {
                tile_size: 16,
                width: 256,
                height: 64,
                position_on_screen: vec2(0.0, 192.0),
                background_color: 0xffffefd5,
                bag_sprite: bag_sprite,
                masks: Vec::new(),
            },
            time: 0.0,

            color_mask: crate::bitmap::BLUE,
            editor_mode: false,
        }
    }

    pub(crate) fn on_mouse_moved(&mut self, x: f32, y: f32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }
    pub(crate) fn on_mouse_button_down(&mut self, button: MouseButton, _x: f32, _y: f32) {
        self.input_state.mouse_state[button as usize] = true;
        self.input_state.mouse_pressed[button as usize] = true;
    }
    pub(crate) fn on_mouse_button_up(&mut self, button: MouseButton, _x: f32, _y: f32) {
        self.input_state.mouse_state[button as usize] = false;
        self.input_state.mouse_released[button as usize] = true;
    }
    pub(crate) fn on_key_down(&mut self, key: Key) {
        self.input_state.key_state[key as usize] = true;
        self.input_state.key_pressed[key as usize] = true;

        if self.music_mode {
            if let Some(audio) = &self.audio {
                audio.key_sender.send((key, true)).unwrap();
            }
        }

        match key {
            Key::Space => self.editor_mode = !self.editor_mode,
            Key::M => self.music_mode = !self.music_mode,
            _ => {}
        }
    }
    pub(crate) fn on_key_up(&mut self, key: Key) {
        self.input_state.key_state[key as usize] = false;
        self.input_state.key_released[key as usize] = true;

        if self.music_mode {
            if let Some(audio) = &self.audio {
                audio.key_sender.send((key, false)).unwrap();
            }
        }
    }

    pub fn set_color_mask(&mut self, color_channel: crate::bitmap::ColorChannel) {
        // if its a new mask, set new mask
        if (self.color_mask != color_channel) {
            self.color_mask = color_channel;
        } else {
            // else wear no mask
            self.color_mask = bitmap::BLACK;
        }
    }

    pub fn toggle_color_mask(&mut self, color_channel: crate::bitmap::ColorChannel) {
        if self.color_mask & color_channel != 0 {
            self.remove_color_mask(color_channel);
        } else {
            self.add_color_mask(color_channel);
        }
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

        if !self.editor_mode {
            self.camera = self.player.position - vec2(132.0, 128.0);
        }

        self.tile_map.draw(
            &self.tile_set,
            screen,
            self.camera,
            if self.editor_mode {
                &0xffffffff
            } else {
                &self.color_mask
            },
        );

        if self.editor_mode {
            if self.input_state.is_key_pressed(Key::S) {
                self.tile_map.store_to_file("assets/level0.txt");
                println!("Level Saved!");
            }

            if self.input_state.is_key_down(Key::Left) {
                self.camera.x -= delta_time * 150.0;
            }
            if self.input_state.is_key_down(Key::Right) {
                self.camera.x += delta_time * 150.0;
            }
            if self.input_state.is_key_down(Key::Up) {
                self.camera.y -= delta_time * 150.0;
            }
            if self.input_state.is_key_down(Key::Down) {
                self.camera.y += delta_time * 150.0;
            }

            if self.input_state.is_key_pressed(Key::LeftBracket) {
                if self.editor_state.selected_tile > 0 {
                    self.editor_state.selected_tile -= 1;
                }
            }
            if self.input_state.is_key_pressed(Key::RightBracket) {
                if self.editor_state.selected_tile < (self.tile_set.tiles.len() - 1) as u32 {
                    self.editor_state.selected_tile += 1;
                }
            }

            screen.plot(self.mouse_x as i32, self.mouse_y as i32, 0xff00ff);

            if self.mouse_y < 192.0 {
                if self.input_state.is_mouse_down(MouseButton::Left) {
                    let mouse_ws =
                        screen_to_world_space(vec2(self.mouse_x, self.mouse_y), self.camera);
                    let mouse_ws = mouse_ws.as_uvec2();
                    let mouse_ts = mouse_ws / self.tile_map.tile_size;
                    self.tile_map.tiles[(mouse_ts.x + mouse_ts.y * self.tile_map.width) as usize] =
                        self.editor_state.selected_tile + 1;
                }
                if self.input_state.is_mouse_down(MouseButton::Right) {
                    let mouse_ws =
                        screen_to_world_space(vec2(self.mouse_x, self.mouse_y), self.camera);
                    let mouse_ws = mouse_ws.as_uvec2();
                    let mouse_ts = mouse_ws / self.tile_map.tile_size;
                    self.tile_map.tiles[(mouse_ts.x + mouse_ts.y * self.tile_map.width) as usize] =
                        0;
                }
            }
        } else {
            // do game things here
            if self.input_state.is_key_down(Key::Left) {
                self.player.velocity.x = self.player.velocity.x.min(0.0);
                self.player.velocity.x -= MOVEMENT_ACCELERATION * delta_time;
            }
            if self.input_state.is_key_down(Key::Right) {
                self.player.velocity.x = self.player.velocity.x.max(0.0);
                self.player.velocity.x += MOVEMENT_ACCELERATION * delta_time;
            }

            if !self.input_state.is_key_down(Key::Left) && !self.input_state.is_key_down(Key::Right)
            {
                if self.player.velocity.x > 0.0 {
                    self.player.velocity.x -= self.player.velocity.x.min(FRICTION * delta_time);
                }
                if self.player.velocity.x < 0.0 {
                    self.player.velocity.x -= (-self.player.velocity.x).min(-FRICTION * delta_time);
                }
            }
            self.player.velocity.x = self
                .player
                .velocity
                .x
                .clamp(-MOVEMENT_SPEED_X, MOVEMENT_SPEED_X);

            if self.input_state.is_key_pressed(Key::A) && self.player.on_ground {
                self.player.velocity.y = -JUMP_IMPULSE;
                self.player.is_jumping = true;
            }
            if self.input_state.is_key_down(Key::A) {
                if self.player.is_jumping {
                    self.player.velocity.y -= JUMP_SUSTAIN * delta_time;
                }
            } else {
                self.player.is_jumping = false;
            }

            // Current situ: activating a new mask disables old mask
            // If you toggle the same mask again, you take it off
            if self.input_state.is_key_released(Key::R) {
                if let Some(red_mask) = self
                    .player_inventory
                    .masks
                    .iter()
                    .find(|&x| x.activation_key == Key::R)
                {
                    self.set_color_mask(red_mask.color);
                };
            }
            if self.input_state.is_key_released(Key::G) {
                if let Some(green_mask) = self
                    .player_inventory
                    .masks
                    .iter()
                    .find(|&x| x.activation_key == Key::G)
                {
                    self.set_color_mask(green_mask.color);
                };
            }
            if self.input_state.is_key_released(Key::B) {
                if let Some(blue_mask) = self
                    .player_inventory
                    .masks
                    .iter()
                    .find(|&x| x.activation_key == Key::B)
                {
                    self.set_color_mask(blue_mask.color);
                };
            }

            self.player.velocity.y += GRAVITY * delta_time;

            // Speed limit!
            self.player.velocity = self
                .player
                .velocity
                .clamp(vec2(-242.0, -242.0), vec2(242.0, 242.0));

            // self.player.position += self.player.velocity * delta_time;

            self.player.position.x += self.player.velocity.x * delta_time;
            {
                let aabb_ws = self.player.aabb_world_space();
                let samples_positions_left = [
                    vec2(aabb_ws.min.x, aabb_ws.min.y),
                    vec2(aabb_ws.min.x, aabb_ws.center().y),
                    vec2(aabb_ws.min.x, aabb_ws.max.y - 1.0),
                ];
                let tiles_left = [
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_left[0],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_left[1],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_left[2],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                ];

                let samples_positions_right = [
                    vec2(aabb_ws.max.x, aabb_ws.min.y),
                    vec2(aabb_ws.max.x, aabb_ws.center().y),
                    vec2(aabb_ws.max.x, aabb_ws.max.y - 1.0),
                ];
                let tiles_right = [
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_right[0],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_right[1],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_right[2],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                ];
                let tile_collision_left =
                    tiles_left.iter().any(|a| a.contains(TileFlags::COLLISION));
                let tile_collision_right =
                    tiles_right.iter().any(|a| a.contains(TileFlags::COLLISION));

                if tiles_left
                    .iter()
                    .chain(tiles_right.iter())
                    .any(|a| a.contains(TileFlags::SPIKE))
                {
                    self.player.is_dead = true;
                }

                if tile_collision_left {
                    self.player.velocity.x = self.player.velocity.x.max(0.0);
                    let tile_size = self.tile_map.tile_size as f32;
                    let limit =
                        (self.player.aabb_world_space().min.x / tile_size).ceil() * tile_size;
                    let offset = -self.player.aabb.min.x;
                    self.player.position.x = self.player.position.x.max(offset + limit);
                }
                if tile_collision_right {
                    self.player.velocity.x = self.player.velocity.x.min(0.0);

                    let tile_size = self.tile_map.tile_size as f32;
                    let limit =
                        (self.player.aabb_world_space().max.x / tile_size).floor() * tile_size;
                    let offset = -self.player.aabb.max.x - 1.0;
                    self.player.position.x = self.player.position.x.min(offset + limit);
                }
            }

            // Move player out of tile map
            self.player.position.y += self.player.velocity.y * delta_time;
            {
                let aabb_ws = self.player.aabb_world_space();

                let samples_positions_below = [
                    vec2(aabb_ws.min.x, aabb_ws.max.y + 0.99999),
                    vec2(aabb_ws.center().x, aabb_ws.max.y + 0.99999),
                    vec2(aabb_ws.max.x - 1.0, aabb_ws.max.y + 0.99999),
                ];

                let tiles_below = [
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_below[0],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_below[1],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_below[2],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                ];

                let samples_positions_above = [
                    vec2(aabb_ws.min.x, aabb_ws.min.y),
                    vec2(aabb_ws.center().x, aabb_ws.min.y),
                    vec2(aabb_ws.max.x - 1.0, aabb_ws.min.y),
                ];
                let tiles_above = [
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_above[0],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_above[1],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_above[2],
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        &self.color_mask,
                    ),
                ];

                let tile_collision_below =
                    tiles_below.iter().any(|a| a.contains(TileFlags::COLLISION));
                let tile_collision_above =
                    tiles_above.iter().any(|a| a.contains(TileFlags::COLLISION));

                if tiles_above
                    .iter()
                    .chain(tiles_below.iter())
                    .any(|a| a.contains(TileFlags::SPIKE))
                {
                    self.player.is_dead = true;
                }

                if tile_collision_above {
                    self.player.velocity.y = self.player.velocity.y.max(0.0);

                    let tile_size = self.tile_map.tile_size as f32;
                    let limit =
                        (self.player.aabb_world_space().min.y / tile_size).ceil() * tile_size;
                    let offset = -self.player.aabb.min.y;
                    self.player.position.y = self.player.position.y.max(offset + limit);

                    self.player.is_jumping = false;
                }
                if tile_collision_below {
                    self.player.velocity.y = self.player.velocity.y.min(0.0);

                    let tile_size = self.tile_map.tile_size as f32;
                    let limit = ((self.player.aabb_world_space().max.y + 0.99999) / tile_size)
                        .floor()
                        * tile_size;
                    let offset = -self.player.aabb.max.y - 1.0;

                    self.player.position.y = self.player.position.y.min(offset + limit);

                    self.player.is_jumping = false;
                }
                self.player.on_ground = tile_collision_below;
            }
            self.player.tick(delta_time);
        }

        self.player.draw(screen, self.camera);

        // Loop over masks
        for mask in self.mask_game_objects.iter_mut() {
            if mask.visible {
                let pos = world_space_to_screen_space(mask.position, self.camera);
                mask.sprite_scene
                    .draw_on(screen, pos.x as i32, pos.y as i32);

                // Add to collection
                if mask
                    .aabb_world_space()
                    .overlaps(&self.player.aabb_world_space())
                {
                    self.player_inventory.masks.push(mask.clone());
                    mask.visible = false;
                }
            }
        }

        screen.draw_str(
            &self.font,
            &format!("delta time: {:.5} s", delta_time),
            10,
            10,
            0xffff00,
        );

        screen.draw_str(
            &self.font,
            &format!("player position: {}", self.player.position),
            10,
            20,
            0xffff00,
        );

        // draw inventory on top
        // TODO: Could make inventory-overlay its own bitmap and draw items on that and then draw the inventory on the screen

        if self.editor_mode {
            screen.draw_str(&self.font, "editor_mode", 191, 10, 0xffff00);

            let aabb = Aabb {
                min: vec2(-1.0, -1.0),
                max: vec2(self.tile_map.width as f32, self.tile_map.height as f32)
                    * self.tile_map.tile_size as f32,
            };
            draw_aabb(screen, &aabb, self.camera, 0x00ff00);

            screen.draw_rectangle(0, 192, 255, 207, true, 0x0);
            screen.draw_rectangle(0, 192, 255, 207, false, 0xffffffff);

            for (i, tile) in self.tile_set.tiles.iter().take(24).enumerate() {
                let aabb = Aabb {
                    min: vec2(7.0 + i as f32 * 10.0, 192.0 + 3.0),
                    max: vec2(16.0 + i as f32 * 10.0, 192.0 + 12.0),
                };
                if i == self.editor_state.selected_tile as usize {
                    draw_aabb(screen, &aabb, Vec2::ZERO, 0xffffff);
                }

                if self.input_state.is_mouse_pressed(MouseButton::Left)
                    && aabb.point_intersects(vec2(self.mouse_x, self.mouse_y))
                {
                    self.editor_state.selected_tile = i as u32;
                }
                tile.draw_on(screen, 8 + i as i32 * 10, 192 + 4);
            }
        } else {
            screen.draw_rectangle(
                self.player_inventory.position_on_screen.x as i32,
                self.player_inventory.position_on_screen.y as i32,
                self.player_inventory.position_on_screen.x as i32 + self.player_inventory.width,
                self.player_inventory.position_on_screen.y as i32 + self.player_inventory.height,
                true,
                self.player_inventory.background_color,
            );
            self.player_inventory.bag_sprite.draw_on(
                screen,
                self.player_inventory.position_on_screen.x as i32,
                self.player_inventory.position_on_screen.y as i32,
            );
            for i in 0..self.player_inventory.masks.len() {
                self.player_inventory.masks[i].sprite_inventory.draw_on(
                    screen,
                    self.player_inventory.position_on_screen.x as i32
                        + (i as i32 + 2) * self.player_inventory.tile_size,
                    self.player_inventory.position_on_screen.y as i32,
                );
            }
        }

        // reset state
        self.input_state.reset();
    }
}
