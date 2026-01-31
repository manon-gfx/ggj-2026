use super::Aabb;
use crate::bitmap::{self, Bitmap};
use bitflags::bitflags;
use glam::*;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct TileFlags: u32 {
        const COLLISION = 0x1;
        const SPIKE = 0x2;

        const RED = 0x4;
        const GREEN = 0x8;
        const BLUE = 0x10;
        const WHITE = Self::RED.bits() | Self::GREEN.bits() | Self::BLUE.bits();
    }
}

impl TileFlags {
    fn is_colored(&self) -> bool {
        self.intersects(Self::WHITE)
    }
}

pub struct TileSet {
    pub tiles: Vec<Bitmap>,
    pub tile_colors: Vec<bitmap::ColorChannel>,
    pub tile_types: Vec<TileFlags>,
    pub aura: Bitmap,
    pub aura_low: Bitmap,
}

pub struct TileMap {
    pub tile_size: u32,

    pub width: u32,
    pub height: u32,
    pub tiles: Vec<u32>,
}

impl TileMap {
    pub fn from_file(path: &str) -> Self {
        // Read level file
        let level_layout_file =
            std::fs::read_to_string(path).expect("Could not load level file :(");
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

        Self {
            tile_size: 8,
            width: tile_count_x,
            height: tile_count_y,
            tiles: tile_indices,
        }
    }

    pub fn world_to_tile_index(&self, position: Vec2) -> IVec2 {
        (position / self.tile_size as f32).as_ivec2()
    }
    pub fn tile_index_to_world_coord(&self, tile_index: IVec2) -> Vec2 {
        (tile_index * self.tile_size as i32).as_vec2()
    }
    pub fn round_world_coord_to_tile(&self, position: Vec2) -> Vec2 {
        (position / self.tile_size as f32).floor() * self.tile_size as f32
    }

    pub fn sample_world_pos(
        &self,
        position: Vec2,
        tile_flags: &Vec<TileFlags>,
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
            if tile_index != 0 {
                if tile_flags[(tile_index - 1) as usize].is_colored()
                    && tile_colors[(tile_index - 1) as usize] & color_mask == 0
                {
                    0
                } else {
                    tile_index
                }
            } else {
                tile_index
            }
        }
    }

    pub fn sample_tile_type_ws(
        &self,
        position: Vec2,
        tile_flags: &Vec<TileFlags>,
        tile_colors: &Vec<bitmap::ColorChannel>,
        color_mask: &bitmap::ColorChannel,
    ) -> TileFlags {
        let tile_index = self.sample_world_pos(position, tile_flags, tile_colors, &color_mask);
        if tile_index == 0 {
            TileFlags::empty()
        } else {
            tile_flags[(tile_index - 1) as usize]
        }
    }

    pub fn draw(
        &self,
        tile_set: &TileSet,
        target: &mut Bitmap,
        camera: Vec2,
        color_mask: crate::bitmap::ColorChannel,
        lerped_color_mask: u32,
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
                // skip empty space
                if tile_index == 0 {
                    continue;
                }

                // leave white tiles white
                let tile = &tile_set.tiles[(tile_index - 1) as usize];
                let color = tile_set.tile_colors[(tile_index - 1) as usize];
                let tile_type = &tile_set.tile_types[(tile_index - 1) as usize];

                let is_colored = tile_type.intersects(TileFlags::WHITE);

                tile.draw_tile(
                    target,
                    sx - camera.x as i32 + tile_min_x as i32 * self.tile_size as i32,
                    sy - camera.y as i32 + tile_min_y as i32 * self.tile_size as i32,
                    is_colored,
                    color,
                    lerped_color_mask,
                    &tile_set.aura_low,
                    &tile_set.aura,
                );
            }
        }
    }

    pub fn store_to_file(&self, path: &str) {
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
