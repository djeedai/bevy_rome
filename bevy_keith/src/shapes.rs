//! Definition of the various shapes available to draw on a [`Canvas`].
//!
//! All the shapes implement the [`Shape`] trait.
//!
//! | Shape | Description |
//! |---|---|
//! | [`Rect`] | Axis-aligned rectangle. |
//! | [`RoundedRect`] | Axis-aligned rectangle with rounded corners. |

use bevy::{prelude::{Rect, Vec2}, render::color::Color};

use crate::{
    canvas::{QuarterPiePrimitive, RectPrimitive},
    render_context::Brush,
    Canvas,
};

/// Abstraction of a shape to draw on a [`Canvas`].
pub trait Shape {
    /// Fill the shape with the given [`Brush`].
    fn fill(&self, canvas: &mut Canvas, brush: &Brush);

    /// Stroke the shape with the given [`Brush`] and thickness.
    ///
    /// This produces a stroke of the given thickness matching the underlying
    /// shape. The stroke is centered on the shape's edge.
    fn stroke(&self, canvas: &mut Canvas, brush: &Brush, thickness: f32);
}

impl Shape for Rect {
    fn fill(&self, canvas: &mut Canvas, brush: &Brush) {
        canvas.draw(RectPrimitive {
            rect: *self,
            radius: 0.,
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
            border_width: 0.,
            border_color: Color::NONE,
        });
    }

    fn stroke(&self, canvas: &mut Canvas, brush: &Brush, thickness: f32) {
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
        canvas.draw(prim);
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
    fn fill(&self, canvas: &mut Canvas, brush: &Brush) {
        canvas.draw(RectPrimitive {
            rect: self.rect,
            radius: self.radius,
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
        });
    }

    fn stroke(&self, canvas: &mut Canvas, brush: &Brush, thickness: f32) {
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
            flip_x: false,
            flip_y: false,
            image: None,
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
        });
    }
}
