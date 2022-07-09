use std::mem::MaybeUninit;

use bevy::{
    ecs::{component::Component, reflect::ReflectComponent, system::Query},
    log::trace,
    math::{Vec2, Vec3},
    prelude::OrthographicProjection,
    reflect::Reflect,
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

#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Canvas {
    /// The canvas dimensions relative to its origin.
    rect: Rect,
    /// Optional background color to clear the canvas with.
    background_color: Option<Color>,
    /// Collection of primitives serialized as a float array, ready for shader consumption.
    #[reflect(ignore)]
    primitives: Vec<f32>,
    /// Collection of primitive indices serialized as an index array, ready for shader consumption.
    #[reflect(ignore)]
    indices: Vec<u32>,
}

impl Canvas {
    /// Create a new canvas with given dimensions.
    pub fn new(rect: Rect) -> Self {
        Self { rect, ..default() }
    }

    /// Create a new canvas with dimensions calculated to cover the area of an orthographic
    /// projection.
    pub fn from_ortho(ortho: &OrthographicProjection) -> Self {
        Self::new(Rect {
            min: Vec2::new(ortho.left, ortho.top),
            max: Vec2::new(ortho.right, ortho.bottom),
        })
    }

    /// Change the dimensions of the canvas.
    pub fn set_rect(&mut self, rect: Rect) {
        // if let Some(color) = self.background_color {
        //     if self.rect != rect {
        //         TODO - clear new area if any? or resize the clear() rect?!
        //     }
        // }
        self.rect = rect;
    }

    /// Get the dimensions of the canvas relative to its origin.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Change the background color of the canvas.
    ///
    /// This only has an effect starting from the next [`clear()`] call.
    pub fn set_background_color(&mut self, background_color: Option<Color>) {
        self.background_color = background_color;
    }

    /// Get the current background color of the canvas.
    pub fn background_color(&self) -> Option<Color> {
        self.background_color
    }

    /// Clear the canvas, discarding all primitives previously drawn on it.
    pub fn clear(&mut self) {
        self.primitives.clear();
        self.indices.clear();
        if let Some(color) = self.background_color {
            self.draw(RectPrim {
                rect: self.rect,
                color,
                ..default()
            });
        }
    }

    /// Draw a new primitive onto the canvas.
    ///
    /// This is a lower level entry point to canvas drawing; in general, you should
    /// prefer acquiring a [`RenderContext`] via [`Canvas::render_context()`] and
    /// using it to draw primitives.
    pub fn draw(&mut self, prim: impl Primitive) {
        let offset = self.primitives.len() as u32;

        // Allocate storage for primitives and indices
        let (prim_size, idx_size) = prim.sizes();
        self.primitives.reserve(prim_size);
        self.indices.reserve(idx_size);
        let prim_slice = self.primitives.spare_capacity_mut();
        let idx_slice = self.indices.spare_capacity_mut();

        // Write primitives and indices
        prim.write(
            &mut prim_slice[..prim_size],
            offset,
            &mut idx_slice[..idx_size],
        );

        // Apply new storage sizes
        let prim_size = self.primitives.len() + prim_size;
        unsafe { self.primitives.set_len(prim_size) };
        let idx_size = self.indices.len() + idx_size;
        unsafe { self.indices.set_len(idx_size) };
    }

    /// Acquire a new render context to draw on this canvas.
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

/// Update the dimensions of any [`Canvas`] component attached to the same entity as
/// as an [`OrthographicProjection`] component.
pub fn update_canvas_from_ortho_camera(mut query: Query<(&mut Canvas, &OrthographicProjection)>) {
    for (mut canvas, projection) in query.iter_mut() {
        let proj_rect = Rect {
            min: Vec2::new(projection.left, projection.bottom),
            max: Vec2::new(projection.right, projection.top),
        };
        trace!("ortho canvas rect = {:?}", proj_rect);
        canvas.set_rect(proj_rect);
    }
}
