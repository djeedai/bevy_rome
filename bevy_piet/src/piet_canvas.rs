use bevy::{ecs::component::Component, render::color::Color, sprite::Rect};

#[derive(Debug, Default, Clone, Copy)]
pub struct Quad {
    pub rect: Rect,
    pub color: Color,
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Component, Debug, Default, Clone)]
pub struct PietCanvas {
    quads: Vec<Quad>,
}

impl PietCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn quads(&self) -> &[Quad] {
        &self.quads[..]
    }

    pub fn quads_mut(&mut self) -> &mut [Quad] {
        &mut self.quads[..]
    }

    pub fn quads_vec(&mut self) -> &mut Vec<Quad> {
        &mut self.quads
    }
}