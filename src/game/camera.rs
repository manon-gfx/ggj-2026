use glam::*;

pub struct Camera {
    pub position: Vec2,
    pub zoom: f32,
}

pub fn screen_to_world_space(pos_on_screen: Vec2, camera: &Camera) -> Vec2 {
    pos_on_screen / camera.zoom + camera.position // / camera.zoom
}
pub fn world_space_to_screen_space(pos_in_world: Vec2, camera: &Camera) -> Vec2 {
    (pos_in_world - camera.position) * camera.zoom
}
