use bevy::prelude::{Rect, Vec2};

use crate::{
    canvas::{QuarterPiePrimitive, RectPrimitive},
    render_context::Brush,
    Canvas,
};

/// Abstraction of a shape to draw on a [`Canvas`].
pub trait Shape {
    fn fill(&self, canvas: &mut Canvas, brush: &Brush);
    fn stroke(&self, canvas: &mut Canvas, brush: &Brush, thickness: f32);
}

impl Shape for Rect {
    fn fill(&self, canvas: &mut Canvas, brush: &Brush) {
        canvas.draw(RectPrimitive {
            rect: *self,
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
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
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
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

/// Rounded rectangle shape with separate radius in each direction.
pub struct RoundedRect {
    /// The rectangle itself.
    pub rect: Rect,
    /// The radii of the corners in the X and Y directions.
    pub radii: Vec2,
}

impl Shape for RoundedRect {
    fn fill(&self, canvas: &mut Canvas, brush: &Brush) {
        if self.radii.cmple(Vec2::ZERO).all() {
            //canvas.fill(...) // TODO
            return;
        }

        let half_size = self.rect.half_size();
        let radii = self.radii.min(half_size);

        // Top
        canvas.draw(RectPrimitive {
            rect: Rect::new(
                self.rect.min.x + radii.x,
                self.rect.max.y - radii.y,
                self.rect.max.x - radii.x,
                self.rect.max.y,
            ),
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
        });

        // Center (including left/right sides)
        canvas.draw(RectPrimitive {
            rect: Rect::new(
                self.rect.min.x,
                self.rect.min.y + radii.y,
                self.rect.max.x,
                self.rect.max.y - radii.y,
            ),
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
        });

        // Bottom
        canvas.draw(RectPrimitive {
            rect: Rect::new(
                self.rect.min.x + radii.x,
                self.rect.min.y,
                self.rect.max.x - radii.x,
                self.rect.min.y + radii.y,
            ),
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
        });

        // Top-left corner
        canvas.draw(QuarterPiePrimitive {
            origin: Vec2::new(self.rect.min.x + radii.x, self.rect.max.y - radii.y),
            radii,
            color: brush.color(),
            flip_x: true,
            flip_y: false,
        });

        // Top-right corner
        canvas.draw(QuarterPiePrimitive {
            origin: self.rect.max - radii,
            radii,
            color: brush.color(),
            flip_x: false,
            flip_y: false,
        });

        // Bottom-left corner
        canvas.draw(QuarterPiePrimitive {
            origin: self.rect.min + radii,
            radii,
            color: brush.color(),
            flip_x: true,
            flip_y: true,
        });

        // Bottom-right corner
        canvas.draw(QuarterPiePrimitive {
            origin: Vec2::new(self.rect.max.x - radii.x, self.rect.min.y + radii.y),
            radii,
            color: brush.color(),
            flip_x: false,
            flip_y: true,
        });
    }

    fn stroke(&self, canvas: &mut Canvas, brush: &Brush, thickness: f32) {
        let eps = thickness / 2.;
        let color = brush.color();
        let half_size = self.rect.half_size();
        let radii = self.radii.min(half_size);

        // Top
        let mut prim = RectPrimitive {
            rect: Rect {
                min: Vec2::new(self.rect.min.x + radii.x, self.rect.max.y - eps),
                max: Vec2::new(self.rect.max.x - radii.x, self.rect.max.y + eps),
            },
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
