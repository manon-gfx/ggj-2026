use super::{Aabb, TileMap, TileSet, tilemap::TileFlags};
use crate::{
    bitmap::{self, Bitmap, GREEN},
    game::{sprite::Sprite, world_space_to_screen_space},
};
use glam::*;

#[derive(Debug)]
pub struct Enemy {
    pub position: Vec2,
    pub going_left: bool,
    pub visual_box: Aabb, // check map collision
    pub hitbox: Aabb,     // check player collision
    pub sprite: Sprite,
    pub color_mask: u32,
}

impl Enemy {
    pub fn new(
        position: Vec2,
        going_left: bool, /*, sprite: Sprite*/
        sprite: &Sprite,
        color_mask: u32,
    ) -> Self {
        Self {
            position,
            going_left,
            visual_box: Aabb {
                min: vec2(0.0, 0.0),
                max: vec2(7.0, 7.0),
            },
            hitbox: Aabb {
                min: vec2(1.0, 1.0),
                max: vec2(6.0, 6.0),
            },
            sprite: sprite.clone(),
            color_mask,
        }
    }

    pub fn is_colored(&self) -> bool {
        (self.color_mask & 0xffffff) != 0xffffff
    }

    pub fn tick(&mut self, delta_time: f32, tile_map: &TileMap, tile_set: &TileSet) {
        let speed = 50.0;
        let delta_x = if self.going_left {
            -speed * delta_time
        } else {
            speed * delta_time
        };

        let sample_points = if self.going_left {
            [
                self.position + vec2(self.visual_box.min.x + delta_x, self.visual_box.min.y),
                self.position + vec2(self.visual_box.min.x + delta_x, self.visual_box.max.y),
            ]
        } else {
            [
                self.position + vec2(self.visual_box.max.x + delta_x, self.visual_box.min.y),
                self.position + vec2(self.visual_box.max.x + delta_x, self.visual_box.max.y),
            ]
        };

        let flags = [
            tile_map.sample_tile_type_ws(
                sample_points[0],
                &tile_set.tile_types,
                &tile_set.tile_colors,
                self.color_mask,
            ),
            tile_map.sample_tile_type_ws(
                sample_points[1],
                &tile_set.tile_types,
                &tile_set.tile_colors,
                self.color_mask,
            ),
        ];

        if flags.iter().any(|flag| flag.contains(TileFlags::COLLISION)) {
            self.going_left = !self.going_left;
        } else {
            self.position.x += delta_x;
        }

        // self.sprite.seconds_per_frame = 1.0 / 4.0;
        self.sprite.tick(delta_time);
    }
    pub fn visual_aabb_world_space(&self) -> Aabb {
        Aabb {
            min: self.visual_box.min + self.position,
            max: self.visual_box.max + self.position,
        }
    }
    pub fn hitbox_aabb_world_space(&self) -> Aabb {
        Aabb {
            min: self.hitbox.min + self.position,
            max: self.hitbox.max + self.position,
        }
    }
    pub fn draw(
        &self,
        screen: &mut Bitmap,
        camera: Vec2,
        lerped_color_mask: u32,
        aura_low: &Bitmap,
        aura: &Bitmap,
    ) {
        let scale_x = if self.going_left { -1.0 } else { 1.0 };

        let position = world_space_to_screen_space(self.position, camera);
        let scale = vec2(scale_x, 1.0);
        let is_colored = self.color_mask != 0xffffff;

        self.sprite.draw_colored(
            screen,
            position,
            scale,
            is_colored,
            self.color_mask,
            lerped_color_mask,
            aura_low,
            aura,
        );
    }
}

pub fn spawn_enemies(
    sprite_white: &Sprite,
    sprite_red: &Sprite,
    sprite_green: &Sprite,
    sprite_blue: &Sprite,
) -> Vec<Enemy> {
    vec![Enemy::new(
        vec2(2263.0, 2176.0),
        true,
        sprite_green,
        bitmap::GREEN & 0xffffff,
    )]
}
