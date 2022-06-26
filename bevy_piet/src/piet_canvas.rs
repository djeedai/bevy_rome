use bevy::{
    ecs::component::Component,
    math::{Vec2, Vec3},
    render::{camera::CameraProjection, color::Color},
    sprite::Rect,
};

use crate::render_context::BevyRenderContext;

#[derive(Debug, Default, Clone, Copy)]
pub struct Quad {
    pub rect: Rect,
    pub color: Color,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl Quad {
    pub fn center(&self) -> Vec3 {
        let c = (self.rect.min + self.rect.max) * 0.5;
        Vec3::new(c.x, c.y, 0.)
    }
}

#[derive(Component, Debug, Default)]
pub struct PietCanvas {
    rect: Rect,
    quads: Vec<Quad>,
}

impl PietCanvas {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            ..Default::default()
        }
    }

    // pub fn from_projection<P: CameraProjection>(projection: &P) -> Self {
    //     let mat4 = projection.get_projection_matrix();
    //     let x = mat4.transform_point3a(Vec3A::X);
    //     let y = mat4.transform_point3a(Vec3A::Y);
    //     Self {
    //         rect: Rect {
    //             min: Vec2::new(ortho.left, ortho.top),
    //             max: Vec2::new(ortho.right, ortho.bottom),
    //         },
    //         ..Default::default()
    //     }
    // }

    pub fn rect(&self) -> Rect {
        self.rect
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
