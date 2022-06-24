#![allow(unused_variables, dead_code, unused_imports)]

use std::ops::RangeBounds;
//use bevy::prelude::*;
use bevy::render::render_resource::Buffer;
use kurbo::{Affine, Point, Rect, Shape, Size};
use piet::{IntoBrush, *};

#[derive(Clone)]
struct BevyBrush {
    color: Color,
}

#[derive(Clone)]
struct BevyText;

impl Text for BevyText {
    type TextLayoutBuilder = BevyTextLayoutBuilder;
    type TextLayout = BevyTextLayout;
    fn font_family(&mut self, family_name: &str) -> Option<FontFamily> {
        unimplemented!()
    }
    fn load_font(&mut self, data: &[u8]) -> Result<FontFamily, Error> {
        unimplemented!()
    }
    fn new_text_layout(&mut self, text: impl TextStorage) -> Self::TextLayoutBuilder {
        unimplemented!()
    }
}

#[derive(Clone)]
struct BevyTextLayout;

impl TextLayout for BevyTextLayout {
    fn size(&self) -> Size {
        unimplemented!()
    }
    fn trailing_whitespace_width(&self) -> f64 {
        unimplemented!()
    }
    fn image_bounds(&self) -> Rect {
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
    fn rects_for_range(&self, range: impl RangeBounds<usize>) -> Vec<Rect> {
        unimplemented!()
    }
}

struct BevyTextLayoutBuilder;

impl TextLayoutBuilder for BevyTextLayoutBuilder {
    type Out = BevyTextLayout;
    fn max_width(self, width: f64) -> Self {
        unimplemented!()
    }
    fn alignment(self, alignment: TextAlignment) -> Self {
        unimplemented!()
    }
    fn default_attribute(self, attribute: impl Into<TextAttribute>) -> Self {
        unimplemented!()
    }
    fn range_attribute(
        self,
        range: impl RangeBounds<usize>,
        attribute: impl Into<TextAttribute>,
    ) -> Self {
        unimplemented!()
    }
    fn build(self) -> Result<Self::Out, Error> {
        unimplemented!()
    }
    fn font(self, font: FontFamily, font_size: f64) -> Self {
        unimplemented!()
    }
    fn text_color(self, color: Color) -> Self {
        unimplemented!()
    }
}

#[derive(Clone)]
struct BevyImage {
    image: bevy::render::texture::Image,
}

impl BevyImage {
    fn new(width: usize, height: usize, buf: &[u8], format: ImageFormat) -> Self {
        let data = buf.to_vec();
        let format = match format {
            ImageFormat::Grayscale => bevy::render::render_resource::TextureFormat::R8Unorm,
            ImageFormat::Rgb => unimplemented!(),
            ImageFormat::RgbaSeparate => bevy::render::render_resource::TextureFormat::Rgba8Unorm,
            ImageFormat::RgbaPremul => unimplemented!(),
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

impl Image for BevyImage {
    fn size(&self) -> Size {
        let size = self.image.size().to_array();
        Size::new(size[0] as f64, size[1] as f64)
    }
}

struct BevyRenderContext {
    text: BevyText,
    buffer: Option<Buffer>,
    transform: Affine,
}

impl BevyRenderContext {
    pub fn new() -> Self {
        Self {
            text: BevyText {},
            buffer: None,
            transform: Affine::IDENTITY,
        }
    }

    pub fn render(ctx: &mut bevy::render::renderer::RenderContext) {}
}

impl IntoBrush<BevyRenderContext> for BevyBrush {
    fn make_brush<'b>(
        &'b self,
        _piet: &mut BevyRenderContext,
        _bbox: impl FnOnce() -> Rect,
    ) -> std::borrow::Cow<'b, BevyBrush> {
        std::borrow::Cow::Borrowed(self)
    }
}

impl RenderContext for BevyRenderContext {
    type Brush = BevyBrush;
    type Text = BevyText;
    type TextLayout = BevyTextLayout;
    type Image = BevyImage;

    fn status(&mut self) -> Result<(), Error> {
        Ok(())
    }

    fn solid_brush(&mut self, color: Color) -> Self::Brush {
        BevyBrush { color }
    }

    fn gradient(&mut self, gradient: impl Into<FixedGradient>) -> Result<Self::Brush, Error> {
        Ok(self.solid_brush(Color::PURPLE))
    }

    fn clear(&mut self, region: impl Into<Option<Rect>>, color: Color) {
        unimplemented!()
    }

    fn stroke(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>, width: f64) {
        unimplemented!()
    }

    fn stroke_styled(
        &mut self,
        shape: impl Shape,
        brush: &impl IntoBrush<Self>,
        width: f64,
        style: &StrokeStyle,
    ) {
        unimplemented!()
    }

    fn fill(&mut self, shape: impl Shape, brush: &impl IntoBrush<Self>) {
        unimplemented!()
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

    fn save(&mut self) -> Result<(), Error> {
        unimplemented!()
    }

    fn restore(&mut self) -> Result<(), Error> {
        unimplemented!()
    }

    fn finish(&mut self) -> Result<(), Error> {
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
        format: ImageFormat,
    ) -> Result<Self::Image, Error> {
        Ok(BevyImage::new(width, height, buf, format))
    }

    fn draw_image(
        &mut self,
        image: &Self::Image,
        dst_rect: impl Into<Rect>,
        interp: InterpolationMode,
    ) {
        unimplemented!()
    }

    fn draw_image_area(
        &mut self,
        image: &Self::Image,
        src_rect: impl Into<Rect>,
        dst_rect: impl Into<Rect>,
        interp: InterpolationMode,
    ) {
        unimplemented!()
    }

    fn capture_image_area(&mut self, src_rect: impl Into<Rect>) -> Result<Self::Image, Error> {
        unimplemented!()
    }

    fn blurred_rect(&mut self, rect: Rect, blur_radius: f64, brush: &impl IntoBrush<Self>) {
        unimplemented!()
    }
}
