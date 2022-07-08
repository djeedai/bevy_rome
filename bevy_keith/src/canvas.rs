use std::mem::MaybeUninit;

use bevy::{
    ecs::component::Component,
    math::{Vec2, Vec3},
    render::{camera::CameraProjection, color::Color},
    sprite::Rect,
    utils::default,
};

use crate::render_context::RenderContext;

pub trait Primitive {
    fn sizes(&self) -> (usize, usize);
    fn write(&self, prim: &mut [MaybeUninit<f32>], offset: u32, idx: &mut [MaybeUninit<u32>]);
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LinePrim {
    pub start: Vec2,
    pub end: Vec2,
    pub color: Color,
    pub thickness: f32,
}

impl Primitive for LinePrim {
    fn sizes(&self) -> (usize, usize) {
        (6, 6)
    }

    fn write(&self, prim: &mut [MaybeUninit<f32>], offset: u32, idx: &mut [MaybeUninit<u32>]) {
        assert_eq!(6, prim.len());
        prim[0].write(self.start.x);
        prim[1].write(self.start.y);
        prim[2].write(self.end.x);
        prim[3].write(self.end.y);
        prim[4].write(bytemuck::cast(self.color.as_linear_rgba_u32()));
        prim[5].write(self.thickness);
        assert_eq!(6, idx.len());
        let prim_id = 1; // LINE
        for (i, corner) in [0, 2, 3, 0, 1, 2].iter().enumerate() {
            let index = offset | corner << 24 | prim_id << 26;
            idx[i].write(index);
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RectPrim {
    pub rect: Rect,
    pub color: Color,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl RectPrim {
    pub fn center(&self) -> Vec3 {
        let c = (self.rect.min + self.rect.max) * 0.5;
        Vec3::new(c.x, c.y, 0.)
    }
}

impl Primitive for RectPrim {
    fn sizes(&self) -> (usize, usize) {
        (5, 6)
    }

    fn write(&self, prim: &mut [MaybeUninit<f32>], offset: u32, idx: &mut [MaybeUninit<u32>]) {
        assert_eq!(5, prim.len());
        prim[0].write(self.rect.min.x);
        prim[1].write(self.rect.min.y);
        prim[2].write(self.rect.max.x - self.rect.min.x);
        prim[3].write(self.rect.max.y - self.rect.min.y);
        prim[4].write(bytemuck::cast(self.color.as_linear_rgba_u32()));
        assert_eq!(6, idx.len());
        let prim_id = 0; // RECT
        for (i, corner) in [0, 2, 3, 0, 3, 1].iter().enumerate() {
            let index = offset | corner << 24 | prim_id << 26;
            idx[i].write(index);
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct Canvas {
    rect: Rect,
    primitives: Vec<f32>,
    indices: Vec<u32>,
    background_color: Option<Color>,
}

impl Canvas {
    pub fn new(rect: Rect) -> Self {
        Self { rect, ..default() }
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

    pub fn set_rect(&mut self, rect: Rect) {
        if let Some(color) = self.background_color {
            //if self.rect != rect {
                // TODO - clear new area if any? or resize the clear() rect?!
            //}
        }
        self.rect = rect;
    }

    pub fn rect(&self) -> Rect {
        self.rect
    }

    pub fn clear(&mut self) {
        self.primitives.clear();
        self.indices.clear();
        if let Some(color) = self.background_color {
            self.push(RectPrim {
                rect: self.rect,
                color,
                ..default()
            });
        }
    }

    pub fn push(&mut self, prim: impl Primitive) {
        let offset = self.primitives.len() as u32;

        // Allocate storage for primitives and indices
        let (prim_size, idx_size) = prim.sizes();
        self.primitives.reserve(prim_size);
        self.indices.reserve(idx_size);
        let prim_slice = self.primitives.spare_capacity_mut();
        let idx_slice = self.indices.spare_capacity_mut();

        // Write primitives and indices
        prim.write(&mut prim_slice[..prim_size], offset, &mut idx_slice[..idx_size]);

        // Apply new storage sizes
        let prim_size = self.primitives.len() + prim_size;
        unsafe { self.primitives.set_len(prim_size) };
        let idx_size = self.indices.len() + idx_size;
        unsafe { self.indices.set_len(idx_size) };
    }

    pub fn render_context(&mut self) -> RenderContext {
        RenderContext::new(self)
    }

    pub(crate) fn finish(&mut self) {
        //
    }

    pub(crate) fn take_primitives_buffer(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.primitives)
    }

    pub(crate) fn take_indices_buffer(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.indices)
    }
}
