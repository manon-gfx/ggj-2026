use crate::bitmap::{Bitmap, Font};
use glam::*;
use kira::{
    AudioManager, AudioManagerSettings, DefaultBackend, sound::static_sound::StaticSoundData,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
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

impl TileMap {
    fn draw(&self, tile_set: &TileSet, target: &mut Bitmap) {
        for y in 0..self.height {
            for x in 0..self.width {
                let tile_index = self.tiles[(y * self.width + x) as usize];
                if tile_index != 0 {
                    let tile = &tile_set.tiles[(tile_index - 1) as usize];
                    tile.draw_on(
                        target,
                        (x * self.tile_size) as i32,
                        (y * self.tile_size) as i32,
                    );
                }
            }
        }
    }
}

pub struct Game {
    audio_manager: AudioManager<DefaultBackend>,

    font: Font,

    tile_set: TileSet,
    tile_map: TileMap,

    test_sprite: Bitmap,
    test_sound: StaticSoundData,

    mouse_x: i32,
    mouse_y: i32,

    player_x: i32,
    player_y: i32,

    time: f32,
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
        let audio_manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .expect("Failed to initialize audio manager");
        let test_sound =
            StaticSoundData::from_file("assets/test_sound.wav").expect("Failed to load sound");

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
            audio_manager,
            font: Font::new_default(),

            test_sprite: Bitmap::load("assets/test_sprite.png"),
            test_sound,

            tile_set,
            tile_map,

            mouse_x: 0,
            mouse_y: 0,

            player_x: 200,
            player_y: 200,

            time: 0.0,
        }
    }

    pub(crate) fn on_mouse_moved(&mut self, x: i32, y: i32) {
        self.mouse_x = x;
        self.mouse_y = y;
    }
    pub(crate) fn on_mouse_button_down(&mut self, _button: super::MouseButton, _x: i32, _y: i32) {}
    pub(crate) fn on_mouse_button_up(&mut self, _button: super::MouseButton, _x: i32, _y: i32) {}
    pub(crate) fn on_key_down(&mut self, key: Key) {
        match key {
            Key::Up => self.player_y -= 10,
            Key::Down => self.player_y += 10,
            Key::Left => self.player_x -= 10,
            Key::Right => self.player_x += 10,
            Key::A => {
                self.audio_manager
                    .play(self.test_sound.clone())
                    .expect("Failed to play sound");
            }
            _ => {}
        }
    }
    pub(crate) fn on_key_up(&mut self, _key: Key) {}

    pub fn tick(&mut self, delta_time: f32, screen: &mut Bitmap) {
        self.time += delta_time;

        screen.clear(0);

        self.tile_map.draw(&self.tile_set, screen);

        self.test_sprite
            .draw_on(screen, self.player_x, self.player_y);

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
    }
}
