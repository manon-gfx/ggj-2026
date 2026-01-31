pub mod editor;
pub mod sprite;
pub mod tilemap;

use crate::audio::Audio;
use crate::audio::sound::SoundTypes;
use crate::bitmap::{self, Bitmap, Font};
use crate::game::sprite::Sprite;
use editor::EditorState;
use glam::*;

use tilemap::{TileFlags, TileMap, TileSet};

const GRAVITY: f32 = 600.0;
const JUMP_IMPULSE: f32 = 150.0;
const JUMP_SUSTAIN: f32 = 350.0;
const MOVEMENT_ACCELERATION: f32 = 1500.0;
const MOVEMENT_SPEED_X: f32 = 100.0;
const FRICTION: f32 = 1500.0;

const DEBUG_MASKS: bool = false;
const DEBUG_MODE: bool = false;

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
    mouse: Vec2,

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

    background: Bitmap,
    test_sprite: Bitmap,

    mouse_x: f32,
    mouse_y: f32,

    mask_game_objects: Vec<MaskObject>,

    player: Player,
    player_inventory: PlayerInventory,
    is_player_walking: bool,
    was_player_walking: bool,
    time: f32,

    death_sequence_is_playing: bool,
    death_sequence_duration: f32,

    editor_mode: bool,

    color_mask: crate::bitmap::ColorChannel,
    lerp_color_mask: Vec3,
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
    // consts
    const PLAYER_START_POS: Vec2 = vec2(2200.0, 2110.0);
    const START_COLOR_MASK: bitmap::ColorChannel = bitmap::BLACK;

    pub fn new() -> Self {
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
            TileFlags::COLLISION | TileFlags::RED,
            TileFlags::COLLISION | TileFlags::BLUE,
            TileFlags::COLLISION | TileFlags::GREEN,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::COLLISION,
            TileFlags::SPIKE,
            TileFlags::SPIKE | TileFlags::RED,
            TileFlags::SPIKE | TileFlags::BLUE,
            TileFlags::SPIKE | TileFlags::GREEN,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
            TileFlags::SPIKE,
        ];

        let tiles = build_frame_list(&tile_sheet, &coords, (8, 8));

        let mut aura_low = Bitmap::new(16, 16);
        for y in 0..aura_low.height {
            let v = y as f32 / aura_low.height as f32;
            for x in 0..aura_low.width {
                let u = x as f32 / aura_low.height as f32;
                let uv = vec2(u, v);
                let p = uv * 2.0 - 1.0;
                let brightness = (1.0 - (p.length_squared() * 1.2)).clamp(0.0, 1.0);
                // let brightness = (brightness * 8.0) as u32;
                aura_low.plot(x as i32, y as i32, (brightness * 384.0) as u32 | 0xff000000);
            }
        }
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

        let tile_set = TileSet {
            tiles,
            tile_types,
            tile_colors,
            aura,
            aura_low: aura2,
        };
        let tile_map = TileMap::from_file("assets/level0.txt");

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
            sprite_inventory_activated: Bitmap::load(
                "assets/sprites/red_mask_in_bag_activated.png",
            ),
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
            sprite_inventory_activated: Bitmap::load(
                "assets/sprites/green_mask_in_bag_activated.png",
            ),
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
            sprite_inventory_activated: Bitmap::load(
                "assets/sprites/blue_mask_in_bag_activated.png",
            ),
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

        let mut background_part = Bitmap::new(8, 8);
        tile_sheet.draw_on(&mut background_part, -128, -64);
        let mut background = Bitmap::new(16, 16);
        background_part.draw_on(&mut background, 0, 0);
        background_part.draw_on(&mut background, 8, 0);
        background_part.draw_on(&mut background, 0, 8);
        background_part.draw_on(&mut background, 8, 8);

        Self {
            // audio: Some(Audio::new()),
            audio: None,
            music_mode: false,
            font: Font::new_default(),

            test_sprite: player_sprite,

            camera: vec2(2000.0, 2000.0),

            input_state: InputState::default(),

            editor_state: EditorState::default(),

            background,
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
                position: Self::PLAYER_START_POS,
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
            is_player_walking: false,
            was_player_walking: false,
            time: 0.0,

            death_sequence_duration: 1.5,
            death_sequence_is_playing: false,

            color_mask: Self::START_COLOR_MASK,
            editor_mode: false,
            lerp_color_mask: Vec3::ZERO,
        }
    }

    pub fn reset_game(&mut self) {
        // Reset player
        self.player.position = Self::PLAYER_START_POS;
        self.player.on_ground = false;
        self.player.is_jumping = false;
        self.player.is_dead = false;

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
        let delta_time = delta_time.min(1.0 / 30.0);
        self.time += delta_time;

        screen.clear(0);

        if !self.editor_mode && !self.player.is_dead {
            self.camera = self.player.position - vec2(132.0, 128.0);
        }

        {
            let r = ((self.color_mask >> 16) & 0xff) as f32 / 255.0;
            let g = ((self.color_mask >> 8) & 0xff) as f32 / 255.0;
            let b = ((self.color_mask) & 0xff) as f32 / 255.0;
            self.lerp_color_mask = self.lerp_color_mask.lerp(vec3(r, g, b), delta_time * 5.0);
        }

        let color_mask_uvec3 = (self.lerp_color_mask * 8.0).as_uvec3() * 32;
        let lerped_color_mask =
            color_mask_uvec3.x << 16 | color_mask_uvec3.y << 8 | color_mask_uvec3.z | 0xff000000;

        let background_offset = -(self.camera * 0.2) % 256.0;
        self.background.draw_background(
            screen,
            background_offset.x as i32,
            background_offset.y as i32,
            16.0,
            16.0,
            lerped_color_mask,
            &self.tile_set.aura_low,
            &self.tile_set.aura,
        );
        self.background.draw_background(
            screen,
            background_offset.x as i32 + 256,
            background_offset.y as i32,
            16.0,
            16.0,
            lerped_color_mask,
            &self.tile_set.aura_low,
            &self.tile_set.aura,
        );
        self.background.draw_background(
            screen,
            background_offset.x as i32,
            background_offset.y as i32 + 256,
            16.0,
            16.0,
            lerped_color_mask,
            &self.tile_set.aura_low,
            &self.tile_set.aura,
        );
        self.background.draw_background(
            screen,
            background_offset.x as i32 + 256,
            background_offset.y as i32 + 256,
            16.0,
            16.0,
            lerped_color_mask,
            &self.tile_set.aura_low,
            &self.tile_set.aura,
        );

        self.tile_map.draw(
            &self.tile_set,
            screen,
            self.camera,
            if self.editor_mode {
                0xffffffff
            } else {
                self.color_mask
            },
            lerped_color_mask,
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
            }
        }

        if !DEBUG_MODE {
            // If we are death, play fixed death sequence and restart the game
            if self.player.is_dead {
                // just died
                if !self.death_sequence_is_playing {
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
                    self.player.draw(screen, self.camera);
                }
                return;
            }
        }

        // Some things we only need to do if we aren't dead
        if !self.editor_mode {
            self.is_player_walking = false;

            // do game things here
            if self.input_state.is_key_down(Key::Left) {
                self.player.velocity.x = self.player.velocity.x.min(0.0);
                self.player.velocity.x -= MOVEMENT_ACCELERATION * delta_time;

                if self.player.on_ground {
                    self.is_player_walking = true;
                }
            }
            if self.input_state.is_key_down(Key::Right) {
                self.player.velocity.x = self.player.velocity.x.max(0.0);
                self.player.velocity.x += MOVEMENT_ACCELERATION * delta_time;

                if self.player.on_ground {
                    self.is_player_walking = true;
                }
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

                if let Some(audio) = &self.audio {
                    audio
                        .sfx_sender
                        .send((SoundTypes::JumpSound, true))
                        .unwrap();
                }
            }
            if self.input_state.is_key_down(Key::A) {
                if self.player.is_jumping {
                    self.player.velocity.y -= JUMP_SUSTAIN * delta_time;
                }
            } else {
                self.player.is_jumping = false;
            }

            if DEBUG_MASKS {
                if self.input_state.is_key_released(Key::R) {
                    self.toggle_color_mask(0xff0000);
                }
                if self.input_state.is_key_released(Key::G) {
                    self.toggle_color_mask(0x00ff00);
                }
                if self.input_state.is_key_released(Key::B) {
                    self.toggle_color_mask(0x0000ff);
                }
            } else {
                // Current situ: activating a new mask disables old mask (can't wear two masks)
                if self.input_state.is_key_released(Key::R) {
                    if let Some(red_mask) = self
                        .player_inventory
                        .masks
                        .iter()
                        .find(|&x| x.activation_key == Key::R)
                    {
                        // self.toggle_color_mask(red_mask.color);
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
                        // self.toggle_color_mask(green_mask.color);
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
