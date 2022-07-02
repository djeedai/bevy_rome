#![allow(unused_variables, dead_code, unused_imports)]

use std::{ops::RangeBounds, str, sync::Arc};
//use bevy::prelude::*;
use bevy::{math::Vec2, render::render_resource::Buffer, sprite::Rect as SRect, text::Font, utils::HashMap};
use kurbo::{Affine, Line, Point, Rect as KRect, Shape, Size};
use piet::{FixedGradient, HitTestPoint, HitTestPosition, IntoBrush, LineMetric};

use crate::piet_canvas::{PietCanvas, Quad};

trait KurboRectEx {
    fn from_sprite(rect: SRect) -> KRect;
    fn into_sprite(rect: KRect) -> SRect;
}

impl KurboRectEx for KRect {
    fn from_sprite(rect: SRect) -> KRect {
        let min = rect.min.as_dvec2().to_array();
        let max = rect.max.as_dvec2().to_array();
        KRect::new(min[0], min[1], max[0], max[1])
    }

    fn into_sprite(rect: KRect) -> SRect {
        SRect {
            min: Vec2::new(rect.x0 as f32, rect.y0 as f32),
            max: Vec2::new(rect.x1 as f32, rect.y1 as f32),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BevyBrush {
    color: piet::Color,
}

impl Default for BevyBrush {
    fn default() -> Self {
        Self {
            color: piet::Color::BLACK,
        }
    }
}

impl BevyBrush {
    fn color(&self) -> piet::Color {
        self.color.clone()
    }
}

impl<'q> IntoBrush<BevyRenderContext<'q>> for BevyBrush {
    fn make_brush<'b>(
        &'b self,
        _piet: &mut BevyRenderContext,
        _bbox: impl FnOnce() -> KRect,
    ) -> std::borrow::Cow<'b, BevyBrush> {
        std::borrow::Cow::Borrowed(self)
    }
}

#[derive(Debug, Default, Clone)]
pub struct BevyText;

impl piet::Text for BevyText {
    type TextLayoutBuilder = BevyTextLayoutBuilder;
    type TextLayout = BevyTextLayout;
    fn font_family(&mut self, family_name: &str) -> Option<piet::FontFamily> {
        unimplemented!()
    }
    fn load_font(&mut self, data: &[u8]) -> Result<piet::FontFamily, piet::Error> {
        unimplemented!()
    }
    fn new_text_layout(&mut self, text: impl piet::TextStorage) -> Self::TextLayoutBuilder {
        unimplemented!()
    }
}

#[derive(Debug, Default, Clone)]
pub struct BevyTextLayout;

impl piet::TextLayout for BevyTextLayout {
    fn size(&self) -> Size {
        unimplemented!()
    }
    fn trailing_whitespace_width(&self) -> f64 {
        unimplemented!()
    }
    fn image_bounds(&self) -> KRect {
        unimplemented!()
    }
    fn text(&self) -> &str {
        unimplemented!()
    }
    fn line_text(&self, line_number: usize) -> Option<&str> {
        unimplemented!()
    }
    fn line_metric(&self, line_number: usize) -> Option<LineMetric> {
        unimplemented!()
    }
    fn line_count(&self) -> usize {
        unimplemented!()
    }
    fn hit_test_point(&self, point: Point) -> HitTestPoint {
        unimplemented!()
    }
    fn hit_test_text_position(&self, idx: usize) -> HitTestPosition {
        unimplemented!()
    }
    fn rects_for_range(&self, range: impl RangeBounds<usize>) -> Vec<KRect> {
        unimplemented!()
    }
}

pub struct BevyTextLayoutBuilder;

impl piet::TextLayoutBuilder for BevyTextLayoutBuilder {
    type Out = BevyTextLayout;
    fn max_width(self, width: f64) -> Self {
        unimplemented!()
    }
    fn alignment(self, alignment: piet::TextAlignment) -> Self {
        unimplemented!()
    }
    fn default_attribute(self, attribute: impl Into<piet::TextAttribute>) -> Self {
        unimplemented!()
    }
    fn range_attribute(
        self,
        range: impl RangeBounds<usize>,
        attribute: impl Into<piet::TextAttribute>,
    ) -> Self {
        unimplemented!()
    }
    fn build(self) -> Result<Self::Out, piet::Error> {
        unimplemented!()
    }
    fn font(self, font: piet::FontFamily, font_size: f64) -> Self {
        unimplemented!()
    }
    fn text_color(self, color: piet::Color) -> Self {
        unimplemented!()
    }
}

#[derive(Debug, Default, Clone)]
pub struct BevyImage {
    image: bevy::render::texture::Image,
}

impl BevyImage {
    fn new(width: usize, height: usize, buf: &[u8], format: piet::ImageFormat) -> Self {
        let data = buf.to_vec();
        let format = match format {
            piet::ImageFormat::Grayscale => bevy::render::render_resource::TextureFormat::R8Unorm,
            piet::ImageFormat::Rgb => unimplemented!(),
            piet::ImageFormat::RgbaSeparate => {
                bevy::render::render_resource::TextureFormat::Rgba8Unorm
            }
            piet::ImageFormat::RgbaPremul => unimplemented!(),
            _ => unimplemented!(),
        };
        let image = bevy::render::texture::Image::new(
            bevy::render::render_resource::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            },
            bevy::render::render_resource::TextureDimension::D2,
            data,
            format,
        );
        Self { image }
    }
}

impl piet::Image for BevyImage {
    fn size(&self) -> Size {
        let size = self.image.size().to_array();
        Size::new(size[0] as f64, size[1] as f64)
    }
}

#[derive(Debug)]
pub struct BevyRenderContext<'q> {
    text: BevyText,
    buffer: Option<Buffer>,
    transform: Affine,
    canvas: &'q mut PietCanvas,
}

impl<'q> BevyRenderContext<'q> {
    pub fn new(canvas: &'q mut PietCanvas) -> Self {
        Self {
            text: BevyText {},
            buffer: None,
            transform: Affine::IDENTITY,
            canvas,
        }
    }

    pub fn render(ctx: &mut bevy::render::renderer::RenderContext) {}
}

impl<'q> Drop for BevyRenderContext<'q> {
    fn drop(&mut self) {
        self.canvas.finish();
    }
}

impl<'q> piet::RenderContext for BevyRenderContext<'q> {
    type Brush = BevyBrush;
    type Text = BevyText;
    type TextLayout = BevyTextLayout;
    type Image = BevyImage;

    fn status(&mut self) -> Result<(), piet::Error> {
        Ok(())
    }

    fn solid_brush(&mut self, color: piet::Color) -> Self::Brush {
        BevyBrush { color }
    }

    fn gradient(&mut self, gradient: impl Into<FixedGradient>) -> Result<Self::Brush, piet::Error> {
        unimplemented!()
    }

    fn clear(&mut self, region: impl Into<Option<kurbo::Rect>>, color: piet::Color) {
        if let Some(rect) = region.into() {
            // TODO - delete primitives covered by region
            self.fill(rect, &color);
        } else {
            self.canvas.clear();
            self.fill(KRect::from_sprite(self.canvas.rect()), &color);
        }
    }

    fn stroke(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, width: f64) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        let color = brush.color().as_rgba();
        if let Some(line) = shape.as_line() {
            self.canvas.lines.push(crate::piet_canvas::Line {
                start: Vec2::new(line.p0.x as f32, line.p0.y as f32),
                end: Vec2::new(line.p1.x as f32, line.p1.y as f32),
                color: bevy::render::color::Color::rgba_linear(
                    color.0 as f32,
                    color.1 as f32,
                    color.2 as f32,
                    color.3 as f32,
                ),
                thickness: width as f32,
            });
        }
    }

    fn stroke_styled(
        &mut self,
        shape: impl Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: &piet::StrokeStyle,
    ) {
        unimplemented!()
    }

    fn fill(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        let brush = brush.make_brush(self, || shape.bounding_box());
        let color = brush.color().as_rgba();
        if let Some(rect) = shape.as_rect() {
            self.canvas.quads_vec().push(Quad {
                rect: SRect {
                    min: Vec2::new(rect.x0 as f32, rect.y0 as f32),
                    max: Vec2::new(rect.x1 as f32, rect.y1 as f32),
                },
                color: bevy::render::color::Color::rgba_linear(
                    color.0 as f32,
                    color.1 as f32,
                    color.2 as f32,
                    color.3 as f32,
                ),
                flip_x: false,
                flip_y: false,
            });
        } else if let Some(line) = shape.as_line() {
            // nothing to do; cannot "fill" a line, only stroke it
        } else {
            unimplemented!()
        }
    }

    fn fill_even_odd(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        unimplemented!()
    }

    fn clip(&mut self, shape: impl Shape) {
        unimplemented!()
    }

    fn text(&mut self) -> &mut Self::Text {
        &mut self.text
    }

    fn draw_text(&mut self, layout: &Self::TextLayout, pos: impl Into<Point>) {
        unimplemented!()
    }

    fn save(&mut self) -> Result<(), piet::Error> {
        unimplemented!()
    }

    fn restore(&mut self) -> Result<(), piet::Error> {
        unimplemented!()
    }

    fn finish(&mut self) -> Result<(), piet::Error> {
        unimplemented!()
    }

    fn transform(&mut self, transform: Affine) {
        unimplemented!()
    }

    fn current_transform(&self) -> Affine {
        self.transform
    }

    fn make_image(
        &mut self,
        width: usize,
        height: usize,
        buf: &[u8],
        format: piet::ImageFormat,
    ) -> Result<Self::Image, piet::Error> {
        Ok(BevyImage::new(width, height, buf, format))
    }

    fn draw_image(
        &mut self,
        image: &Self::Image,
        dst_rect: impl Into<KRect>,
        interp: piet::InterpolationMode,
    ) {
        unimplemented!()
    }

    fn draw_image_area(
        &mut self,
        image: &Self::Image,
        src_rect: impl Into<KRect>,
        dst_rect: impl Into<KRect>,
        interp: piet::InterpolationMode,
    ) {
        unimplemented!()
    }

    fn capture_image_area(
        &mut self,
        src_rect: impl Into<KRect>,
    ) -> Result<Self::Image, piet::Error> {
        unimplemented!()
    }

    fn blurred_rect(&mut self, rect: KRect, blur_radius: f64, brush: &impl IntoBrush<Self>) {
        unimplemented!()
    }
}
