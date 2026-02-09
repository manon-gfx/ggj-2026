use super::{Aabb, MouseButton, draw_aabb_ws};
use crate::{
    Bitmap,
    game::{
        InputState, Key,
        camera::{Camera, screen_to_world_space, world_space_to_screen_space},
        draw_aabb_ss,
        tilemap::{TileMap, TileSet},
    },
};
use glam::*;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub(crate) enum ObjectType {
    #[default]
    WhiteHedgehog,
    RedHedgehog,
    GreenHedgehog,
    BlueHedgehog,
}

pub(crate) struct ObjectSpawn {
    pub(crate) position: Vec2,
    pub(crate) aabb: Aabb,
    pub(crate) object_type: ObjectType,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(usize)]
pub enum EditorMode {
    #[default]
    TileMode,
    ObjectMode,
}

pub(crate) struct ObjectButton {
    pub(crate) object_type: ObjectType,
    pub(crate) icon_bitmap: Bitmap,
    pub(crate) icon_scale: f32,
}

pub struct EditorState {
    pub(crate) editor_mode: EditorMode,

    // tile mode
    pub(crate) selected_tile: u32,

    // object mode
    pub(crate) selected_object: u32,
    pub(crate) object_spawns: Vec<ObjectSpawn>,
    pub(crate) object_buttons: Vec<ObjectButton>,

    held_object: Option<usize>,
}

fn extract_sprite_from_sheet(sheet: &Bitmap, x: i32, y: i32, w: usize, h: usize) -> Bitmap {
    let mut icon = Bitmap::new(w, h);
    sheet.draw_on(&mut icon, -x, -y);
    icon
}

impl EditorState {
    pub fn new(enemy_sprite_sheet: &Bitmap) -> Self {
        let white_hedgehog_icon = extract_sprite_from_sheet(enemy_sprite_sheet, 0, 0, 16, 8);
        let red_hedgehog_icon = extract_sprite_from_sheet(enemy_sprite_sheet, 0, 8, 16, 8);
        let green_hedgehog_icon = extract_sprite_from_sheet(enemy_sprite_sheet, 0, 16, 16, 8);
        let blue_hedgehog_icon = extract_sprite_from_sheet(enemy_sprite_sheet, 0, 24, 16, 8);

        let object_buttons = vec![
            ObjectButton {
                object_type: ObjectType::WhiteHedgehog,
                icon_bitmap: white_hedgehog_icon,
                icon_scale: 1.0,
            },
            ObjectButton {
                object_type: ObjectType::RedHedgehog,
                icon_bitmap: red_hedgehog_icon,
                icon_scale: 1.0,
            },
            ObjectButton {
                object_type: ObjectType::GreenHedgehog,
                icon_bitmap: green_hedgehog_icon,
                icon_scale: 1.0,
            },
            ObjectButton {
                object_type: ObjectType::BlueHedgehog,
                icon_bitmap: blue_hedgehog_icon,
                icon_scale: 1.0,
            },
        ];

        Self {
            editor_mode: Default::default(),
            selected_tile: Default::default(),
            selected_object: Default::default(),
            object_spawns: Default::default(),
            object_buttons,
            held_object: None,
        }
    }

    pub fn tick(
        &mut self,
        delta_time: f32,
        screen: &mut Bitmap,
        tile_map: &mut TileMap,
        tile_set: &TileSet,
        camera: &mut Camera,
        input_state: &InputState,
    ) {
        if input_state.is_key_pressed(Key::S) {
            tile_map.store_to_file("assets/level0.txt");
            println!("Level Saved!");
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
            max: vec2(tile_map.width as f32, tile_map.height as f32) * (tile_map.tile_size as f32),
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
                        let mouse_ts = mouse_ws / tile_map.tile_size;
                        if mouse_ts.x < tile_map.width && mouse_ts.y < tile_map.height {
                            tile_map.tiles[(mouse_ts.x + mouse_ts.y * tile_map.width) as usize] =
                                self.selected_tile + 1;
                        }
                    }
                    if input_state.is_mouse_down(MouseButton::Right) {
                        let mouse_ws = screen_to_world_space(input_state.mouse, camera);
                        let mouse_ws = mouse_ws.as_uvec2();
                        let mouse_ts = mouse_ws / tile_map.tile_size;
                        if mouse_ts.x < tile_map.width && mouse_ts.y < tile_map.height {
                            tile_map.tiles[(mouse_ts.x + mouse_ts.y * tile_map.width) as usize] = 0;
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
                    && self.selected_object < (self.object_buttons.len() - 1) as u32
                {
                    self.selected_object += 1;
                }

                // Draw object spawn list
                for object in self.object_spawns.iter() {
                    // TODO(manon): Linear search for every object *PUKE*
                    if let Some(button) = self
                        .object_buttons
                        .iter()
                        .find(|button| button.object_type == object.object_type)
                    {
                        let position = world_space_to_screen_space(object.position, camera);

                        button.icon_bitmap.draw_on_scaled(
                            screen,
                            position.x as i32,
                            position.y as i32,
                            camera.zoom,
                            camera.zoom,
                        );
                    }
                }

                let selected_button = &self.object_buttons[self.selected_object as usize];
                let mouse_pos_ws = screen_to_world_space(input_state.mouse, camera);
                let rounded_pos_ws = (mouse_pos_ws / 8.0).floor() * 8.0;

                if input_state.is_mouse_pressed(MouseButton::Right)
                    && input_state.mouse.y < 184.0
                    && let Some(index_to_remove) = self.object_spawns.iter().position(|object| {
                        object
                            .aabb
                            .translate(object.position)
                            .point_intersects(mouse_pos_ws)
                    })
                {
                    self.object_spawns.remove(index_to_remove);
                }

                if input_state.is_mouse_pressed(MouseButton::Left) {
                    if input_state.mouse.y < 184.0 {
                        self.held_object = self.object_spawns.iter().position(|object| {
                            object
                                .aabb
                                .translate(object.position)
                                .point_intersects(mouse_pos_ws)
                        });

                        if self.held_object.is_none() {
                            self.held_object = Some(self.object_spawns.len());

                            self.object_spawns.push(ObjectSpawn {
                                position: rounded_pos_ws,
                                aabb: Aabb {
                                    min: Vec2::ZERO,
                                    max: vec2(
                                        (selected_button.icon_bitmap.width - 1) as f32,
                                        (selected_button.icon_bitmap.height - 1) as f32,
                                    ),
                                },
                                object_type: selected_button.object_type,
                            })
                        }
                    }
                } else if input_state.is_mouse_down(MouseButton::Left)
                    && let Some(held_object) = self.held_object
                {
                    self.object_spawns[held_object].position = rounded_pos_ws;
                }

                if input_state.is_mouse_released(MouseButton::Left) {
                    self.held_object = None;
                }

                screen.draw_rectangle(0, 184, 255, 207, true, 0x0);
                screen.draw_rectangle(0, 184, 255, 207, false, 0xffffffff);
                for (i, button) in self.object_buttons.iter().enumerate() {
                    let aabb = Aabb {
                        min: vec2(3.0 + i as f32 * 18.0, 184.0 + 3.0),
                        max: vec2(20.0 + i as f32 * 18.0, 184.0 + 20.0),
                    };

                    button.icon_bitmap.draw_on_scaled(
                        screen,
                        4 + i as i32 * 18,
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
