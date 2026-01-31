use crate::bitmap::Bitmap;
use glam::*;

#[derive(Debug)]
pub struct Sprite {
    pub frames: Vec<Bitmap>,
    pub frame_index: usize,
    pub t: f32,
    pub seconds_per_frame: f32,
}

impl Sprite {
    pub fn tick(&mut self, delta_time: f32) {
        self.t += delta_time;
        while self.t > self.seconds_per_frame {
            self.t -= self.seconds_per_frame;
            self.frame_index = (self.frame_index + 1) % self.frames.len();
        }
    }
    pub fn draw(&self, target: &mut Bitmap, position: Vec2, scale: Vec2) {
        let bitmap = &self.frames[self.frame_index];
        bitmap.draw_on_scaled(
            target,
            position.x as i32,
            position.y as i32,
            scale.x,
            scale.y,
        );
    }
}
