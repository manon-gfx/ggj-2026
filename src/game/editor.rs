use super::{Aabb, MouseButton, draw_aabb};

// use crate::bitmap::{self};
use crate::{
    Bitmap,
    game::{
        InputState, Key, screen_to_world_space,
        tilemap::{TileMap, TileSet, TileStruct},
    },
};
use glam::*;

pub struct EditorState {
    pub selected_tile: u32,
    pub selected_color: u32,
    pub selected_tile_type: u32,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            selected_tile: 0,
            selected_color: 0,
            selected_tile_type: 0,
        }
    }
}

impl EditorState {
    pub fn tick(
        &mut self,
        delta_time: f32,
        screen: &mut Bitmap,
        tile_map: &mut TileMap,
        tile_set: &TileSet,
        camera: &mut Vec2,
        input_state: &InputState,
    ) {
        if input_state.is_key_pressed(Key::SaveLevelEdit) {
            // tile_map.store_to_file("assets/level0.txt");
            tile_map.store_to_file("assets/level0.csv");
            println!("Level Saved!");
        }
        if input_state.is_key_pressed(Key::Jump) {
            let mouse_ws = screen_to_world_space(input_state.mouse, *camera);
            println!("Mouse position: {}", mouse_ws);
        }

        if input_state.is_key_down(Key::MoveLeft) {
            camera.x -= delta_time * 150.0;
        }
        if input_state.is_key_down(Key::MoveRight) {
            camera.x += delta_time * 150.0;
        }
        if input_state.is_key_down(Key::MoveUp) {
            camera.y -= delta_time * 150.0;
        }
        if input_state.is_key_down(Key::MoveDown) {
            camera.y += delta_time * 150.0;
        }

        if input_state.is_key_pressed(Key::SelectPrev) {
            if self.selected_tile > 0 {
                self.selected_tile -= 1;
            }
        }
        if input_state.is_key_pressed(Key::RightBracket) {
            if self.selected_tile < (tile_set.tiles.len() - 1) as u32 {
                self.selected_tile += 1;
            }
        }

        screen.plot(
            input_state.mouse.x as i32,
            input_state.mouse.y as i32,
            0xff00ff,
        );

        if input_state.mouse.y < 192.0 && input_state.mouse.x > 12.0 {
            if input_state.is_mouse_down(MouseButton::Left) {
                let mouse_ws = screen_to_world_space(input_state.mouse, *camera);
                let mouse_ws = mouse_ws.as_uvec2();
                let mouse_ts = mouse_ws / tile_map.tile_size;
                tile_map.tiles[(mouse_ts.x + mouse_ts.y * tile_map.width) as usize] =
                    self.selected_tile as i32;
            }
            if input_state.is_mouse_down(MouseButton::Right) {
                let mouse_ws = screen_to_world_space(input_state.mouse, *camera);
                let mouse_ws = mouse_ws.as_uvec2();
                let mouse_ts = mouse_ws / tile_map.tile_size;
                tile_map.tiles[(mouse_ts.x + mouse_ts.y * tile_map.width) as usize] = 0;
            }
        }

        let aabb = Aabb {
            min: vec2(-1.0, -1.0),
            max: vec2(tile_map.width as f32, tile_map.height as f32) * tile_map.tile_size as f32,
        };
        draw_aabb(screen, &aabb, *camera, 0x00ff00);

        screen.draw_rectangle(0, 192, 255, 207, true, 0x0);
        screen.draw_rectangle(0, 192, 255, 207, false, 0xffffffff);

        // screen.draw_rectangle(0, 0, 12, 192, true, 0x0);
        // screen.draw_rectangle(0, 0, 12, 192, false, 0xffffffff);

        // let num_unique_colors = tile_set.unique_tile_colors.len();
        // let num_unique_types = tile_set.unique_tile_types.len();

        for (i, color) in tile_set.unique_tile_colors.iter().enumerate() {
            // let x0 = 8 + i as i32 * 10;
            // let y0 = 180;
            let size = (8, 8);

            let x0 = 2;
            let y0 = 8 + i as i32 * 10;
            screen.draw_square(x0, y0, x0 + size.0, y0 + size.1, *color);
            let aabb = Aabb {
                min: vec2((x0 - 1) as f32, (y0 - 1) as f32),
                max: vec2((x0 + size.0) as f32, (y0 + size.1) as f32),
            };
            if i == self.selected_color as usize {
                draw_aabb(screen, &aabb, Vec2::ZERO, 0xffffff);
            }

            if input_state.is_mouse_pressed(MouseButton::Left)
                && aabb.point_intersects(input_state.mouse)
            {
                self.selected_color = i as u32;
                self.selected_tile = if tile_set
                    .tile_objs
                    .contains_key(&(tile_set.color_start[i] + self.selected_tile_type as usize))
                {
                    tile_set.color_start[i] as u32 + self.selected_tile_type
                } else {
                    tile_set.color_start[i] as u32
                };

                // println!("Selected color: {}, selected tile: {}",self.selected_color , self.selected_tile);
            }
        }

        let filtered_tiles = tile_set
            .tile_objs
            .iter()
            .filter(|&(k, v)| v.color == tile_set.unique_tile_colors[self.selected_color as usize]);

        for (j, (key, value)) in filtered_tiles.enumerate() {
            let x0 = 8 + j as i32 * 10;
            let y0 = 196;
            let size = (8, 8);
            // screen.draw_square(x0, y0, x0 + size.0, y0 + size.1, *color);
            let aabb = Aabb {
                min: vec2((x0 - 1) as f32, (y0 - 1) as f32),
                max: vec2((x0 + size.0) as f32, (y0 + size.1) as f32),
            };

            let relative_tile_type = *key - tile_set.color_start[self.selected_color as usize];
            if relative_tile_type == self.selected_tile_type as usize {
                draw_aabb(screen, &aabb, Vec2::ZERO, 0xffffff);
            }

            if input_state.is_mouse_pressed(MouseButton::Left)
                && aabb.point_intersects(input_state.mouse)
            {
                self.selected_tile_type = relative_tile_type as u32;
                self.selected_tile = *key as u32;
            }
            value.sprite.draw_on(screen, x0, y0);
        }

        // for (j, tile_type) in tile_set.unique_tile_types.iter().enumerate() {
        //     // only put show these colors to save space
        //     // if tile_set.tile_colors[i] == bitmap::BLACK
        //     //     || tile_set.tile_colors[i] == bitmap::RED
        //     //     || tile_set.tile_colors[i] == bitmap::BLUE
        //     //     || tile_set.tile_colors[i] == bitmap::GREEN
        //     //     || tile_set.tile_colors[i] == bitmap::GREY
        //     //     || tile_set.tile_colors[i] == bitmap::YELLOW
        //     // {

        //     let current_tile_num =
        //         (num_unique_colors as u32 * j as u32) as u32 + self.selected_color;
        //     // println!("Rendering tile num: {}, tile type: {}, selected color: {}, total num: {}", current_tile_num, j, self.selected_color, tile_set.tile_objs.len());

        //     // let tile_index = (num_unique_types * self.selected_color as usize + j);
        //     if tile_set
        //         .tile_objs
        //         .contains_key(&(current_tile_num as usize))
        //     {
        //         let x0 = 8 + j as i32 * 10;
        //         let y0 = 196;
        //         let size = (8, 8);
        //         // screen.draw_square(x0, y0, x0 + size.0, y0 + size.1, *color);
        //         let aabb = Aabb {
        //             min: vec2((x0 - 1) as f32, (y0 - 1) as f32),
        //             max: vec2((x0 + size.0) as f32, (y0 + size.1) as f32),
        //         };
        //         if j == self.selected_tile_type as usize {
        //             draw_aabb(screen, &aabb, Vec2::ZERO, 0xffffff);
        //         }

        //         if input_state.is_mouse_pressed(MouseButton::Left)
        //             && aabb.point_intersects(input_state.mouse)
        //         {
        //             self.selected_tile_type = j as u32;
        //             self.selected_tile = current_tile_num;
        //         }
        //         tile_set.tile_objs[&(current_tile_num as usize)]
        //             .sprite
        //             .draw_on(screen, x0, y0);
        //     } else {
        //         println!("No tile found for key (editor.rs): {:?}", current_tile_num);
        //     }

        // for (i, tile) in tile_set.tiles.iter().take(24).enumerate() {
        // let mut num_drawn_tiles = 0;

        // for (i, tile) in tile_set.tiles.iter().enumerate() {
        //     // only show these colors to save space
        //     if tile_set.tile_colors[i] == bitmap::BLACK
        //         || tile_set.tile_colors[i] == bitmap::RED
        //         || tile_set.tile_colors[i] == bitmap::BLUE
        //         || tile_set.tile_colors[i] == bitmap::GREEN
        //         || tile_set.tile_colors[i] == bitmap::GREY
        //         || tile_set.tile_colors[i] == bitmap::YELLOW
        //     {
        //         let aabb = Aabb {
        //             min: vec2(7.0 + num_drawn_tiles as f32 * 10.0, 192.0 + 3.0),
        //             max: vec2(16.0 + num_drawn_tiles as f32 * 10.0, 192.0 + 12.0),
        //         };
        //         if i == self.selected_tile as usize {
        //             draw_aabb(screen, &aabb, Vec2::ZERO, 0xffffff);
        //         }

        //         if input_state.is_mouse_pressed(MouseButton::Left)
        //             && aabb.point_intersects(input_state.mouse)
        //         {
        //             self.selected_tile = i as u32;
        //         }
        //         tile.draw_on(screen, 8 + num_drawn_tiles as i32 * 10, 192 + 4);
        //         num_drawn_tiles += 1;
        //     }
        // }
        // }
    }
}
