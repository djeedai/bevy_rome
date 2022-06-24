use bevy::{ecs::component::Component, render::color::Color, sprite::Rect};

use crate::render_context::BevyRenderContext;

#[derive(Debug, Default, Clone, Copy)]
pub struct Quad {
    pub rect: Rect,
    pub color: Color,
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Component, Debug, Default)]
pub struct PietCanvas {
    quads: Vec<Quad>,
}

impl PietCanvas {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.quads.clear();
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

    pub fn render_context(&mut self) -> BevyRenderContext {
        BevyRenderContext::new(self)
    }

    pub(crate) fn finish(&mut self) {
        //
    }
}
