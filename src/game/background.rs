use crate::bitmap::Bitmap;
use glam::*;

pub struct Background {
    pub layers: Vec<Bitmap>,
}

impl Background {
    pub fn new() -> Self {
        let layers = vec![
            Bitmap::load("assets/backgrounds/1.png"),
            Bitmap::load("assets/backgrounds/2.png"),
            Bitmap::load("assets/backgrounds/3.png"),
            Bitmap::load("assets/backgrounds/4.png"),
            Bitmap::load("assets/backgrounds/5.png"),
        ];

        Self { layers }
    }

    pub fn draw(
        &self,
        screen: &mut Bitmap,
        camera: Vec2,
        lerped_color_mask: u32,
        aura_low: &Bitmap,
        aura: &Bitmap,
        aura_transl: IVec2,
    ) {
        let offset_scales = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5];
        for (i, layer) in self.layers.iter().enumerate() {
            let w = layer.width as i32;
            let h = layer.height as i32;

            let background_offset =
                -(camera * vec2(offset_scales[i], 0.0)) % vec2(w as f32, h as f32);

            let offsets = [(0, 0), (w, 0), (0, h), (w, h)];
            for (offset_x, offset_y) in offsets {
                layer.draw_background(
                    screen,
                    background_offset.x as i32 + offset_x,
                    -116 as i32 + offset_y,
                    1.0,
                    1.0,
                    lerped_color_mask,
                    aura_low,
                    aura,
                    aura_transl,
                );
            }
        }
    }
}
