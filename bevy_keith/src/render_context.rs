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
use crate::{CanvasTextId, Shape};

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

impl From<Color> for Brush {
    fn from(color: Color) -> Self {
        Self { color }
    }
}

impl From<&Color> for Brush {
    fn from(color: &Color) -> Self {
        Self { color: *color }
    }
}

impl Brush {
    pub fn color(&self) -> Color {
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
    pub(crate) alignment: JustifyText,
    /// Text bounds, used for glyph clipping.
    pub(crate) bounds: Vec2,
    /// Calculated text size based on glyphs alone, updated by
    /// [`process_glyphs()`].
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
            alignment: JustifyText::Left,
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
    alignment: JustifyText,
}

impl<'c> TextLayoutBuilder<'c> {
    fn new(canvas: &'c mut Canvas, storage: impl TextStorage) -> Self {
        Self {
            canvas,
            style: TextStyle::default(),
            value: storage.as_str().to_owned(),
            bounds: Vec2::new(f32::MAX, f32::MAX),
            anchor: Anchor::default(),
            alignment: JustifyText::Left, // Bottom,
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
    /// The text will be formatted with line wrapping and clipped to fit in
    /// those bounds.
    ///
    /// FIXME - Currently no clipping for partially visible glyphs, only
    /// completely outside ones are clipped.
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
    pub fn alignment(mut self, alignment: JustifyText) -> Self {
        self.alignment = alignment;
        self
    }

    /// Finalize the layout building and return the newly allocated text layout
    /// ID.
    ///
    /// FIXME - Return CanvasTextId somehow, to ensure texts are not used
    /// cross-Canvas.
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
//     fn new(width: usize, height: usize, buf: &[u8], format:
// piet::ImageFormat) -> Self {         let data = buf.to_vec();
//         let format = match format {
//             piet::ImageFormat::Grayscale =>
// bevy::render::render_resource::TextureFormat::R8Unorm,
// piet::ImageFormat::Rgb => unimplemented!(),
// piet::ImageFormat::RgbaSeparate => {
// bevy::render::render_resource::TextureFormat::Rgba8Unorm             }
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
            transform: Affine2::IDENTITY, // FIXME - unused
            canvas,
        }
    }

    /// Create a solid-color brush.
    pub fn solid_brush(&mut self, color: Color) -> Brush {
        color.into()
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
    pub fn fill(&mut self, shape: impl Shape, brush: &Brush) {
        shape.fill(self.canvas, brush);
    }

    /// Stroke a shape with a given brush.
    pub fn stroke(&mut self, shape: impl Shape, brush: &Brush, thickness: f32) {
        shape.stroke(self.canvas, brush, thickness);
    }

    /// Draw a line between two points with the given brush.
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
            radius: 0.,
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
