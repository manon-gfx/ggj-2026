#[allow(dead_code)]
use super::Aabb;
use crate::{
    bitmap::{self, Bitmap},
    game::camera::Camera,
};
use bitflags::bitflags;
use glam::*;
use std::collections::BTreeMap;

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct TileFlags: u32 {
        const NONE = 0x0;
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

#[derive(Debug)]
pub struct TileSet {
    pub tiles: Vec<Bitmap>,
    pub tile_colors: Vec<bitmap::ColorChannel>,
    pub tile_types: Vec<TileFlags>,
    pub tile_objs: BTreeMap<usize, TileStruct>,
    pub unique_tile_colors: Vec<bitmap::ColorChannel>,
    pub color_start: Vec<usize>,
    // pub aura: Bitmap,
    // pub aura_low: Bitmap,
    pub brightness_low: Vec<u32>,
    pub brightness: Vec<u32>,
}

#[derive(Debug)]
pub struct TileStruct {
    pub index: usize, // what number in the level files corresponds to this tile
    pub sprite: Bitmap,
    pub color: bitmap::ColorChannel,
    pub flags: TileFlags,
}
pub struct TileMap {
    pub tile_size: u32,

    pub width: u32,
    pub height: u32,
    pub tiles: Vec<i32>,
    pub unsupported_tile: TileStruct,
}

impl TileMap {
    pub fn from_file(path: &str) -> Self {
        // Read level file
        let level_layout_file =
            std::fs::read_to_string(path).expect("Could not load level file :(");
        let mut accumulator = String::new();
        let mut row_content: Vec<i32> = Vec::new();
        let mut layout: Vec<Vec<i32>> = Vec::new();
        let mut tile_count_x: u32 = 0;
        for char in level_layout_file.chars() {
            if char == ',' || char == '\n' {
                let tile_index: i32 = accumulator // i32 needed because I found a mapmaking tool that exports to csv, but it renders empty space as "-1" and the first tile as "0"
                    .parse::<i32>()
                    .unwrap_or_else(|_| {
                        panic!(
                            "Could not parse! :({}), line: {:?}, digit: {:?}",
                            &accumulator,
                            layout.len(),
                            row_content.len()
                        )
                    });
                row_content.push(tile_index as i32);
                accumulator = String::new();
                if char == '\n' {
                    layout.push(row_content.clone());
                    tile_count_x = tile_count_x.max(row_content.len() as u32);
                    row_content.clear();
                }
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
        let mut tile_indices: Vec<i32> = Vec::new();
        let tile_count_y: u32 = layout.len() as u32;
        for mut row in layout {
            assert!(row.len() <= tile_count_x as usize);
            row.resize(tile_count_x as usize, 0);
            tile_indices.append(&mut row);
        }

        let unsupported_tile = TileStruct {
            index: 0,
            sprite: Bitmap::load("assets/sprites/unsupported_tile.png"),
            color: bitmap::BLACK,
            flags: TileFlags::NONE,
        };

        Self {
            tile_size: 8,
            width: tile_count_x,
            height: tile_count_y,
            tiles: tile_indices,
            unsupported_tile,
        }
    }

    pub fn world_to_tile_index(&self, position: Vec2) -> IVec2 {
        (position / self.tile_size as f32).as_ivec2()
    }

    pub fn sample_world_pos(
        &self,
        position: Vec2,
        tile_objs: &BTreeMap<usize, TileStruct>,
        tile_flags: &Vec<TileFlags>,
        tile_colors: &Vec<bitmap::ColorChannel>,
        color_mask: &bitmap::ColorChannel,
    ) -> TileFlags {
        let color_mask = color_mask & 0xffffff;
        let tile_pos = self.world_to_tile_index(position);
        if tile_pos.x < 0
            || tile_pos.y < 0
            || tile_pos.x >= self.width as i32
            || tile_pos.y >= self.height as i32
        {
            TileFlags::NONE
        } else {
            let tile_index = self.tiles[(tile_pos.x + tile_pos.y * self.width as i32) as usize];
            //  If the tile is non-empty and non-white, and this color is masked out, treat as if there is no tile here

            if tile_index != -1 && tile_objs.contains_key(&((tile_index) as usize)) {
                let tile_flags = tile_objs[&((tile_index) as usize)].flags;
                if tile_flags.is_colored()
                    && tile_objs[&((tile_index) as usize)].color & color_mask == 0
                {
                    TileFlags::NONE
                } else {
                    tile_flags
                }
            } else {
                // if tile_index != 0 {
                //     // println!(
                //     //     "No tile found for key (sample world pos): {:?}",
                //     //     (tile_index - 1)
                //     // );
                // }
                TileFlags::NONE
            }
            //     if tile_index != 0 {
            //         if tile_objs
            //             .contains_key(&((tile_index - 1) as usize))
            //         {
            //             if tile_objs[&((tile_index - 1) as usize)]
            //                 .flags
            //                 .is_colored()
            //                 && tile_objs[&((tile_index - 1) as usize)].color & color_mask == 0
            //             {
            //                 0
            //             } else {
            //                 tile_index
            //             }
            //         } else {
            //             println!("No tile found for key: {:?}", (tile_index - 1));
            //             0
            //         }
            //     } else {
            //         tile_index
            //     }
        }
    }

    pub fn sample_tile_type_ws(
        &self,
        position: Vec2,
        tile_objs: &BTreeMap<usize, TileStruct>,
        tile_flags: &Vec<TileFlags>,
        tile_colors: &Vec<bitmap::ColorChannel>,
        color_mask: bitmap::ColorChannel,
    ) -> TileFlags {
        self.sample_world_pos(position, tile_objs, tile_flags, tile_colors, &color_mask)
    }

    pub fn editor_draw(&self, tile_set: &TileSet, target: &mut Bitmap, camera: &Camera) {
        let tile_size = self.tile_size as f32;
        let draw_tile_size = self.tile_size as f32 * camera.zoom;

        let mut draw_tiles_max_x = (target.width as f32 / draw_tile_size).ceil() as i32 + 1;
        let mut draw_tiles_max_y = (target.height as f32 / draw_tile_size).ceil() as i32 + 1;

        let start_tile_x = (camera.position.x / tile_size) as i32;
        let start_tile_y = (camera.position.y / tile_size) as i32;

        let camera_offset = (camera.position % tile_size) * camera.zoom;

        let mut draw_tiles_min_x = 0;
        let mut draw_tiles_min_y = 0;

        if start_tile_x < 0 {
            draw_tiles_min_x -= start_tile_x;
        }
        if start_tile_y < 0 {
            draw_tiles_min_y -= start_tile_y;
        }

        if start_tile_x + draw_tiles_max_x >= self.width as i32 {
            draw_tiles_max_x = self.width as i32 - start_tile_x;
        }
        if start_tile_y + draw_tiles_max_y >= self.height as i32 {
            draw_tiles_max_y = self.height as i32 - start_tile_y;
        }

        for y in draw_tiles_min_y..draw_tiles_max_y {
            // NOTE(manon): Flooring before int-cast is necessary to properly handle negative numbers
            let draw_y = (draw_tile_size * y as f32 - camera_offset.y).floor() as i32;
            for x in draw_tiles_min_x..draw_tiles_max_x {
                let draw_x = (draw_tile_size * x as f32 - camera_offset.x).floor() as i32;

                let tile_y = (start_tile_y + y) as u32;
                let tile_x = (start_tile_x + x) as u32;

                let tile_id = self.tiles[(tile_y * self.width + tile_x) as usize];
                // skip empty space (stored as -1 in the csv by the Tiled tile editor)
                if tile_id != -1 {
                    let tile_bmp = if tile_set.tile_objs.contains_key(&((tile_id) as usize)) {
                        &tile_set.tile_objs[&((tile_id) as usize)].sprite
                    } else {
                        // println!("No tile found for key (draw): {:?}", (tile_index - 1));
                        &self.unsupported_tile.sprite
                    };
                    tile_bmp.draw_on_scaled(target, draw_x, draw_y, camera.zoom, camera.zoom);
                }
            }
        }
    }

    pub fn draw(
        &self,
        tile_set: &TileSet,
        target: &mut Bitmap,
        camera: Vec2,
        color_mask: crate::bitmap::ColorChannel,
        lerped_color_mask: u32,
        aura_transl: IVec2,
        editor_mode: bool,
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

        let mut mask_color = TileFlags::NONE;
        // let mut color_mask_alpha = 0xff;
        if ((color_mask >> 16) & 0xff) > 0 {
            mask_color = TileFlags::RED;
            // color_mask_alpha = (lerped_color_mask >> 16) & 0xff;
        } else if ((color_mask >> 8) & 0xff) > 0 {
            mask_color = TileFlags::GREEN;
            // color_mask_alpha = (lerped_color_mask >> 8) & 0xff;
        } else if (color_mask & 0xff) > 0 {
            mask_color = TileFlags::BLUE;
            // color_mask_alpha = (lerped_color_mask) & 0xff;
        };

        for y in 0..tile_count_y {
            for x in 0..tile_count_x {
                let tx = (tile_min_x + x) as u32;
                let ty = (tile_min_y + y) as u32;

                let sx = x as i32 * self.tile_size as i32;
                let sy = y as i32 * self.tile_size as i32;

                let tile_index = self.tiles[(ty * self.width + tx) as usize];
                // skip empty space
                if tile_index == -1 {
                    continue;
                }

                let tile_obj = if tile_set.tile_objs.contains_key(&((tile_index) as usize)) {
                    &tile_set.tile_objs[&((tile_index) as usize)]
                } else {
                    // println!("No tile found for key (draw): {:?}", (tile_index - 1));
                    &self.unsupported_tile
                };
                // match tile_set.tile_objs.get((tile_index - 1) as usize) {
                //     Some(x) => x,
                //     None => &TileStruct{index: 0, sprite: self.unsupported_tile, color: bitmap::BLACK, flags: TileFlags::NONE}
                // };

                // let tile_obj = &tile_set.tile_objs[(tile_index - 1) as usize];

                if editor_mode {
                    tile_obj.sprite.draw_on(
                        target,
                        sx - camera.x as i32 + tile_min_x as i32 * self.tile_size as i32,
                        sy - camera.y as i32 + tile_min_y as i32 * self.tile_size as i32,
                    );
                } else {
                    // leave white tiles white
                    // let color = tile_set.tile_colors[(tile_index - 1) as usize];
                    let tile_type: &TileFlags = &tile_obj.flags;

                    let is_colored = tile_type.intersects(TileFlags::WHITE);
                    let is_tile_shown =
                        !tile_type.intersects(TileFlags::WHITE) || tile_type.intersects(mask_color);
                    if is_tile_shown {
                        tile_obj.sprite.draw_tile_different_colors(
                            target,
                            sx - camera.x as i32 + tile_min_x as i32 * self.tile_size as i32,
                            sy - camera.y as i32 + tile_min_y as i32 * self.tile_size as i32,
                            is_colored,
                            mask_color,
                            tile_type,
                            lerped_color_mask,
                            // &tile_set.aura_low,
                            // &tile_set.aura,
                            &tile_set.brightness_low,
                            &tile_set.brightness,
                            aura_transl,
                        );
                    }
                }

                //     let tile = &tile_set.tiles[(tile_index - 1) as usize];
                //     if editor_mode {
                //         tile.draw_on(
                //             target,
                //             sx - camera.x as i32 + tile_min_x as i32 * self.tile_size as i32,
                //             sy - camera.y as i32 + tile_min_y as i32 * self.tile_size as i32,
                //         );
                //     } else {
                //         // leave white tiles white
                //         // let color = tile_set.tile_colors[(tile_index - 1) as usize];
                //         let tile_type: &TileFlags = &tile_set.tile_types[(tile_index - 1) as usize];

                //         // let is_colored = tile_type.intersects(TileFlags::WHITE);
                //         // tile.draw_tile(
                //         //     target,
                //         //     sx - camera.x as i32 + tile_min_x as i32 * self.tile_size as i32,
                //         //     sy - camera.y as i32 + tile_min_y as i32 * self.tile_size as i32,
                //         //     is_colored,
                //         //     color,
                //         //     lerped_color_mask,
                //         //     &tile_set.aura_low,
                //         //     &tile_set.aura,
                //         //     aura_transl,
                //         // );
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
