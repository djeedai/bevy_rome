#![allow(unused_variables, dead_code, unused_imports)]

use bevy::prelude::*;
use bevy::{
    math::Vec2,
    render::render_resource::Buffer,
    sprite::Rect,
    text::Font,
    utils::{default, HashMap},
};
use std::{ops::RangeBounds, str, sync::Arc};

use crate::canvas::{Canvas, LinePrimitive, Primitive, RectPrimitive, TextPrimitive};
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

// impl<'c> Text<'c> {
// }

// impl piet::Text for Text {
//     type TextLayoutBuilder = TextLayoutBuilder;
//     type TextLayout = TextLayout;
//     fn font_family(&mut self, family_name: &str) -> Option<piet::FontFamily> {
//         unimplemented!()
//     }
//     fn load_font(&mut self, data: &[u8]) -> Result<piet::FontFamily, piet::Error> {
//         unimplemented!()
//     }
//     fn new_text_layout(&mut self, text: impl piet::TextStorage) -> Self::TextLayoutBuilder {
//         unimplemented!()
//     }
// }

#[derive(Debug, Default, Clone)]
pub struct TextLayout {
    /// Unique ID of the text into its owner [`Canvas`].
    pub(crate) id: u32,
    /// Sections of text.
    pub(crate) sections: Vec<TextSection>,
    /// Text alignment relative to its origin (render position).
    pub(crate) alignment: TextAlignment,
    /// Text bounds, used for glyph clipping.
    pub(crate) bounds: Vec2,
    /// Calculated text size based on glyphs alone, updated by [`process_glyphs()`].
    pub(crate) calculated_size: Vec2,
}

// impl piet::TextLayout for TextLayout {
//     fn size(&self) -> Size {
//         unimplemented!()
//     }
//     fn trailing_whitespace_width(&self) -> f64 {
//         unimplemented!()
//     }
//     fn image_bounds(&self) -> KRect {
//         unimplemented!()
//     }
//     fn text(&self) -> &str {
//         unimplemented!()
//     }
//     fn line_text(&self, line_number: usize) -> Option<&str> {
//         unimplemented!()
//     }
//     fn line_metric(&self, line_number: usize) -> Option<LineMetric> {
//         unimplemented!()
//     }
//     fn line_count(&self) -> usize {
//         unimplemented!()
//     }
//     fn hit_test_point(&self, point: Point) -> HitTestPoint {
//         unimplemented!()
//     }
//     fn hit_test_text_position(&self, idx: usize) -> HitTestPosition {
//         unimplemented!()
//     }
//     fn rects_for_range(&self, range: impl RangeBounds<usize>) -> Vec<KRect> {
//         unimplemented!()
//     }
// }

pub struct TextLayoutBuilder<'c> {
    canvas: &'c mut Canvas,
    style: TextStyle,
    value: String,
    bounds: Vec2,
    alignment: TextAlignment,
}

impl<'c> TextLayoutBuilder<'c> {
    fn new(canvas: &'c mut Canvas, storage: impl TextStorage) -> Self {
        Self {
            canvas,
            style: TextStyle::default(),
            value: storage.as_str().to_owned(),
            bounds: Vec2::new(f32::MAX, f32::MAX),
            alignment: TextAlignment {
                vertical: VerticalAlign::Bottom,
                horizontal: HorizontalAlign::Left,
            },
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

    /// Set the text alignment relative to its render position.
    pub fn alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Finalize the layout building and return the newly allocated text layout ID.
    /// 
    /// FIXME - Return CanvasTextId somehow, to ensure texts are not used cross-Canvas.
    pub fn build(mut self) -> u32 {
        let layout = TextLayout {
            id: 0, // assigned in finish_layout()
            sections: vec![TextSection {
                style: self.style,
                value: self.value,
            }],
            alignment: self.alignment,
            bounds: self.bounds,
            calculated_size: Vec2::ZERO, // updated in process_glyphs()
        };
        self.canvas.finish_layout(layout)
    }
}

// impl piet::TextLayoutBuilder for TextLayoutBuilder {
//     type Out = TextLayout;
//     fn max_width(self, width: f64) -> Self {
//         unimplemented!()
//     }
//     fn alignment(self, alignment: piet::TextAlignment) -> Self {
//         unimplemented!()
//     }
//     fn default_attribute(self, attribute: impl Into<piet::TextAttribute>) -> Self {
//         unimplemented!()
//     }
//     fn range_attribute(
//         self,
//         range: impl RangeBounds<usize>,
//         attribute: impl Into<piet::TextAttribute>,
//     ) -> Self {
//         unimplemented!()
//     }
//     fn build(self) -> Result<Self::Out, piet::Error> {
//         unimplemented!()
//     }
//     fn font(self, font: piet::FontFamily, font_size: f64) -> Self {
//         unimplemented!()
//     }
//     fn text_color(self, color: Color) -> Self {
//         unimplemented!()
//     }
// }

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

// impl piet::Image for BevyImage {
//     fn size(&self) -> Size {
//         let size = self.image.size().to_array();
//         Size::new(size[0] as f64, size[1] as f64)
//     }
// }

#[derive(Debug)]
pub struct RenderContext<'c> {
    transform: Transform, // TODO -- 2D affine transform only...
    canvas: &'c mut Canvas,
}

impl<'c> RenderContext<'c> {
    /// Create a new render context to draw on an existing canvas.
    pub fn new(canvas: &'c mut Canvas) -> Self {
        Self {
            transform: Transform::identity(),
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
            color: Color::RED, // FIXME - text.sections[0].style.color, // TODO - multi-section?
        });
    }

    pub fn draw_image(&mut self, shape: Rect, image: Handle<Image>) {
        self.canvas.draw(RectPrimitive {
            rect: shape,
            color: Color::WHITE,
            flip_x: false,
            flip_y: false,
            image: Some(image.id),
        });
    }
}

impl<'c> Drop for RenderContext<'c> {
    fn drop(&mut self) {
        self.canvas.finish();
    }
}

// impl<'c> piet::RenderContext for RenderContext<'c> {
//     type Brush = Brush;
//     type Text = Text;
//     type TextLayout = TextLayout;
//     type Image = BevyImage;

//     fn status(&mut self) -> Result<(), piet::Error> {
//         Ok(())
//     }

//     fn solid_brush(&mut self, color: Color) -> Self::Brush {
//         Brush { color }
//     }

//     fn gradient(&mut self, gradient: impl Into<FixedGradient>) -> Result<Self::Brush, piet::Error> {
//         unimplemented!()
//     }

//     fn clear(&mut self, region: impl Into<Option<kurbo::Rect>>, color: Color) {
//         if let Some(rect) = region.into() {
//             // TODO - delete primitives covered by region
//             self.fill(rect, &color);
//         } else {
//             self.canvas.clear();
//             self.fill(KRect::from_sprite(self.canvas.rect()), &color);
//         }
//     }

//     fn stroke(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, width: f64) {
//         let brush = brush.make_brush(self, || shape.bounding_box());
//         let color = brush.color().as_rgba();
//         if let Some(line) = shape.as_line() {
//             self.canvas.lines.push(crate::piet_canvas::Line {
//                 start: Vec2::new(line.p0.x as f32, line.p0.y as f32),
//                 end: Vec2::new(line.p1.x as f32, line.p1.y as f32),
//                 color: bevy::render::color::Color::rgba_linear(
//                     color.0 as f32,
//                     color.1 as f32,
//                     color.2 as f32,
//                     color.3 as f32,
//                 ),
//                 thickness: width as f32,
//             });
//         }
//     }

//     fn stroke_styled(
//         &mut self,
//         shape: impl Shape,
//         brush: &impl IntoBrush<Self>,
//         width: f64,
//         style: &piet::StrokeStyle,
//     ) {
//         unimplemented!()
//     }

//     fn fill(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
//         let brush = brush.make_brush(self, || shape.bounding_box());
//         let color = brush.color().as_rgba();
//         if let Some(rect) = shape.as_rect() {
//             self.canvas.quads_vec().push(Quad {
//                 rect: SRect {
//                     min: Vec2::new(rect.x0 as f32, rect.y0 as f32),
//                     max: Vec2::new(rect.x1 as f32, rect.y1 as f32),
//                 },
//                 color: bevy::render::color::Color::rgba_linear(
//                     color.0 as f32,
//                     color.1 as f32,
//                     color.2 as f32,
//                     color.3 as f32,
//                 ),
//                 flip_x: false,
//                 flip_y: false,
//             });
//         } else if let Some(line) = shape.as_line() {
//             // nothing to do; cannot "fill" a line, only stroke it
//         } else {
//             unimplemented!()
//         }
//     }

//     fn fill_even_odd(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
//         unimplemented!()
//     }

//     fn clip(&mut self, shape: impl Shape) {
//         unimplemented!()
//     }

//     fn text(&mut self) -> &mut Self::Text {
//         &mut self.text
//     }

//     fn draw_text(&mut self, layout: &Self::TextLayout, pos: impl Into<Point>) {
//         unimplemented!()
//     }

//     fn save(&mut self) -> Result<(), piet::Error> {
//         unimplemented!()
//     }

//     fn restore(&mut self) -> Result<(), piet::Error> {
//         unimplemented!()
//     }

//     fn finish(&mut self) -> Result<(), piet::Error> {
//         unimplemented!()
//     }

//     fn transform(&mut self, transform: Affine) {
//         unimplemented!()
//     }

//     fn current_transform(&self) -> Affine {
//         self.transform
//     }

//     fn make_image(
//         &mut self,
//         width: usize,
//         height: usize,
//         buf: &[u8],
//         format: piet::ImageFormat,
//     ) -> Result<Self::Image, piet::Error> {
//         Ok(BevyImage::new(width, height, buf, format))
//     }

//     fn draw_image(
//         &mut self,
//         image: &Self::Image,
//         dst_rect: impl Into<KRect>,
//         interp: piet::InterpolationMode,
//     ) {
//         unimplemented!()
//     }

//     fn draw_image_area(
//         &mut self,
//         image: &Self::Image,
//         src_rect: impl Into<KRect>,
//         dst_rect: impl Into<KRect>,
//         interp: piet::InterpolationMode,
//     ) {
//         unimplemented!()
//     }

//     fn capture_image_area(
//         &mut self,
//         src_rect: impl Into<KRect>,
//     ) -> Result<Self::Image, piet::Error> {
//         unimplemented!()
//     }

//     fn blurred_rect(&mut self, rect: KRect, blur_radius: f64, brush: &impl IntoBrush<Self>) {
//         unimplemented!()
//     }
// }
