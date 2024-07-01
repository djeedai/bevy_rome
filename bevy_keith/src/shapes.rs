//! Definition of the various shapes available to draw on a [`Canvas`].
//!
//! All the shapes implement the [`Shape`] trait.
//!
//! | Shape | Description |
//! |---|---|
//! | [`Rect`] | Axis-aligned rectangle. |
//! | [`RoundedRect`] | Axis-aligned rectangle with rounded corners. |

use bevy::{
    prelude::{Rect, Vec2},
    render::color::Color,
};

use crate::{
    canvas::{QuarterPiePrimitive, RectPrimitive},
    render_context::Brush,
    Canvas, Primitive,
};

/// Reference to a shape being built.
///
/// This is mainly used as the return type of some functions like
/// [`RenderContext::fill()`] to allow a builder-like pattern:
///
/// ```no_run
/// ctx.fill(rect, &brush).border(&border_brush, border_width);
/// ```
pub struct ShapeRef<'c> {
    pub(crate) prim: &'c mut Primitive,
}

/// Extension trait to tweak shapes built by the [`RenderContext`].
///
/// This is mainly used via [`ShapeRef`], which is returned of some functions
/// like [`RenderContext::fill()`] to allow a builder-like pattern:
///
/// ```no_run
/// ctx.fill(rect, &brush).border(&border_brush, border_width);
/// ```
pub trait ShapeExt {
    /// Add a border to the shape.
    fn border(&mut self, brush: &Brush, thickness: f32) -> &mut Self;

    /// Add a glow effect to the shape.
    fn glow(&mut self, brush: &Brush, spread: f32) -> &mut Self;
}

impl<'a> ShapeExt for ShapeRef<'a> {
    fn border(&mut self, brush: &Brush, thickness: f32) -> &mut Self {
        match self.prim {
            Primitive::Rect(r) => {
                r.border_color = brush.color();
                r.border_width = thickness.max(0.);
            }
            Primitive::Line(l) => todo!(),
            Primitive::Text(t) => todo!(),
            Primitive::QuarterPie(q) => todo!(),
        };
        self
    }

    fn glow(&mut self, _brush: &Brush, _spread: f32) -> &mut Self {
        todo!()
    }
}

/// Abstraction of a shape to draw on a [`Canvas`].
pub trait Shape {
    /// Fill the shape with the given [`Brush`].
    ///
    /// You can customize the shape with the [`ShapeExt`] trait functions.
    fn fill<'c>(&self, canvas: &'c mut Canvas, brush: &Brush) -> ShapeRef<'c>;

    /// Stroke the shape with the given [`Brush`] and thickness.
    ///
    /// This produces a stroke of the given thickness matching the underlying
    /// shape. The stroke is centered on the shape's edge. This is more
    /// efficient for some shapes (e.g. rectangle) when the overall shape is
    /// large, and you don't need to fill it.
    fn stroke<'c>(&self, canvas: &'c mut Canvas, brush: &Brush, thickness: f32) -> ShapeRef<'c>;
}

impl Shape for Rect {
    fn fill<'c>(&self, canvas: &'c mut Canvas, brush: &Brush) -> ShapeRef<'c> {
        canvas.draw(RectPrimitive {
            rect: *self,
            color: brush.color(),
            ..Default::default()
        })
    }

    fn stroke<'c>(&self, canvas: &'c mut Canvas, brush: &Brush, thickness: f32) -> ShapeRef<'c> {
        let eps = thickness / 2.;

        // Top (including corners)
        let mut prim = RectPrimitive {
            rect: Rect {
                min: Vec2::new(self.min.x - eps, self.max.y - eps),
                max: Vec2::new(self.max.x + eps, self.max.y + eps),
            },
            radius: 0.,
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
            border_width: 0.,
            border_color: Color::NONE,
        };
        canvas.draw(prim);

        // Bottom (including corners)
        prim.rect = Rect {
            min: Vec2::new(self.min.x - eps, self.min.y - eps),
            max: Vec2::new(self.max.x + eps, self.min.y + eps),
        };
        canvas.draw(prim);

        // Left (excluding corners)
        prim.rect = Rect {
            min: Vec2::new(self.min.x - eps, self.min.y + eps),
            max: Vec2::new(self.min.x + eps, self.max.y - eps),
        };
        canvas.draw(prim);

        // Right (excluding corners)
        prim.rect = Rect {
            min: Vec2::new(self.max.x - eps, self.min.y + eps),
            max: Vec2::new(self.max.x + eps, self.max.y - eps),
        };
        canvas.draw(prim)
    }
}

/// Rounded rectangle shape.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct RoundedRect {
    /// The rectangle itself, inclusive of the rounded corners.
    pub rect: Rect,
    /// The radius of the corners.
    pub radius: f32,
}

impl Shape for RoundedRect {
    fn fill<'c>(&self, canvas: &'c mut Canvas, brush: &Brush) -> ShapeRef<'c> {
        canvas.draw(RectPrimitive {
            rect: self.rect,
            radius: self.radius,
            color: brush.color(),
            ..Default::default()
        })
    }

    fn stroke<'c>(&self, canvas: &'c mut Canvas, brush: &Brush, thickness: f32) -> ShapeRef<'c> {
        let eps = thickness / 2.;
        let color = brush.color();
        let half_size = self.rect.half_size();
        let radii = Vec2::splat(self.radius).min(half_size);

        // Top
        let mut prim = RectPrimitive {
            rect: Rect {
                min: Vec2::new(self.rect.min.x + radii.x, self.rect.max.y - eps),
                max: Vec2::new(self.rect.max.x - radii.x, self.rect.max.y + eps),
            },
            radius: 0.,
            color,
            ..Default::default()
        };
        canvas.draw(prim);

        // Bottom
        prim.rect = Rect {
            min: Vec2::new(self.rect.min.x + radii.x, self.rect.min.y - eps),
            max: Vec2::new(self.rect.max.x - radii.x, self.rect.min.y + eps),
        };
        canvas.draw(prim);

        // Left
        prim.rect = Rect {
            min: Vec2::new(self.rect.min.x - eps, self.rect.min.y + radii.y),
            max: Vec2::new(self.rect.min.x + eps, self.rect.max.y - radii.y),
        };
        canvas.draw(prim);

        // Right (excluding corners)
        prim.rect = Rect {
            min: Vec2::new(self.rect.max.x - eps, self.rect.min.y + radii.y),
            max: Vec2::new(self.rect.max.x + eps, self.rect.max.y - radii.y),
        };
        canvas.draw(prim);

        // Top-left corner
        canvas.draw(QuarterPiePrimitive {
            origin: Vec2::new(self.rect.min.x + radii.x, self.rect.max.y - radii.y),
            radii,
            color,
            flip_x: true,
            flip_y: false,
        });

        // Top-right corner
        canvas.draw(QuarterPiePrimitive {
            origin: self.rect.max - radii,
            radii,
            color,
            flip_x: false,
            flip_y: false,
        });

        // Bottom-left corner
        canvas.draw(QuarterPiePrimitive {
            origin: self.rect.min + radii,
            radii,
            color,
            flip_x: true,
            flip_y: true,
        });

        // Bottom-right corner
        canvas.draw(QuarterPiePrimitive {
            origin: Vec2::new(self.rect.max.x - radii.x, self.rect.min.y + radii.y),
            radii,
            color,
            flip_x: false,
            flip_y: true,
        })
    }
}
