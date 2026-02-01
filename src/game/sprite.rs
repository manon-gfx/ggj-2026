use crate::bitmap::{self, Bitmap};
use glam::*;

#[derive(Debug, Clone)]
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

    pub fn draw_colored(
        &self,
        target: &mut Bitmap,
        position: Vec2,
        scale: Vec2,
        is_colored: bool,
        visible_mask: u32,
        lerped_color_mask: u32,
        aura_low: &Bitmap,
        aura: &Bitmap,
        aura_transl: IVec2,
    ) {
        let bitmap = &self.frames[self.frame_index];
        bitmap.draw_on_scaled_colored_obj(
            target,
            position.x as i32,
            position.y as i32,
            scale.x,
            scale.y,
            is_colored,
            visible_mask,
            lerped_color_mask,
            aura_low,
            aura,
            aura_transl,
        );
    }

    pub fn draw_player(&self, target: &mut Bitmap, position: Vec2, scale: Vec2, color_mask: u32) {
        let components = uvec3(
            (color_mask >> 16) & 0xff,
            (color_mask >> 8) & 0xff,
            color_mask & 0xff,
        );
        let color_index = if (color_mask & 0xffffff) == 0 {
            0
        } else if color_mask == bitmap::YELLOW {
            4
        } 
        else {
            if components.x > components.y {
                if components.x > components.z { 1 } else { 3 }
            } else if components.y > components.z {
                2
            } else {
                3
            }
        };

        let bitmap = &self.frames[self.frame_index];
        bitmap.draw_on_scaled_player(
            target,
            position.x as i32,
            position.y as i32,
            scale.x,
            scale.y,
            color_index,
        );
    }
}
