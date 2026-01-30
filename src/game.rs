use crate::bitmap::{Bitmap, Font};
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

pub struct Game {
    audio_manager: AudioManager<DefaultBackend>,

    font: Font,

    test_sprite: Bitmap,
    test_sound: StaticSoundData,

    mouse_x: i32,
    mouse_y: i32,

    player_x: i32,
    player_y: i32,

    time: f32,

    color_mask: crate::bitmap::ColorChannel,
}

impl Game {
    pub fn new() -> Self {
        let audio_manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default())
            .expect("Failed to initialize audio manager");
        let test_sound =
            StaticSoundData::from_file("assets/test_sound.wav").expect("Failed to load sound");

        Self {
            audio_manager,
            font: Font::new_default(),

            test_sprite: Bitmap::load("assets/test_sprite.png"),
            test_sound,

            mouse_x: 0,
            mouse_y: 0,

            player_x: 200,
            player_y: 200,

            time: 0.0,

            color_mask: crate::bitmap::RED,
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
    }
}
