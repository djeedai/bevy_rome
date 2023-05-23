#![allow(unused_variables, dead_code, unused_imports)]

use bevy::math::Affine2;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::TextLayoutInfo;
use bevy::{
    math::{Rect, Vec2},
    render::render_resource::Buffer,
    text::Font,
    utils::{default, HashMap},
};
use std::{ops::RangeBounds, str, sync::Arc};

use crate::canvas::{
    Canvas, LinePrimitive, Primitive, QuarterPiePrimitive, RectPrimitive, TextPrimitive,
};
use crate::CanvasTextId;

#[derive(Debug, Clone)]
pub struct Brush {
    color: Color,
}

impl Default for Brush {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
        }
    }
}

impl Brush {
    fn color(&self) -> Color {
        self.color.clone()
    }
}

// impl<'c> IntoBrush<RenderContext<'c>> for Brush {
//     fn make_brush<'b>(
//         &'b self,
//         _piet: &mut RenderContext,
//         _bbox: impl FnOnce() -> KRect,
//     ) -> std::borrow::Cow<'b, Brush> {
//         std::borrow::Cow::Borrowed(self)
//     }
// }

pub trait TextStorage: 'static {
    fn as_str(&self) -> &str;
}

impl TextStorage for String {
    fn as_str(&self) -> &str {
        &self[..]
    }
}

impl TextStorage for &'static str {
    fn as_str(&self) -> &str {
        self
    }
}

// #[derive(Debug)]
// pub struct Text<'c> {
//     layouts: &'c Vec<TextLayout>,
// }

#[derive(Debug, Clone)]
pub struct TextLayout {
    /// Unique ID of the text into its owner [`Canvas`].
    pub(crate) id: u32,
    /// Sections of text.
    pub(crate) sections: Vec<TextSection>,
    /// Text anchor.
    pub(crate) anchor: Anchor,
    /// Text alignment relative to its origin (render position).
    pub(crate) alignment: TextAlignment,
    /// Text bounds, used for glyph clipping.
    pub(crate) bounds: Vec2,
    /// Calculated text size based on glyphs alone, updated by [`process_glyphs()`].
    pub(crate) calculated_size: Vec2,
    /// Layout info calculated by the [`KeithTextPipeline`].
    pub(crate) layout_info: Option<TextLayoutInfo>,
}

impl Default for TextLayout {
    fn default() -> Self {
        Self {
            id: 0,
            sections: vec![],
            anchor: Anchor::default(),
            alignment: TextAlignment::Left,
            bounds: Vec2::ZERO,
            calculated_size: Vec2::ZERO,
            layout_info: None,
        }
    }
}

pub struct TextLayoutBuilder<'c> {
    canvas: &'c mut Canvas,
    style: TextStyle,
    value: String,
    bounds: Vec2,
    anchor: Anchor,
    alignment: TextAlignment,
}

impl<'c> TextLayoutBuilder<'c> {
    fn new(canvas: &'c mut Canvas, storage: impl TextStorage) -> Self {
        Self {
            canvas,
            style: TextStyle::default(),
            value: storage.as_str().to_owned(),
            bounds: Vec2::new(f32::MAX, f32::MAX),
            anchor: Anchor::default(),
            alignment: TextAlignment::Left, //Bottom,
        }
    }

    /// Select the font to render the text with.
    pub fn font(mut self, font: Handle<Font>) -> Self {
        self.style.font = font;
        self
    }

    /// Set the font size.
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.style.font_size = font_size;
        self
    }

    /// Set the text color.
    ///
    /// FIXME - this vs. RenderContext::draw_text()'s color
    pub fn color(mut self, color: Color) -> Self {
        self.style.color = color;
        self
    }

    /// Set some bounds around the text.
    ///
    /// The text will be formatted with line wrapping and clipped to fit in those bounds.
    ///
    /// FIXME - Currently no clipping for partially visible glyphs, only completely outside
    /// ones are clipped.
    pub fn bounds(mut self, bounds: Vec2) -> Self {
        self.bounds = bounds;
        self
    }

    /// Set the text anchor point.
    pub fn anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    /// Set the text alignment relative to its render position.
    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Finalize the layout building and return the newly allocated text layout ID.
    ///
    /// FIXME - Return CanvasTextId somehow, to ensure texts are not used cross-Canvas.
    pub fn build(self) -> u32 {
        let layout = TextLayout {
            id: 0, // assigned in finish_layout()
            sections: vec![TextSection {
                style: self.style,
                value: self.value,
            }],
            anchor: self.anchor,
            alignment: self.alignment,
            bounds: self.bounds,
            calculated_size: Vec2::ZERO, // updated in process_glyphs()
            layout_info: None,
        };
        self.canvas.finish_layout(layout)
    }
}

#[derive(Debug, Default, Clone)]
pub struct BevyImage {
    image: bevy::render::texture::Image,
}

// impl BevyImage {
//     fn new(width: usize, height: usize, buf: &[u8], format: piet::ImageFormat) -> Self {
//         let data = buf.to_vec();
//         let format = match format {
//             piet::ImageFormat::Grayscale => bevy::render::render_resource::TextureFormat::R8Unorm,
//             piet::ImageFormat::Rgb => unimplemented!(),
//             piet::ImageFormat::RgbaSeparate => {
//                 bevy::render::render_resource::TextureFormat::Rgba8Unorm
//             }
//             piet::ImageFormat::RgbaPremul => unimplemented!(),
//             _ => unimplemented!(),
//         };
//         let image = bevy::render::texture::Image::new(
//             bevy::render::render_resource::Extent3d {
//                 width: width as u32,
//                 height: height as u32,
//                 depth_or_array_layers: 1,
//             },
//             bevy::render::render_resource::TextureDimension::D2,
//             data,
//             format,
//         );
//         Self { image }
//     }
// }

#[derive(Debug)]
pub struct RenderContext<'c> {
    /// Transform applied to all operations on this render context.
    transform: Affine2,
    /// Underlying canvas render operations are directed to.
    canvas: &'c mut Canvas,
}

impl<'c> RenderContext<'c> {
    /// Create a new render context to draw on an existing canvas.
    pub fn new(canvas: &'c mut Canvas) -> Self {
        Self {
            transform: Affine2::IDENTITY,
            canvas,
        }
    }

    /// Create a solid-color brush.
    pub fn solid_brush(&mut self, color: Color) -> Brush {
        Brush { color }
    }

    /// Clear an area of the render context with a specific color.
    ///
    /// To clear the entire underlying canvas, prefer using [`Canvas::clear()`].
    pub fn clear(&mut self, region: Option<Rect>, color: Color) {
        if let Some(rect) = region {
            // TODO - delete primitives covered by region
            self.fill(rect, &Brush { color });
        } else {
            self.canvas.clear();
            self.fill(self.canvas.rect(), &Brush { color });
        }
    }

    /// Fill a shape with a given brush.
    pub fn fill(&mut self, shape: Rect, brush: &Brush) {
        self.canvas.draw(RectPrimitive {
            rect: shape,
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
        });
    }

    /// Fill a shape with a given brush.
    pub fn rfill(&mut self, shape: Rect, radius: f32, brush: &Brush) {
        if radius <= 0. {
            self.fill(shape, brush);
            return;
        }

        let h = shape.half_size();
        let radius = radius.min(h.x).min(h.y);

        // Top
        self.canvas.draw(RectPrimitive {
            rect: Rect::new(
                shape.min.x + radius,
                shape.max.y - radius,
                shape.max.x - radius,
                shape.max.y,
            ),
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
        });

        // Center (including left/right sides)
        self.canvas.draw(RectPrimitive {
            rect: Rect::new(
                shape.min.x,
                shape.min.y + radius,
                shape.max.x,
                shape.max.y - radius,
            ),
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
        });

        // Bottom
        self.canvas.draw(RectPrimitive {
            rect: Rect::new(
                shape.min.x + radius,
                shape.min.y,
                shape.max.x - radius,
                shape.min.y + radius,
            ),
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
        });

        let radii = Vec2::splat(radius);

        // Top-left corner
        self.canvas.draw(QuarterPiePrimitive {
            origin: Vec2::new(shape.min.x + radius, shape.max.y - radius),
            radii,
            color: brush.color(),
            flip_x: true,
            flip_y: false,
        });

        // Top-right corner
        self.canvas.draw(QuarterPiePrimitive {
            origin: shape.max - radius,
            radii,
            color: brush.color(),
            flip_x: false,
            flip_y: false,
        });

        // Bottom-left corner
        self.canvas.draw(QuarterPiePrimitive {
            origin: shape.min + radius,
            radii,
            color: brush.color(),
            flip_x: true,
            flip_y: true,
        });

        // Bottom-right corner
        self.canvas.draw(QuarterPiePrimitive {
            origin: Vec2::new(shape.max.x - radius, shape.min.y + radius),
            radii,
            color: brush.color(),
            flip_x: false,
            flip_y: true,
        });
    }

    /// Stroke a shape with a given brush.
    pub fn stroke(&mut self, shape: Rect, brush: &Brush, thickness: f32) {
        let eps = thickness / 2.;

        // Top (including corners)
        let mut prim = RectPrimitive {
            rect: Rect {
                min: Vec2::new(shape.min.x - eps, shape.max.y - eps),
                max: Vec2::new(shape.max.x + eps, shape.max.y + eps),
            },
            color: brush.color(),
            flip_x: false,
            flip_y: false,
            image: None,
        };
        self.canvas.draw(prim);

        // Bottom (including corners)
        prim.rect = Rect {
            min: Vec2::new(shape.min.x - eps, shape.min.y - eps),
            max: Vec2::new(shape.max.x + eps, shape.min.y + eps),
        };
        self.canvas.draw(prim);

        // Left (excluding corners)
        prim.rect = Rect {
            min: Vec2::new(shape.min.x - eps, shape.min.y + eps),
            max: Vec2::new(shape.min.x + eps, shape.max.y - eps),
        };
        self.canvas.draw(prim);

        // Right (excluding corners)
        prim.rect = Rect {
            min: Vec2::new(shape.max.x - eps, shape.min.y + eps),
            max: Vec2::new(shape.max.x + eps, shape.max.y - eps),
        };
        self.canvas.draw(prim);
    }

    pub fn line(&mut self, p0: Vec2, p1: Vec2, brush: &Brush, thickness: f32) {
        self.canvas.draw(LinePrimitive {
            start: p0,
            end: p1,
            color: brush.color(),
            thickness,
        });
    }

    pub fn new_layout(&mut self, text: impl TextStorage) -> TextLayoutBuilder {
        TextLayoutBuilder::new(self.canvas, text)
    }

    pub fn draw_text(&mut self, text_id: u32, pos: Vec2) {
        self.canvas.draw(TextPrimitive {
            id: text_id,
            rect: Rect { min: pos, max: pos },
        });
    }

    pub fn draw_image(&mut self, shape: Rect, image: Handle<Image>) {
        self.canvas.draw(RectPrimitive {
            rect: shape,
            color: Color::WHITE,
            flip_x: false,
            flip_y: false,
            image: Some(image.id()),
        });
    }
}

impl<'c> Drop for RenderContext<'c> {
    fn drop(&mut self) {
        self.canvas.finish();
    }
}
