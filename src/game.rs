pub mod background;
pub mod editor;
pub mod enemy;
pub mod sprite;
pub mod tilemap;

use crate::audio::Audio;
use crate::audio::sound::SoundTypes;
use crate::bitmap::{self, Bitmap, Font};
use crate::game::background::Background;
use crate::game::sprite::Sprite;
use editor::EditorState;
use enemy::{Enemy, spawn_enemies};
use glam::*;
use std::collections::BTreeMap;

use tilemap::{TileFlags, TileMap, TileSet, TileStruct};

const GRAVITY: f32 = 600.0;
const JUMP_IMPULSE: f32 = 150.0;
const JUMP_SUSTAIN: f32 = 350.0;
const MOVEMENT_ACCELERATION: f32 = 1500.0;
const MOVEMENT_SPEED_X: f32 = 100.0;
const FRICTION: f32 = 1500.0;

const DEBUG_MASKS: bool = false;
const DEBUG_MODE: bool = true;
const AUDIO_ON: bool = true;

#[derive(Debug)]
pub struct SaveState {
    pub player_position: Vec2,
    pub has_red_mask: bool,
    pub has_green_mask: bool,
    pub has_blue_mask: bool,
    pub color_mask: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum Axis {
    LeftStickX,
    LeftStickY,
    // LeftZ,
    // RightStickX,
    // RightStickY,
    // RightZ,
    // DPadX,
    // DPadY,
    Count,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum Key {
    MoveUp,    // Arrow up
    MoveDown,  // Arrow down
    MoveLeft,  // Arrow left
    MoveRight, // Arrow right
    A,
    SaveLevelEdit, // Save in level editor mode
    EditMode,      // Space
    SelectPrev,    // LeftBracket
    RightBracket,  // RightBracket

    // Mask activation/desactivation
    R,
    G,
    B,

    MusicMode, // M
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

    MaskRed,
    MaskGreen,
    MaskBlue,
    Jump, // Jump
    MuteAudio,

    Count, // I was wondering what this is counting, but found out (the hard way) that it is the number of items in this enum
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Count,
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
    sprite_inventory_activated: Bitmap,
    sprite_key_keyboard: Bitmap,
    keyboard_key_name: String,
    sprite_key_controller: Bitmap,
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

struct SaveGamePoint {
    position: Vec2,
    aabb: Aabb,
    color: crate::bitmap::ColorChannel,
    sprite_scene_off: Bitmap,
    sprite_scene_on: Bitmap,
    activated: bool,
    visible: bool,
}
impl SaveGamePoint {
    fn aabb_world_space(&self) -> Aabb {
        Aabb {
            min: self.aabb.min + self.position,
            max: self.aabb.max + self.position,
        }
    }
}

#[derive(Debug)]
struct Player {
    idle_sprite: Sprite,
    walk_sprite: Sprite,
    jump_sprite: Sprite,
    death_sprite: Sprite,
    win_sprite: Sprite,

    position: Vec2,
    velocity: Vec2,
    aabb: Aabb,
    on_ground: bool,
    is_jumping: bool,
    is_dead: bool,
    is_winner: bool,
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
        if self.is_winner {
            self.win_sprite.tick(delta_time);
        }
    }

    fn draw(&self, screen: &mut Bitmap, camera: Vec2, color_mask: u32) {
        let scale = vec2(if self.velocity.x < 0.0 { -1.0 } else { 1.0 }, 1.0);
        let screen_pos = world_space_to_screen_space(self.position, camera);

        if self.is_winner {
            self.win_sprite
                .draw_player(screen, screen_pos, scale, color_mask);
        } else if self.is_dead {
            self.death_sprite
                .draw_player(screen, screen_pos, scale, color_mask);
        } else if !self.on_ground {
            self.jump_sprite
                .draw_player(screen, screen_pos, scale, color_mask);
        } else {
            if self.velocity.x.abs() < 0.001 {
                self.idle_sprite
                    .draw_player(screen, screen_pos, scale, color_mask);
            } else {
                self.walk_sprite
                    .draw_player(screen, screen_pos, scale, color_mask);
            }
        }
    }
}

#[derive(Debug)]
pub struct InputState {
    pub mouse: Vec2,

    pub axis_state: [f32; Axis::Count as usize],

    pub key_state: [bool; Key::Count as usize],
    pub key_pressed: [bool; Key::Count as usize],
    pub key_released: [bool; Key::Count as usize],

    pub mouse_state: [bool; MouseButton::Count as usize], // is mouse currently pressed
    pub mouse_pressed: [bool; MouseButton::Count as usize], // was mouse just pressed
    pub mouse_released: [bool; MouseButton::Count as usize], // was mouse just release
}
impl Default for InputState {
    fn default() -> Self {
        Self {
            mouse: Vec2::default(),

            axis_state: [0.0; Axis::Count as usize],

            key_state: [false; Key::Count as usize],
            key_pressed: [false; Key::Count as usize],
            key_released: [false; Key::Count as usize],

            mouse_state: [false; MouseButton::Count as usize], // is mouse currently pressed
            mouse_pressed: [false; MouseButton::Count as usize], // was mouse just pressed
            mouse_released: [false; MouseButton::Count as usize], // was mouse just release
        }
    }
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

    fn axis_state(&self, axis: Axis) -> f32 {
        self.axis_state[axis as usize]
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

    actual_camera: Vec2,
    camera: Vec2,

    input_state: InputState,

    editor_state: EditorState,

    background: Background,

    save_state: Option<SaveState>,

    mouse_x: f32,
    mouse_y: f32,

    mask_game_objects: Vec<MaskObject>,
    savepoint_objects: Vec<SaveGamePoint>,
    enemies: Vec<Enemy>,

    enemy_sprite_red: Sprite,
    enemy_sprite_green: Sprite,
    enemy_sprite_blue: Sprite,
    enemy_sprite_white: Sprite,

    died_position: Vec2,
    player: Player,
    player_inventory: PlayerInventory,
    is_player_walking: bool,
    was_player_walking: bool,
    time: f32,

    player_uses_controller: bool,

    death_sequence_is_playing: bool,
    death_sequence_duration: f32,

    winning_sequence_is_playing: bool,
    winning_sequence_duration: f32,

    editor_mode: bool,

    color_mask: crate::bitmap::ColorChannel,
    lerp_color_mask: Vec3,
}

// fn wang_hash(seed: u32) -> u32 {
//     let seed = (seed ^ 61) ^ (seed >> 16);
//     let seed = seed.wrapping_mul(9);
//     let seed = seed ^ (seed >> 4);
//     let seed = seed.wrapping_mul(0x27d4eb2d);
//     seed ^ (seed >> 15)
// }

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

fn build_tileset(
    tileset_sheet: &Bitmap,
    color_lst: Vec<u32>,
    color_start_pos: &Vec<(i32, i32)>,
    rel_coords: &Vec<(i32, i32)>,
    rel_tile_flags: Vec<TileFlags>,
    size: (usize, usize),
) -> TileSet {
    let tiles_per_color = rel_coords.len();
    let mut coords = Vec::<(i32, i32)>::with_capacity(color_lst.len() * tiles_per_color);
    let mut tile_types: Vec<TileFlags> =
        Vec::<TileFlags>::with_capacity(color_lst.len() * tiles_per_color);
    let mut tile_colors = Vec::<u32>::with_capacity(color_lst.len() * tiles_per_color);
    for (j, &(rel_x, rel_y)) in rel_coords.iter().enumerate() {
        for (i, &c) in color_start_pos.iter().enumerate() {
            // println!("tile set coords? {:?}", c);
            coords.push((c.0 + rel_x, c.1 + rel_y));
            tile_colors.push(color_lst[i]);
            if color_lst[i] == bitmap::RED {
                tile_types.push(rel_tile_flags[j] | TileFlags::RED);
            } else if color_lst[i] == bitmap::BLUE {
                tile_types.push(rel_tile_flags[j] | TileFlags::BLUE);
            } else if color_lst[i] == bitmap::GREEN {
                tile_types.push(rel_tile_flags[j] | TileFlags::GREEN);
            } else {
                tile_types.push(rel_tile_flags[j]);
            };
        }
    }
    let tiles = build_frame_list(&tileset_sheet, &coords, (8, 8));

    // let mut tile_objs = Vec::<TileStruct>::with_capacity(color_lst.len() * tiles_per_color);
    let mut tile_objs = BTreeMap::<usize, TileStruct>::new();
    let tilesheet_stride_in_tiles = tileset_sheet.width / size.0 as usize;

    let mut color_start = Vec::<usize>::with_capacity(color_lst.len());
    for (i, &c) in color_start_pos.iter().enumerate() {
        color_start.push(
            (((c.0) as usize) / size.0) as usize
                + (((c.1) as usize) / size.1) as usize * tilesheet_stride_in_tiles,
        ); // start position of each color as an index
        for (j, &(rel_x, rel_y)) in rel_coords.iter().enumerate() {
            let mut bmp = Bitmap::new(size.0, size.1);
            tileset_sheet.draw_on(&mut bmp, -(c.0 + rel_x), -(c.1 + rel_y));

            let flags = if color_lst[i] == bitmap::RED {
                rel_tile_flags[j] | TileFlags::RED
            } else if color_lst[i] == bitmap::BLUE {
                rel_tile_flags[j] | TileFlags::BLUE
            } else if color_lst[i] == bitmap::GREEN {
                rel_tile_flags[j] | TileFlags::GREEN
            } else {
                rel_tile_flags[j]
            };
            let tile_obj = TileStruct {
                // sprite: bmp,
                sprite: bmp,
                index: tiles_per_color * j + i,
                color: color_lst[i],
                flags: flags,
            };
            // println!("tile_obks keys: {:?}", (((c.0+rel_x) as usize)/size.0) as usize + (((c.1 + rel_y) as usize)/size.1) as usize  * tilesheet_stride_in_tiles);

            tile_objs.insert(
                (((c.0 + rel_x) as usize) / size.0) as usize
                    + (((c.1 + rel_y) as usize) / size.1) as usize * tilesheet_stride_in_tiles,
                tile_obj,
            );
        }
    }
    // coords
    //     .iter()
    //     .map(|(x, y)| {
    //         let mut bmp = Bitmap::new(size.0, size.1);
    //         sprite_sheet.draw_on(&mut bmp, -x, -y);
    //         bmp
    //     })
    // .collect::<Vec<_>>()

    // println!("tile_obks: {:?}", tile_objs);

    let mut aura_low = Bitmap::new(16, 16);
    let mut p_vec_sqrd = Vec::<f32>::new();
    for y in 0..aura_low.height {
        let v = y as f32 / aura_low.height as f32;
        for x in 0..aura_low.width {
            let u = x as f32 / aura_low.height as f32;
            let uv = vec2(u, v);
            let p = uv * 2.0 - 1.0;
            let brightness: f32 = (1.0 - (p.length_squared() * 1.2)).clamp(0.0, 1.0);
            // let brightness = (brightness * 8.0) as u32;

            aura_low.plot(x as i32, y as i32, (brightness * 384.0) as u32 | 0xff000000);

        }
    }
    for y in 0..aura_low.height {
        let v = 0.125*y as f32 -1.0; // aura_low.height as f32;
        for x in 0..aura_low.width {
            let u = 0.125* x as f32 - 1.0; // / aura_low.height as f32;
            let p = vec2(u, v);
            // let p = uv * 2.0 - 1.0;
            p_vec_sqrd.push(p.length_squared());
            // let brightness: f32 = (1.0 - (p.length_squared() * 1.2)).clamp(0.0, 1.0);
        }
    }
    let brightness_high = p_vec_sqrd.iter().map(|&p| (((1.0 - (p * 1.2)).clamp(0.0, 1.0))*384.0) as u32  & 0xffff).collect();
    let brightness_low = p_vec_sqrd.iter().map(|&p| (((1.0 - (p * 5.0)).clamp(0.0, 1.0))*150.0) as u32  & 0xffff).collect();
    // println!("brightness high: {:?}",  brightness_high);

    let mut aura = Bitmap::new(256, 256);
    aura_low.draw_on_scaled(&mut aura, 0, 0, 16.0, 16.0);

    let mut aura_low = Bitmap::new(16, 16);
    for y in 0..aura_low.height {
        let v = y as f32 / aura_low.height as f32;
        for x in 0..aura_low.width {
            let u = x as f32 / aura_low.height as f32;
            let uv = vec2(u, v);
            let p = uv * 2.0 - 1.0;
            let brightness = (1.0 - (p.length_squared() * 5.0)).clamp(0.0, 1.0);
            aura_low.plot(x as i32, y as i32, (brightness * 150.0) as u32 | 0xff000000);
        }
    }

    let mut aura2 = Bitmap::new(256, 256);
    aura_low.draw_on_scaled(&mut aura2, 0, 0, 16.0, 16.0);

    TileSet {
        tiles,
        tile_types,
        tile_colors,
        tile_objs,
        unique_tile_colors: color_lst,
        color_start,
        // aura,
        // aura_low: aura2,
        brightness_low,
        brightness: brightness_high,
    }
}

impl Game {
    // consts
    const PLAYER_START_POS: Vec2 = vec2(2200.0, 2110.0);
    const START_COLOR_MASK: bitmap::ColorChannel = bitmap::BLACK;

    pub fn new() -> Self {
        let tile_sheet = Bitmap::load("assets/level_tiles_8x8_v2.png");
        let tile_colors_once = vec![
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

        let color_start_coords = vec![
            // first col
            (0, 0),
            (0, 32),
            (0, 64),
            (0, 96),
            (0, 128),
            (96, 0),
            (96, 32),
            (96, 64),
            (96, 96),
            (96, 128),
        ];
        let tile_coords_per_color = vec![
            (32, 0),  // basic block
            (32, 16), // spike
            (40, 16), // spike
            (48, 16), // spike
            (56, 16), // spike
            (40, 0),  // small platform
            (48, 0),  // small platform
            (56, 0),  // small platform
            (64, 0),  // small platform
            (32, 8),  // filled h platform
            (40, 8),  // filled h platform
            (48, 8),  // filled h platform
            (64, 8),  // filled v platform
            (64, 16),  // filled v platform
            (64, 24),  // filled v platform
            (0, 0),   // upper left corner block
            (8, 0),   // top block
            (16, 0),   // top block
            (24, 0),  // upper right corner block
            (0, 8),   // left block
            (8, 8),   // middle block
            (16, 8),   // middle block
            (24, 8),  // right block
            (0, 16),   // left block
            (8, 16),   // middle block
            (16, 16),   // middle block
            (24, 16),  // right block
            (0, 24),  // lower left corner block
            (8, 24),  // bottom block
            (16, 24),  // bottom block
            (24, 24), // lower right corner
            (32, 24), // middle vertical spike/lava etc.
            (40, 24), // bottom vertical spike/lava etc.
            (48, 24), // top vertical spike/lava etc.
            (56, 24), // top vertical spike/lava etc. filled
        ];
        let tile_flags_per_color = vec![
            TileFlags::COLLISION,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
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
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE | TileFlags::COLLISION,
            TileFlags::SPIKE | TileFlags::COLLISION,
        ];

        let tile_set: TileSet = build_tileset(
            &tile_sheet,
            tile_colors_once,
            &color_start_coords,
            &tile_coords_per_color,
            tile_flags_per_color,
            (8, 8),
        );

        // let tile_map = TileMap::from_file("assets/level0.txt");
        let tile_map = TileMap::from_file("assets/level0.csv");

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
            sprite_inventory_activated: Bitmap::load(
                "assets/sprites/red_mask_in_bag_activated.png",
            ),
            sprite_key_keyboard: Bitmap::load("assets/sprites/red-r.png"),
            keyboard_key_name: "J".to_string(),
            sprite_key_controller: Bitmap::load("assets/sprites/red-b.png"),
            visible: true,
        };

        let green_mask = MaskObject {
            position: vec2(2037.0, 1872.0),
            aabb: Aabb {
                min: Vec2::ZERO,
                max: vec2(MASK_SPRITE_SIZE, MASK_SPRITE_SIZE),
            },
            color: crate::bitmap::GREEN,
            sprite_scene: Bitmap::load("assets/sprites/green_mask_in_scene.png"),
            sprite_inventory: Bitmap::load("assets/sprites/green_mask_in_bag.png"),
            sprite_inventory_activated: Bitmap::load(
                "assets/sprites/green_mask_in_bag_activated.png",
            ),
            sprite_key_keyboard: Bitmap::load("assets/sprites/green-g.png"),
            keyboard_key_name: "L".to_string(),
            sprite_key_controller: Bitmap::load("assets/sprites/green-a.png"),

            visible: true,
        };

        let blue_mask = MaskObject {
            position: vec2(2503.0, 1872.0),
            aabb: Aabb {
                min: Vec2::ZERO,
                max: vec2(MASK_SPRITE_SIZE, MASK_SPRITE_SIZE),
            },
            color: crate::bitmap::BLUE,
            sprite_scene: Bitmap::load("assets/sprites/blue_mask_in_scene.png"),
            sprite_inventory: Bitmap::load("assets/sprites/blue_mask_in_bag.png"),
            sprite_inventory_activated: Bitmap::load(
                "assets/sprites/blue_mask_in_bag_activated.png",
            ),
            sprite_key_keyboard: Bitmap::load("assets/sprites/blue-b.png"),
            keyboard_key_name: "K".to_string(),
            sprite_key_controller: Bitmap::load("assets/sprites/blue-x.png"),
            visible: true,
        };

        let golden_mask = MaskObject {
            position: vec2(2618.0, 2112.0),
            aabb: Aabb {
                min: Vec2::ZERO,
                max: vec2(MASK_SPRITE_SIZE, MASK_SPRITE_SIZE),
            },
            color: crate::bitmap::YELLOW,
            sprite_scene: Bitmap::load("assets/sprites/king_mask_in_scene.png"),
            sprite_inventory: Bitmap::load("assets/sprites/king_mask_in_scene.png"), // not used
            sprite_inventory_activated: Bitmap::load(
                "assets/sprites/king_mask_in_scene.png", // not used
            ),
            sprite_key_keyboard: Bitmap::load("assets/sprites/red-r.png"), // not used
            sprite_key_controller: Bitmap::load("assets/sprites/red-b.png"), // not used
            keyboard_key_name: ".".to_string(),                            // not used
            visible: true,
        };

        let savepoint_1 = SaveGamePoint {
            position: vec2(1809.0, 2176.0),
            aabb: Aabb {
                min: Vec2::ZERO,
                max: vec2(8.0, 8.0),
            },
            color: crate::bitmap::WHITE,
            sprite_scene_off: Bitmap::load("assets/sprites/savepoint_off.png"),
            sprite_scene_on: Bitmap::load("assets/sprites/savepoint_on.png"),
            activated: false,
            visible: true,
        };

        let savepoint_2 = SaveGamePoint {
            position: vec2(2105.0, 2013.0),
            aabb: Aabb {
                min: Vec2::ZERO,
                max: vec2(8.0, 8.0),
            },
            color: crate::bitmap::WHITE,
            sprite_scene_off: Bitmap::load("assets/sprites/savepoint_off.png"),
            sprite_scene_on: Bitmap::load("assets/sprites/savepoint_on.png"),
            activated: false,
            visible: true,
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
        let win_frames = [
            (0, 100),
            (16, 100),
            (32, 100),
            (48, 100),
            (64, 100),
            (80, 100),
        ];

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
            frames: build_frame_list(&player_sprite_sheet, &[(0, 32)], (16, 16)), // TODO: Add anim
            frame_index: 0,
            t: 0.0,
            seconds_per_frame: 1.0 / 4.0,
        };
        let win_sprite = Sprite {
            frames: build_frame_list(&player_sprite_sheet, &win_frames, (16, 16)), // TODO: Own frames
            frame_index: 0,
            t: 0.0,
            seconds_per_frame: 1.0 / 12.0,
        };

        let enemy_sprite_sheet = Bitmap::load("assets/sprite/enemy_sprite.png");
        let enemy_sprite_white = Sprite {
            frames: build_frame_list(
                &enemy_sprite_sheet,
                &(0..8).map(|i| (i * 16, 0)).collect::<Vec<_>>(),
                (16, 8),
            ),
            frame_index: 0,
            t: 0.0,
            seconds_per_frame: 1.0 / 24.0,
        };
        let enemy_sprite_red = Sprite {
            frames: build_frame_list(
                &enemy_sprite_sheet,
                &(0..8).map(|i| (i * 16, 8)).collect::<Vec<_>>(),
                (16, 8),
            ),
            frame_index: 0,
            t: 0.0,
            seconds_per_frame: 1.0 / 24.0,
        };
        let enemy_sprite_green = Sprite {
            frames: build_frame_list(
                &enemy_sprite_sheet,
                &(0..8).map(|i| (i * 16, 16)).collect::<Vec<_>>(),
                (16, 8),
            ),
            frame_index: 0,
            t: 0.0,
            seconds_per_frame: 1.0 / 24.0,
        };
        let enemy_sprite_blue = Sprite {
            frames: build_frame_list(
                &enemy_sprite_sheet,
                &(0..8).map(|i| (i * 16, 24)).collect::<Vec<_>>(),
                (16, 8),
            ),
            frame_index: 0,
            t: 0.0,
            seconds_per_frame: 1.0 / 24.0,
        };
        let mut audio = None;
        if AUDIO_ON {
            audio = Some(Audio::new())
        }

        let mut game = Self {
            audio,
            music_mode: false,
            font: Font::new_default(),

            actual_camera: vec2(2000.0, 2000.0),
            camera: vec2(2000.0, 2000.0),

            input_state: InputState::default(),

            save_state: None,

            editor_state: EditorState::default(),

            background: Background::new(),
            tile_set,
            tile_map,

            mouse_x: 0.0,
            mouse_y: 0.0,

            // Add game objects
            mask_game_objects: vec![red_mask, green_mask, blue_mask, golden_mask],
            savepoint_objects: vec![savepoint_1, savepoint_2],
            enemies: vec![],

            enemy_sprite_white,
            enemy_sprite_red,
            enemy_sprite_green,
            enemy_sprite_blue,

            died_position: Vec2::ZERO,
            player: Player {
                idle_sprite,
                walk_sprite,
                jump_sprite,
                death_sprite,
                win_sprite,
                position: Self::PLAYER_START_POS,
                velocity: Vec2::ZERO,
                aabb: Aabb {
                    min: vec2(3.0, 5.0),
                    max: vec2(12.0, 15.0),
                },
                on_ground: false,
                is_jumping: false,
                is_dead: false,
                is_winner: false,
            },
            player_inventory: PlayerInventory {
                tile_size: 16,
                width: 256,
                height: 64,
                position_on_screen: vec2(0.0, 180.0),
                background_color: 0xffffefd5,
                bag_sprite: bag_sprite,
                masks: Vec::new(),
            },
            is_player_walking: false,
            was_player_walking: false,
            time: 0.0,

            player_uses_controller: true,

            death_sequence_duration: 1.5,
            death_sequence_is_playing: false,

            winning_sequence_duration: 2.5,
            winning_sequence_is_playing: false,

            color_mask: Self::START_COLOR_MASK,
            editor_mode: false,
            lerp_color_mask: Vec3::ZERO,
        };

        game.reset_game();
        game
    }

    pub fn reset_game(&mut self) {
        // Reset player
        self.player.position = Self::PLAYER_START_POS;
        self.player.on_ground = false;
        self.player.is_jumping = false;
        self.player.is_dead = false;
        self.player.is_winner = false;

        self.enemies = spawn_enemies(
            &self.enemy_sprite_white,
            &self.enemy_sprite_red,
            &self.enemy_sprite_green,
            &self.enemy_sprite_blue,
        );

        // Reset inventory
        self.player_inventory.masks.clear();

        // Reset game objects
        for mask in self.mask_game_objects.iter_mut() {
            mask.visible = true;
        }
        self.color_mask = Self::START_COLOR_MASK;

        // Reset death sequence
        self.death_sequence_duration = 1.5;
        self.death_sequence_is_playing = false;

        self.winning_sequence_duration = 2.5;
        self.winning_sequence_is_playing = false;

        self.restore_save_game();
    }

    pub fn restore_save_game(&mut self) {
        if let Some(save_state) = &self.save_state {
            self.player.position = save_state.player_position;
            if save_state.has_red_mask {
                for mask in self.mask_game_objects.iter_mut() {
                    if mask.color == bitmap::RED {
                        self.player_inventory.masks.push(mask.clone());
                        mask.visible = false;
                    }
                }
            }
            if save_state.has_blue_mask {
                for mask in self.mask_game_objects.iter_mut() {
                    if mask.color == bitmap::BLUE {
                        self.player_inventory.masks.push(mask.clone());
                        mask.visible = false;
                    }
                }
            }
            if save_state.has_green_mask {
                for mask in self.mask_game_objects.iter_mut() {
                    if mask.color == bitmap::GREEN {
                        self.player_inventory.masks.push(mask.clone());
                        mask.visible = false;
                    }
                }
            }
            self.color_mask = save_state.color_mask;
        }
    }

    pub(crate) fn on_mouse_moved(&mut self, x: f32, y: f32) {
        self.input_state.mouse = vec2(x, y);
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
            Key::EditMode => self.editor_mode = !self.editor_mode,
            Key::MusicMode => self.music_mode = !self.music_mode,
            Key::MuteAudio => {
                if let Some(_audio) = &self.audio {
                    self.audio = None;
                    println!("turning audio off");
                } else {
                    self.audio = Some(Audio::new());
                    println!("turning audio on");
                }
            }
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

    pub(crate) fn on_axis_change(&mut self, axis: Axis, value: f32) {
        self.input_state.axis_state[axis as usize] = value;
    }

    pub fn set_color_mask(&mut self, color_channel: crate::bitmap::ColorChannel) {
        self.color_mask = color_channel;
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
        let delta_time = delta_time.min(1.0 / 30.0);
        self.time += delta_time;

        screen.clear(0);

        let screen_offset = vec2(128.0, 104.0 + 32.0);
        let target = if self.player.is_dead {
            Aabb {
                min: self.player.aabb.min + self.died_position,
                max: self.player.aabb.max + self.died_position,
            }
            .center()
                - screen_offset
        } else {
            let target = self.player.aabb_world_space().center() - screen_offset;
            target + self.player.velocity * vec2(0.35, 0.1)
        };
        self.actual_camera = self.actual_camera.lerp(target, delta_time * 4.0);

        if !self.editor_mode {
            self.camera = self.actual_camera.round()
        }

        {
            let r = ((self.color_mask >> 16) & 0xff) as f32 / 255.0;
            let g = ((self.color_mask >> 8) & 0xff) as f32 / 255.0;
            let b = ((self.color_mask) & 0xff) as f32 / 255.0;
            self.lerp_color_mask = self.lerp_color_mask.lerp(vec3(r, g, b), delta_time * 5.0);
        }

        let color_mask_uvec3 = (self.lerp_color_mask * 8.0).as_uvec3() * 32;

        if let Some(audio) = &self.audio {
            audio.color_mask_sender.send(color_mask_uvec3).unwrap();
        }

        let lerped_color_mask =
            color_mask_uvec3.x << 16 | color_mask_uvec3.y << 8 | color_mask_uvec3.z | 0xff000000;

        let aura_translation =
            world_space_to_screen_space(self.player.position, self.camera) - vec2(128.0, 128.0);
        let aura_translation = aura_translation.as_ivec2();

        self.background.draw(
            screen,
            self.camera,
            lerped_color_mask,
            &self.tile_set.brightness_low,
            &self.tile_set.brightness,
            aura_translation,
        );

        self.tile_map.draw(
            &self.tile_set,
            screen,
            self.camera,
            self.color_mask,
            lerped_color_mask,
            aura_translation,
            self.editor_mode,
        );

        // draw inventory on top
        // TODO: Could make inventory-overlay its own bitmap and draw items on that and then draw the inventory on the screen
        if self.editor_mode {
            screen.draw_str(&self.font, "editor_mode", 191, 10, 0xffff00);
            self.editor_state.tick(
                delta_time,
                screen,
                &mut self.tile_map,
                &self.tile_set,
                &mut self.camera,
                &self.input_state,
            );
        } else {
            self.player_inventory.bag_sprite.draw_on(
                screen,
                self.player_inventory.position_on_screen.x as i32,
                self.player_inventory.position_on_screen.y as i32,
            );
            for i in 0..self.player_inventory.masks.len() {
                if self.player_inventory.masks[i].color == self.color_mask {
                    self.player_inventory.masks[i]
                        .sprite_inventory_activated
                        .draw_on(
                            screen,
                            self.player_inventory.position_on_screen.x as i32
                                + (i as i32 + 2) * self.player_inventory.tile_size,
                            self.player_inventory.position_on_screen.y as i32,
                        );
                } else {
                    self.player_inventory.masks[i].sprite_inventory.draw_on(
                        screen,
                        self.player_inventory.position_on_screen.x as i32
                            + (i as i32 + 2) * self.player_inventory.tile_size,
                        self.player_inventory.position_on_screen.y as i32,
                    );
                }

                // Draw key hint
                if self.player_uses_controller {
                    self.player_inventory.masks[i]
                        .sprite_key_controller
                        .draw_on(
                            screen,
                            self.player_inventory.position_on_screen.x as i32
                                + (i as i32 + 2) * self.player_inventory.tile_size,
                            self.player_inventory.position_on_screen.y as i32 + 12,
                        )
                } else {
                    screen.draw_str(
                        &self.font,
                        self.player_inventory.masks[i].keyboard_key_name.as_str(),
                        self.player_inventory.position_on_screen.x as i32
                            + (i as i32 + 2) * self.player_inventory.tile_size
                            + 6,
                        self.player_inventory.position_on_screen.y as i32 + 18,
                        self.player_inventory.masks[i].color,
                    );

                    screen.draw_rectangle(
                        self.player_inventory.position_on_screen.x as i32
                            + (((i as f32 + 2.25) * (self.player_inventory.tile_size as f32))
                                as i32),
                        self.player_inventory.position_on_screen.y as i32
                            + self.player_inventory.tile_size,
                        self.player_inventory.position_on_screen.x as i32
                            + (((i as f32 + 2.75) * (self.player_inventory.tile_size as f32))
                                as i32),
                        self.player_inventory.position_on_screen.y as i32
                            + 3 * (self.player_inventory.tile_size >> 1),
                        false,
                        self.player_inventory.masks[i].color,
                    );
                    // self.player_inventory.masks[i].sprite_key_keyboard.draw_on(
                    //     screen,
                    //     self.player_inventory.position_on_screen.x as i32
                    //         + (i as i32 + 2) * self.player_inventory.tile_size,
                    //     self.player_inventory.position_on_screen.y as i32 + 12,
                    // )
                }
            }
        }

        for enemy in self.enemies.iter_mut() {
            enemy.tick(delta_time, &self.tile_map, &self.tile_set);

            if !enemy.is_colored() || (self.color_mask & enemy.color_mask) & 0xffffff != 0 {
                if self
                    .player
                    .aabb_world_space()
                    .overlaps(&enemy.hitbox_aabb_world_space())
                {
                    self.player.is_dead = true;
                }
            }

            enemy.draw(
                screen,
                self.camera,
                lerped_color_mask & 0xffffff,
                &self.tile_set.brightness_low,
                &self.tile_set.brightness,
                aura_translation,
            );
        }

        // If we won, play winning sequence
        if self.player.is_winner {
            // let desired_position = vec2(
            //     self.player.position.x,
            //     self.player.position.y - self.player.win_sprite.frames[0].height as f32,
            // );
            // let desired_scale_scalar = 2.0;

            // Just won
            if !self.winning_sequence_is_playing {}

            self.winning_sequence_duration -= delta_time;
            screen.draw_str(&self.font, "U WON :)", 100, 50, bitmap::GREEN);
            self.save_state = None;

            // Reset game objects
            for savepoint in self.savepoint_objects.iter_mut() {
                savepoint.activated = false;
            }

            // Some(SaveState {
            //     color_mask: Self::START_COLOR_MASK,
            // player_position: Self::PLAYER_START_POS,
            // has_blue_mask: false,
            // has_red_mask: false,
            // has_green_mask: false,
            // });

            if self.winning_sequence_duration < 0.0 {
                self.reset_game();
            } else {
                self.player.tick(delta_time);
                self.player.draw(screen, self.camera, self.color_mask); // draw with golden mask

                // TODO: I was thinking we could lerp to bigger scale & higher position but it needs fixing with aligning with pixels --> looks jerky now
                // Lerp to pos
                // let start_pos = self.player.position;
                // let player_pos = desired_position.lerp(start_pos, self.winning_sequence_duration / (2.5 * 2.0)); // lerping the wrong way

                // // Draw
                // let scale = vec2(if self.player.velocity.x < 0.0 { -1.0 } else { 1.0 }, 1.0);
                // let start_scale = 1.0;
                // let scale_scalar = desired_scale_scalar.lerp(start_scale, self.winning_sequence_duration / (2.5 * 2.0)); // lerping the wrong way

                // let screen_pos = world_space_to_screen_space(player_pos, self.camera);
                // // self.player.tick(delta_time);
                // self.player.win_sprite
                //     .draw_player(screen, screen_pos, scale * scale_scalar, 0xffff00); // draw with golden mask
            }
            return;
        }

        if !DEBUG_MODE {
            // If we are death, play fixed death sequence and restart the game
            if self.player.is_dead {
                // just died
                if !self.death_sequence_is_playing {
                    self.died_position = self.player.position;
                    self.player.velocity.y = -2.0 * JUMP_IMPULSE;
                    self.death_sequence_is_playing = true;

                    if let Some(audio) = &self.audio {
                        audio
                            .sfx_sender
                            .send((SoundTypes::DeathSound, true))
                            .unwrap();
                    }
                }
                self.death_sequence_duration -= delta_time;
                screen.draw_str(&self.font, "U DIED :(", 100, 50, bitmap::RED);

                if self.death_sequence_duration < 0.0 {
                    self.reset_game();
                } else {
                    self.player.velocity.y += GRAVITY * delta_time;
                    self.player.position.y += self.player.velocity.y * delta_time;
                    self.player.tick(delta_time);
                    self.player.draw(screen, self.camera, self.color_mask);
                }
                return;
            }
        }

        // Some things we only need to do if we aren't dead
        if !self.editor_mode {
            self.is_player_walking = false;

            // controller input
            let mut movement_axis = self.input_state.axis_state(Axis::LeftStickX);
            if movement_axis.abs() < 0.1 {
                self.player_uses_controller = false;

                // keyboard input
                if self.input_state.is_key_down(Key::MoveLeft) {
                    movement_axis -= 1.0;
                }
                if self.input_state.is_key_down(Key::MoveRight) {
                    movement_axis += 1.0;
                }
            } else {
                self.player_uses_controller = true;
            }

            // do game things here
            if movement_axis < 0.0 {
                self.player.velocity.x = self.player.velocity.x.min(0.0);
                self.player.velocity.x += MOVEMENT_ACCELERATION * delta_time * movement_axis;

                if self.player.on_ground {
                    self.is_player_walking = true;
                }
            }
            if movement_axis > 0.0 {
                self.player.velocity.x = self.player.velocity.x.max(0.0);
                self.player.velocity.x += MOVEMENT_ACCELERATION * delta_time * movement_axis;

                if self.player.on_ground {
                    self.is_player_walking = true;
                }
            }

            if movement_axis == 0.0 {
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

            if self.input_state.is_key_pressed(Key::Jump) && self.player.on_ground {
                self.player.velocity.y = -JUMP_IMPULSE;
                self.player.is_jumping = true;

                if let Some(audio) = &self.audio {
                    audio
                        .sfx_sender
                        .send((SoundTypes::JumpSound, true))
                        .unwrap();
                }
            }
            if self.input_state.is_key_down(Key::Jump) {
                if self.player.is_jumping {
                    self.player.velocity.y -= JUMP_SUSTAIN * delta_time;
                }
            } else {
                self.player.is_jumping = false;
            }

            if DEBUG_MASKS {
                if self.input_state.is_key_pressed(Key::MaskRed) {
                    self.toggle_color_mask(0xff0000);
                }
                if self.input_state.is_key_pressed(Key::MaskGreen) {
                    self.toggle_color_mask(0x00ff00);
                }
                if self.input_state.is_key_pressed(Key::MaskBlue) {
                    self.toggle_color_mask(0x0000ff);
                }
            } else {
                // Current situ: activating a new mask disables old mask (can't wear two masks)
                if self.input_state.is_key_pressed(Key::MaskRed) {
                    if let Some(red_mask) = self
                        .player_inventory
                        .masks
                        .iter()
                        .find(|&x| x.color == bitmap::RED)
                    {
                        // self.toggle_color_mask(red_mask.color);
                        self.set_color_mask(red_mask.color);
                    };
                }
                if self.input_state.is_key_pressed(Key::MaskGreen) {
                    if let Some(green_mask) = self
                        .player_inventory
                        .masks
                        .iter()
                        .find(|&x| x.color == bitmap::GREEN)
                    {
                        // self.toggle_color_mask(green_mask.color);
                        self.set_color_mask(green_mask.color);
                    };
                }
                if self.input_state.is_key_pressed(Key::MaskBlue) {
                    if let Some(blue_mask) = self
                        .player_inventory
                        .masks
                        .iter()
                        .find(|&x| x.color == bitmap::BLUE)
                    {
                        // self.toggle_color_mask(blue_mask.color);
                        self.set_color_mask(blue_mask.color);
                    };
                }
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
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_left[1],
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_left[2],
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
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
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_right[1],
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_right[2],
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
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
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_below[1],
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_below[2],
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
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
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_above[1],
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
                    ),
                    self.tile_map.sample_tile_type_ws(
                        samples_positions_above[2],
                        &self.tile_set.tile_objs,
                        &self.tile_set.tile_types,
                        &self.tile_set.tile_colors,
                        self.color_mask,
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

            if self.is_player_walking && !self.was_player_walking {
                // send signal to start playing walking sound
                if let Some(audio) = &self.audio {
                    audio
                        .sfx_sender
                        .send((SoundTypes::FootstepSound, true))
                        .unwrap();
                }
            } else if self.was_player_walking && !self.is_player_walking {
                // send signal to stop playing walking sound
                if let Some(audio) = &self.audio {
                    audio
                        .sfx_sender
                        .send((SoundTypes::FootstepSound, false))
                        .unwrap();
                }
            }

            self.was_player_walking = self.is_player_walking;
        }

        self.player.draw(screen, self.camera, self.color_mask);

        // Loop over masks
        for savepoint in self.savepoint_objects.iter_mut() {
            if savepoint.visible {
                let pos: Vec2 = world_space_to_screen_space(savepoint.position, self.camera);
                if savepoint.activated {
                    savepoint
                        .sprite_scene_on
                        .draw_on(screen, pos.x as i32, pos.y as i32);
                } else {
                    savepoint
                        .sprite_scene_off
                        .draw_on(screen, pos.x as i32, pos.y as i32);
                }
                // Save and turn on if position overlaps with player
                if savepoint
                    .aabb_world_space()
                    .overlaps(&self.player.aabb_world_space())
                {
                    if !savepoint.activated {
                        savepoint.activated = true;
                        savepoint
                            .sprite_scene_on
                            .draw_on(screen, pos.x as i32, pos.y as i32);

                        if let Some(audio) = &self.audio {
                            audio
                                .sfx_sender
                                .send((SoundTypes::PickupSound, true))
                                .unwrap();
                        }
                    };

                    self.save_state = Some(SaveState {
                        player_position: self.player.position,
                        has_red_mask: self
                            .player_inventory
                            .masks
                            .iter()
                            .find(|mask| mask.color == bitmap::RED)
                            .is_some(),
                        has_green_mask: self
                            .player_inventory
                            .masks
                            .iter()
                            .find(|mask| mask.color == bitmap::GREEN)
                            .is_some(),
                        has_blue_mask: self
                            .player_inventory
                            .masks
                            .iter()
                            .find(|mask| mask.color == bitmap::BLUE)
                            .is_some(),
                        color_mask: self.color_mask,
                    });
                }
            }
        }

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
                    self.color_mask = mask.color;
                    mask.visible = false;

                    // Special case for the golden mask
                    if mask.color == bitmap::YELLOW {
                        self.player.is_winner = true;
                        return;
                    }

                    self.player_inventory.masks.push(mask.clone());

                    self.save_state = Some(SaveState {
                        player_position: self.player.position,
                        has_red_mask: self
                            .player_inventory
                            .masks
                            .iter()
                            .find(|mask| mask.color == bitmap::RED)
                            .is_some(),
                        has_green_mask: self
                            .player_inventory
                            .masks
                            .iter()
                            .find(|mask| mask.color == bitmap::GREEN)
                            .is_some(),
                        has_blue_mask: self
                            .player_inventory
                            .masks
                            .iter()
                            .find(|mask| mask.color == bitmap::BLUE)
                            .is_some(),
                        color_mask: self.color_mask,
                    });

                    if let Some(audio) = &self.audio {
                        audio
                            .sfx_sender
                            .send((SoundTypes::PickupSound, true))
                            .unwrap();
                    }
                }
            }
        }

        if DEBUG_MODE {
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
        }

        // reset state
        self.input_state.reset();
    }
}
