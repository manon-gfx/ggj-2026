#![allow(dead_code)]

pub mod font;
pub use font::Font;
pub use u32 as ColorChannel;

#[derive(Debug, Clone)]
pub enum BitmapData {
    Owned(Vec<u32>),
    Pointer(*mut u32, usize),
}
impl BitmapData {
    #[inline]
    fn pixels(&self) -> &[u32] {
        match self {
            Self::Owned(vec) => vec,
            Self::Pointer(ptr, size) => unsafe { std::slice::from_raw_parts(*ptr, *size) },
        }
    }
    #[inline]
    fn pixels_mut(&mut self) -> &mut [u32] {
        match self {
            Self::Owned(vec) => vec,
            Self::Pointer(ptr, size) => unsafe { std::slice::from_raw_parts_mut(*ptr, *size) },
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bitmap {
    pub width: usize,
    pub height: usize,
    pub stride: usize,
    pub pixels: BitmapData,
}

// ColorChannels
pub const RED: ColorChannel = 0xffff0000;
pub const GREEN: ColorChannel = 0xff00ff00;
pub const BLUE: ColorChannel = 0xff0000ff;
pub const WHITE: ColorChannel = RED | GREEN | BLUE;

impl Bitmap {
    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            stride: width,
            pixels: BitmapData::Owned(vec![0; width * height]),
        }
    }

    pub(crate) fn new_borrowed(
        pointer: *mut u32,
        width: usize,
        height: usize,
        stride: usize,
    ) -> Self {
        Self {
            width,
            height,
            stride,
            pixels: BitmapData::Pointer(pointer, height * stride),
        }
    }

    pub(crate) fn load(path: &str) -> Self {
        use stb_image::image;
        let image = match image::load_with_depth(path, 4, false) {
            image::LoadResult::ImageU8(img) => img,
            image::LoadResult::ImageF32(_) => panic!("f32 images are not supported"),
            image::LoadResult::Error(msg) => {
                panic!("Failed to load bitmap: \"{}\". Error: \"{}\"", path, msg);
            }
        };

        let pixels = image
            .data
            .chunks_exact(4)
            .map(|bytes| {
                let r = bytes[0] as u32;
                let g = bytes[1] as u32;
                let b = bytes[2] as u32;
                let a = bytes[3] as u32;

                (a << 24) | (r << 16) | (g << 8) | b
            })
            .collect::<Vec<u32>>();

        Self {
            width: image.width,
            height: image.height,
            stride: image.width,

            pixels: BitmapData::Owned(pixels),
        }
    }

    #[inline]
    pub fn pixels(&self) -> &[u32] {
        self.pixels.pixels()
    }
    #[inline]
    pub fn pixels_mut(&mut self) -> &mut [u32] {
        self.pixels.pixels_mut()
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels_mut().fill(color);
    }

    pub fn draw_on(&self, target: &mut Self, x: i32, y: i32, color_mask: ColorChannel) {
        let mut sw = self.width as i32;
        let mut sh = self.height as i32;

        let (sx, tx) = if x < 0 {
            sw += x;
            (x.abs(), 0)
        } else {
            (0, x)
        };
        let (sy, ty) = if y < 0 {
            sh += y;
            (y.abs(), 0)
        } else {
            (0, y)
        };

        sw = sw.min(target.width as i32 - tx);
        sh = sh.min(target.height as i32 - ty);

        for y in 0..sh {
            let line0 = (ty + y) * (target.stride as i32);
            let line1 = (sy + y) * (self.stride as i32);
            for x in 0..sw {
                unsafe {
                    let c = *self.pixels().get_unchecked((line1 + sx + x) as usize);
                    if (c & 0xff000000) != 0 {
                        // alpha
                        let masked_c = c & color_mask;
                        *target
                            .pixels_mut()
                            .get_unchecked_mut((line0 + tx + x) as usize) = masked_c;
                    }
                }
            }
        }
    }

    pub fn draw_square(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        let x0 = x0.clamp(0, self.width as i32 - 1);
        let x1 = x1.clamp(0, self.width as i32 - 1);
        let y0 = y0.clamp(0, self.height as i32 - 1);
        let y1 = y1.clamp(0, self.height as i32 - 1);

        let (x0, x1) = if x0 < x1 { (x0, x1) } else { (x1, x0) };
        let (y0, y1) = if y0 < y1 { (y0, y1) } else { (y1, y0) };

        let mut line = unsafe {
            self.pixels_mut()
                .as_mut_ptr()
                .add(y0 as usize * self.width + x0 as usize)
        };
        let w = x1 - x0;
        let h = y1 - y0;
        for _ in 0..h {
            for x in 0..w {
                unsafe {
                    *line.add(x as usize) = color;
                }
            }
            line = unsafe { line.add(self.width) };
        }
    }

    pub fn plot(&mut self, x: i32, y: i32, color: u32) {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let stride = self.stride;
            self.pixels_mut()[x as usize + y as usize * stride] = color;
        }
    }

    pub fn load_pixel(&self, x: i32, y: i32) -> u32 {
        if x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32 {
            let stride = self.stride;
            self.pixels()[x as usize + y as usize * stride]
        } else {
            0
        }
    }

    pub(crate) fn draw_line(&mut self, mut x0: f32, mut y0: f32, x1: f32, y1: f32, color: u32) {
        let dx = x1 - x0;
        let dy = y1 - y0;

        let l = dy.abs().max(dx.abs());
        let il = l as i32;
        let dx = dx / l;
        let dy = dy / l;
        for _ in 0..=il {
            self.plot(x0 as i32, y0 as i32, color);
            x0 += dx;
            y0 += dy;
        }
    }

    pub(crate) fn draw_rectangle(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        filled: bool,
        color: u32,
    ) {
        if filled {
            for x in x0..=x1 {
                for y in y0..=y1 {
                    self.plot(x, y, color);
                }
            }
        } else {
            self.draw_line(x0 as f32, y0 as f32, x1 as f32, y0 as f32, color);
            self.draw_line(x0 as f32, y1 as f32, x1 as f32, y1 as f32, color);
            self.draw_line(x0 as f32, y0 as f32, x0 as f32, y1 as f32, color);
            self.draw_line(x1 as f32, y0 as f32, x1 as f32, y1 as f32, color);
        }
    }

    pub fn draw_triangle(&mut self, v0: glam::Vec2, v1: glam::Vec2, v2: glam::Vec2, color: u32) {
        pub(crate) const SP_BITS: i32 = 4; //TODO(manon): Get this to 8 bits at some point
        pub(crate) const SP_MUL: i32 = 1 << SP_BITS;
        pub(crate) const SP_MULF: f32 = SP_MUL as f32;
        pub(crate) const SP_MASK: i32 = SP_MUL - 1;

        let vx = [
            (v0.x * SP_MULF) as i32,
            (v1.x * SP_MULF) as i32,
            (v2.x * SP_MULF) as i32,
        ];
        let vy = [
            (v0.y * SP_MULF) as i32,
            (v1.y * SP_MULF) as i32,
            (v2.y * SP_MULF) as i32,
        ];

        // 32 : 0
        let min_x = (vx[0].min(vx[1]).min(vx[2]) + SP_MASK) >> SP_BITS;
        let max_x = (vx[0].max(vx[1]).max(vx[2]) + SP_MASK) >> SP_BITS;
        let min_y = (vy[0].min(vy[1]).min(vy[2]) + SP_MASK) >> SP_BITS;
        let max_y = (vy[0].max(vy[1]).max(vy[2]) + SP_MASK) >> SP_BITS;

        // let tile_x = (tile_index_x * TILE_SIZE_X) as i32;
        // let tile_y = (tile_index_y * TILE_SIZE_Y) as i32;
        let min_xi = min_x.max(0);
        let max_xi = max_x.min(self.width as i32);
        let min_yi = min_y.max(0);
        let max_yi = max_y.min(self.height as i32);

        let dx01 = vx[0] - vx[1];
        let dx12 = vx[1] - vx[2];
        let dx20 = vx[2] - vx[0];

        let dy01 = vy[0] - vy[1];
        let dy12 = vy[1] - vy[2];
        let dy20 = vy[2] - vy[0];

        let fdx01 = dx01 << SP_BITS;
        let fdx12 = dx12 << SP_BITS;
        let fdx20 = dx20 << SP_BITS;

        let fdy01 = -dy01 << SP_BITS;
        let fdy12 = -dy12 << SP_BITS;
        let fdy20 = -dy20 << SP_BITS;

        let c0 = (dy01 * vx[0]) - (dx01 * vy[0]);
        let c1 = (dy12 * vx[1]) - (dx12 * vy[1]);
        let c2 = (dy20 * vx[2]) - (dx20 * vy[2]);

        // Apply top left rule
        let c0 = if !(dy01 < 0 || (dy01 == 0 && dx01 > 0)) {
            c0 - 1
        } else {
            c0
        };

        let c1 = if !(dy12 < 0 || (dy12 == 0 && dx12 > 0)) {
            c1 - 1
        } else {
            c1
        };

        let c2 = if !(dy20 < 0 || (dy20 == 0 && dx20 > 0)) {
            c2 - 1
        } else {
            c2
        };

        let mut w0_row = c0 + ((dx01 * min_yi - dy01 * min_xi) << SP_BITS);
        let mut w1_row = c1 + ((dx12 * min_yi - dy12 * min_xi) << SP_BITS);
        let mut w2_row = c2 + ((dx20 * min_yi - dy20 * min_xi) << SP_BITS);

        for y in min_yi..max_yi {
            let mut w0 = w0_row;
            let mut w1 = w1_row;
            let mut w2 = w2_row;

            for x in min_xi..max_xi {
                let w_all = w0 | w1 | w2;
                if w_all >= 0 {
                    let index = x as usize + y as usize * self.stride;
                    self.pixels_mut()[index] = color;
                }

                w0 += fdy01;
                w1 += fdy12;
                w2 += fdy20;
            }
            w0_row += fdx01;
            w1_row += fdx12;
            w2_row += fdx20;
        }
    }

    pub fn draw_str(&mut self, font: &Font, string: &str, mut x: i32, y: i32, color: u32) {
        for c in string.chars() {
            let ci = if c.is_ascii() {
                (c.to_ascii_lowercase() as usize) & 0xff
            } else if c == '\u{00b5}' || c == 'μ' {
                254
            } else {
                255
            };
            let trans = font.translation[ci];
            let character = &font.data[trans];

            for (v, row) in character.iter().enumerate() {
                for (u, &d) in row.iter().enumerate() {
                    if d {
                        self.plot(x + u as i32, y + v as i32, color);
                        self.plot(x + 1 + u as i32, y + 1 + v as i32, 0);
                    }
                }
            }

            x += 6;
        }
    }
}
