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

        //     let mut sw = bitmap.width as i32;
        //     let mut sh = bitmap.height as i32;

        //     let x = position.x as i32;
        //     let y = position.y as i32;

        //     let (sx, tx) = if x < 0 {
        //         sw += x;
        //         (x.abs(), 0)
        //     } else {
        //         (0, x)
        //     };
        //     let (sy, ty) = if y < 0 {
        //         sh += y;
        //         (y.abs(), 0)
        //     } else {
        //         (0, y)
        //     };

        //     sw = sw.min(target.width as i32 - tx);
        //     sh = sh.min(target.height as i32 - ty);

        //     for y in 0..sh {
        //         let line0 = (ty + y) * (target.stride as i32);
        //         let line1 = (sy + y) * (bitmap.stride as i32);
        //         for x in 0..sw {
        //             unsafe {
        //                 let c = *bitmap.pixels().get_unchecked((line1 + sx + x) as usize);
        //                 if (c & 0xff000000) != 0 {
        //                     // alpha
        //                     *target
        //                         .pixels_mut()
        //                         .get_unchecked_mut((line0 + tx + x) as usize) = c;
        //                 }
        //             }
        //         }
        //     }
    }
}
