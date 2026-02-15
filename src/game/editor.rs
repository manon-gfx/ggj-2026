use std::{collections::HashMap, path::PathBuf};

use super::{Aabb, MouseButton, draw_aabb_ws, level::Level};
use crate::{
    Bitmap,
    game::{
        InputState, Key,
        camera::{Camera, screen_to_world_space, world_space_to_screen_space},
        draw_aabb_ss,
        tilemap::TileSet,
    },
};
use glam::*;
use serde::{Deserialize, Serialize};
use zip::write::SimpleFileOptions;

#[derive(
    Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash,
)]
#[repr(u32)]
pub(crate) enum ObjectType {
    #[default]
    HedgehogWhite,
    HedgehogRed,
    HedgehogGreen,
    HedgehogBlue,
    Savepoint,
    MaskRed,
    MaskGreen,
    MaskBlue,
    MaskGold,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ObjectSpawn {
    pub(crate) position: Vec2,
    pub(crate) object_type: ObjectType,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(usize)]
pub enum EditorMode {
    #[default]
    TileMode,
    ObjectMode,
}

pub(crate) struct PlaceableObject {
    pub(crate) object_type: ObjectType,
    pub(crate) icon_bitmap: Bitmap,
    pub(crate) icon_scale: f32,

    pub(crate) aabb: Aabb,
}
impl PlaceableObject {
    fn new(object_type: ObjectType, icon_bitmap: Bitmap) -> Self {
        let aabb = Aabb {
            min: Vec2::ZERO,
            max: vec2(
                (icon_bitmap.width - 1) as f32,
                (icon_bitmap.height - 1) as f32,
            ),
        };

        Self {
            object_type,
            icon_bitmap,
            icon_scale: 1.0,
            aabb,
        }
    }
}

pub struct EditorState {
    pub(crate) editor_mode: EditorMode,

    // tile mode
    pub(crate) selected_tile: u32,

    // object mode
    pub(crate) selected_object: u32,
    pub(crate) placeable_objects: Vec<PlaceableObject>,
    pub(crate) placeable_objects_lut: HashMap<ObjectType, u32>,

    held_object: Option<usize>,
    pub loaded_level_path: PathBuf,
}

fn extract_sprite_from_sheet(sheet: &Bitmap, x: i32, y: i32, w: usize, h: usize) -> Bitmap {
    let mut icon = Bitmap::new(w, h);
    sheet.draw_on(&mut icon, -x, -y);
    icon
}

impl EditorState {
    pub fn new(
        enemy_sprite_sheet: &Bitmap,
        savepoint_bitmap: Bitmap,
        mask_red_bitmap: Bitmap,
        mask_green_bitmap: Bitmap,
        mask_blue_bitmap: Bitmap,
        mask_gold_bitmap: Bitmap,
    ) -> Self {
        let white_hedgehog_icon = extract_sprite_from_sheet(enemy_sprite_sheet, 0, 0, 16, 8);
        let red_hedgehog_icon = extract_sprite_from_sheet(enemy_sprite_sheet, 0, 8, 16, 8);
        let green_hedgehog_icon = extract_sprite_from_sheet(enemy_sprite_sheet, 0, 16, 16, 8);
        let blue_hedgehog_icon = extract_sprite_from_sheet(enemy_sprite_sheet, 0, 24, 16, 8);
        let placeable_objects = vec![
            PlaceableObject::new(ObjectType::HedgehogWhite, white_hedgehog_icon),
            PlaceableObject::new(ObjectType::HedgehogRed, red_hedgehog_icon),
            PlaceableObject::new(ObjectType::HedgehogGreen, green_hedgehog_icon),
            PlaceableObject::new(ObjectType::HedgehogBlue, blue_hedgehog_icon),
            PlaceableObject::new(ObjectType::Savepoint, savepoint_bitmap),
            PlaceableObject::new(ObjectType::MaskRed, mask_red_bitmap),
            PlaceableObject::new(ObjectType::MaskGreen, mask_green_bitmap),
            PlaceableObject::new(ObjectType::MaskBlue, mask_blue_bitmap),
            PlaceableObject::new(ObjectType::MaskGold, mask_gold_bitmap),
        ];

        let placeable_objects_lut = placeable_objects
            .iter()
            .enumerate()
            .map(|(i, entry)| (entry.object_type, i as u32))
            .collect();

        Self {
            editor_mode: Default::default(),
            selected_tile: Default::default(),
            selected_object: Default::default(),
            placeable_objects_lut,

            placeable_objects,
            held_object: None,
            loaded_level_path: "assets/levels/level0.zip".into(),
        }
    }

    pub fn tick(
        &mut self,
        delta_time: f32,
        screen: &mut Bitmap,
        level: &mut Level,
        tile_set: &TileSet,
        camera: &mut Camera,
        input_state: &InputState,
    ) {
        if input_state.is_key_down(Key::LeftCtrl) && input_state.is_key_pressed(Key::S) {
            let file_path = if input_state.is_key_down(Key::LeftShift) {
                let mut save_dir = std::env::current_dir().unwrap();
                save_dir.push("assets");
                save_dir.push("levels");
                rfd::FileDialog::new()
                    .add_filter("level", &["zip"])
                    .set_directory(save_dir)
                    .save_file()
            } else {
                Some(self.loaded_level_path.clone())
            };

            if let Some(file_path) = file_path {
                if let Some(parent) = file_path.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }

                let mut zip_writer =
                    zip::ZipWriter::new(std::fs::File::create(&file_path).unwrap());
                zip_writer
                    .start_file("tiles.csv", SimpleFileOptions::default())
                    .unwrap();
                level.tile_map.serialize(&mut zip_writer);
                zip_writer
                    .start_file("objects.json", SimpleFileOptions::default())
                    .unwrap();
                serde_json::to_writer(&mut zip_writer, &level.object_spawns).unwrap();
                drop(zip_writer);

                println!("Level Saved! {:?}", &file_path);
            }
        }
        if input_state.is_key_down(Key::LeftCtrl) && input_state.is_key_pressed(Key::O) {
            let mut save_dir = std::env::current_dir().unwrap();
            save_dir.push("assets");
            save_dir.push("levels");
            let file = rfd::FileDialog::new()
                .add_filter("level", &["zip"])
                .set_directory(save_dir)
                .pick_file();

            if let Some(file_path) = file {
                *level = Level::from_file(&file_path);
                println!("Level Loaded! {:?}", &file_path);
                self.loaded_level_path = file_path;
            }
        }

        if input_state.mouse_scroll_delta.y != 0.0 {
            let scroll_amount = (input_state.mouse_scroll_delta.y / 12.0).clamp(-1.0, 1.0);
            camera.zoom = (camera.zoom * 2.0f32.powf(scroll_amount)).clamp(0.125, 2.0);
        }

        if input_state.is_key_pressed(Key::EditorZoomIn) {
            camera.zoom = (camera.zoom * 2.0).min(2.0);
        }
        if input_state.is_key_pressed(Key::EditorZoomOut) {
            camera.zoom = (camera.zoom * 0.5).max(0.125);
        }

        if input_state.is_mouse_down(MouseButton::Middle) {
            camera.position -= input_state.mouse_delta / camera.zoom;
        }

        let editor_speed = 150.0 / camera.zoom;

        if input_state.is_key_down(Key::Left) {
            camera.position.x -= delta_time * editor_speed;
        }
        if input_state.is_key_down(Key::Right) {
            camera.position.x += delta_time * editor_speed;
        }
        if input_state.is_key_down(Key::Up) {
            camera.position.y -= delta_time * editor_speed;
        }
        if input_state.is_key_down(Key::Down) {
            camera.position.y += delta_time * editor_speed;
        }

        if input_state.is_key_pressed(Key::Key1) {
            self.editor_mode = EditorMode::TileMode;
        }
        if input_state.is_key_pressed(Key::Key2) {
            self.editor_mode = EditorMode::ObjectMode;
        }

        // Draw level bounds
        let aabb = Aabb {
            min: vec2(-1.0, -1.0),
            max: vec2(level.tile_map.width as f32, level.tile_map.height as f32)
                * (level.tile_map.tile_size as f32),
        };
        draw_aabb_ws(screen, &aabb, camera, 0x00ff00);

        match self.editor_mode {
            EditorMode::TileMode => {
                if input_state.is_key_pressed(Key::LeftBracket) && self.selected_tile > 0 {
                    self.selected_tile -= 1;
                }
                if input_state.is_key_pressed(Key::RightBracket)
                    && self.selected_tile < (tile_set.tiles.len() - 1) as u32
                {
                    self.selected_tile += 1;
                }

                if input_state.mouse.y < 192.0 {
                    if input_state.is_mouse_down(MouseButton::Left) {
                        let mouse_ws = screen_to_world_space(input_state.mouse, camera);
                        let mouse_ws = mouse_ws.as_uvec2();
                        let mouse_ts = mouse_ws / level.tile_map.tile_size;
                        if mouse_ts.x < level.tile_map.width && mouse_ts.y < level.tile_map.height {
                            level.tile_map.tiles
                                [(mouse_ts.x + mouse_ts.y * level.tile_map.width) as usize] =
                                self.selected_tile + 1;
                        }
                    }
                    if input_state.is_mouse_down(MouseButton::Right) {
                        let mouse_ws = screen_to_world_space(input_state.mouse, camera);
                        let mouse_ws = mouse_ws.as_uvec2();
                        let mouse_ts = mouse_ws / level.tile_map.tile_size;
                        if mouse_ts.x < level.tile_map.width && mouse_ts.y < level.tile_map.height {
                            level.tile_map.tiles
                                [(mouse_ts.x + mouse_ts.y * level.tile_map.width) as usize] = 0;
                        }
                    }
                }

                screen.draw_rectangle(0, 192, 255, 207, true, 0x0);
                screen.draw_rectangle(0, 192, 255, 207, false, 0xffffffff);

                for (i, tile) in tile_set.tiles.iter().take(24).enumerate() {
                    let aabb = Aabb {
                        min: vec2(3.0 + i as f32 * 10.0, 192.0 + 3.0),
                        max: vec2(12.0 + i as f32 * 10.0, 192.0 + 12.0),
                    };
                    if i == self.selected_tile as usize {
                        draw_aabb_ss(screen, &aabb, 0xffffff);
                    }

                    if input_state.is_mouse_down(MouseButton::Left)
                        && aabb.point_intersects(input_state.mouse)
                    {
                        self.selected_tile = i as u32;
                    }
                    tile.draw_on(screen, 4 + i as i32 * 10, 192 + 4);
                }
            }
            EditorMode::ObjectMode => {
                if input_state.is_key_pressed(Key::LeftBracket) && self.selected_object > 0 {
                    self.selected_object -= 1;
                }
                if input_state.is_key_pressed(Key::RightBracket)
                    && self.selected_object < (self.placeable_objects.len() - 1) as u32
                {
                    self.selected_object += 1;
                }

                // Draw object spawn list
                for object in level.object_spawns.iter() {
                    let index = self.placeable_objects_lut[&object.object_type];
                    let placeable_object = &self.placeable_objects[index as usize];
                    let position = world_space_to_screen_space(object.position, camera);

                    placeable_object.icon_bitmap.draw_on_scaled(
                        screen,
                        position.x as i32,
                        position.y as i32,
                        camera.zoom,
                        camera.zoom,
                    );
                }

                let selected_button = &self.placeable_objects[self.selected_object as usize];
                let mouse_pos_ws = screen_to_world_space(input_state.mouse, camera);
                let rounded_pos_ws = (mouse_pos_ws / 8.0).floor() * 8.0;

                if input_state.is_mouse_pressed(MouseButton::Right)
                    && input_state.mouse.y < 184.0
                    && let Some(index_to_remove) = level.object_spawns.iter().position(|object| {
                        self.placeable_objects
                            [self.placeable_objects_lut[&object.object_type] as usize]
                            .aabb
                            .translate(object.position)
                            .point_intersects(mouse_pos_ws)
                    })
                {
                    level.object_spawns.remove(index_to_remove);
                }

                if input_state.is_mouse_pressed(MouseButton::Left) {
                    if input_state.mouse.y < 184.0 {
                        self.held_object = level.object_spawns.iter().position(|object| {
                            self.placeable_objects
                                [self.placeable_objects_lut[&object.object_type] as usize]
                                .aabb
                                .translate(object.position)
                                .point_intersects(mouse_pos_ws)
                        });

                        if self.held_object.is_none() {
                            self.held_object = Some(level.object_spawns.len());

                            level.object_spawns.push(ObjectSpawn {
                                position: rounded_pos_ws,
                                object_type: selected_button.object_type,
                            })
                        }
                    }
                } else if input_state.is_mouse_down(MouseButton::Left)
                    && let Some(held_object) = self.held_object
                {
                    level.object_spawns[held_object].position = rounded_pos_ws;
                }

                if input_state.is_mouse_released(MouseButton::Left) {
                    self.held_object = None;
                }

                screen.draw_rectangle(0, 184, 255, 207, true, 0x0);
                screen.draw_rectangle(0, 184, 255, 207, false, 0xffffffff);
                for (i, button) in self.placeable_objects.iter().enumerate() {
                    let aabb = Aabb {
                        min: vec2(3.0 + i as f32 * 18.0, 184.0 + 3.0),
                        max: vec2(20.0 + i as f32 * 18.0, 184.0 + 20.0),
                    };

                    button.icon_bitmap.draw_on_scaled(
                        screen,
                        4 + i as i32 * 18 + (16 - (button.icon_bitmap.width.min(16) as i32)) / 2,
                        184 + 4 + (16 - (button.icon_bitmap.height.min(16) as i32)) / 2,
                        button.icon_scale,
                        button.icon_scale,
                    );

                    if i == self.selected_object as usize {
                        draw_aabb_ss(screen, &aabb, 0xffffff);
                    }

                    if input_state.is_mouse_down(MouseButton::Left)
                        && aabb.point_intersects(input_state.mouse)
                    {
                        self.selected_object = i as u32;
                    }
                }
            }
        }

        // Draw mouse cursor
        screen.plot(
            input_state.mouse.x as i32,
            input_state.mouse.y as i32,
            0xff00ff,
        );
    }
}
