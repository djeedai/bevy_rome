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

use crate::{
    render::ExtractedText,
    render_context::{RenderContext, TextLayout},
    text::CanvasTextId,
};

/// Implementation trait for primitives.
pub(crate) trait PrimImpl {
    /// Get the size of the primitive and index buffers, in number of elements.
    fn sizes(&self, texts: &[ExtractedText]) -> (usize, usize);

    /// Write the primitive and index buffers into the provided slices.
    fn write(
        &self,
        texts: &[ExtractedText],
        prim: &mut [MaybeUninit<f32>],
        base_index: u32,
        idx: &mut [MaybeUninit<u32>],
    );
}

const PRIM_RECT: u32 = 0;
const PRIM_LINE: u32 = 1;

/// Drawing primitives.
#[derive(Debug, Clone, Copy)]
pub enum Primitive {
    Line(LinePrimitive),
    Rect(RectPrimitive),
    Text(TextPrimitive),
}

impl From<LinePrimitive> for Primitive {
    fn from(line: LinePrimitive) -> Self {
        Self::Line(line)
    }
}

impl From<RectPrimitive> for Primitive {
    fn from(rect: RectPrimitive) -> Self {
        Self::Rect(rect)
    }
}

impl From<TextPrimitive> for Primitive {
    fn from(text: TextPrimitive) -> Self {
        Self::Text(text)
    }
}

impl PrimImpl for Primitive {
    fn sizes(&self, texts: &[ExtractedText]) -> (usize, usize) {
        match &self {
            Primitive::Line(l) => l.sizes(texts),
            Primitive::Rect(r) => r.sizes(texts),
            Primitive::Text(t) => t.sizes(texts),
        }
    }

    fn write(
        &self,
        texts: &[ExtractedText],
        prim: &mut [MaybeUninit<f32>],
        offset: u32,
        idx: &mut [MaybeUninit<u32>],
    ) {
        match &self {
            Primitive::Line(l) => l.write(texts, prim, offset, idx),
            Primitive::Rect(r) => r.write(texts, prim, offset, idx),
            Primitive::Text(t) => t.write(texts, prim, offset, idx),
        };
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LinePrimitive {
    pub start: Vec2,
    pub end: Vec2,
    pub color: Color,
    pub thickness: f32,
}

impl PrimImpl for LinePrimitive {
    fn sizes(&self, _texts: &[ExtractedText]) -> (usize, usize) {
        (6, 6)
    }

    fn write(
        &self,
        _texts: &[ExtractedText],
        prim: &mut [MaybeUninit<f32>],
        base_index: u32,
        idx: &mut [MaybeUninit<u32>],
    ) {
        assert_eq!(6, prim.len());
        prim[0].write(self.start.x);
        prim[1].write(self.start.y);
        prim[2].write(self.end.x);
        prim[3].write(self.end.y);
        prim[4].write(bytemuck::cast(self.color.as_linear_rgba_u32()));
        prim[5].write(self.thickness);
        assert_eq!(6, idx.len());
        for (i, corner) in [0, 2, 3, 0, 1, 2].iter().enumerate() {
            let index = base_index | corner << 24 | PRIM_LINE << 26;
            idx[i].write(index);
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RectPrimitive {
    pub rect: Rect,
    pub color: Color,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl RectPrimitive {
    pub fn center(&self) -> Vec3 {
        let c = (self.rect.min + self.rect.max) * 0.5;
        Vec3::new(c.x, c.y, 0.)
    }
}

impl PrimImpl for RectPrimitive {
    fn sizes(&self, _texts: &[ExtractedText]) -> (usize, usize) {
        (5, 6)
    }

    fn write(
        &self,
        _texts: &[ExtractedText],
        prim: &mut [MaybeUninit<f32>],
        base_index: u32,
        idx: &mut [MaybeUninit<u32>],
    ) {
        assert_eq!(5, prim.len());
        prim[0].write(self.rect.min.x);
        prim[1].write(self.rect.min.y);
        prim[2].write(self.rect.max.x - self.rect.min.x);
        prim[3].write(self.rect.max.y - self.rect.min.y);
        prim[4].write(bytemuck::cast(self.color.as_linear_rgba_u32()));
        assert_eq!(6, idx.len());
        for (i, corner) in [0, 2, 3, 0, 3, 1].iter().enumerate() {
            let index = base_index | corner << 24 | PRIM_RECT << 26;
            idx[i].write(index);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextPrimitive {
    pub id: u32,
    pub rect: Rect,
    pub color: Color,
    pub scale_factor: f32,
}

impl TextPrimitive {
    const ITEM_PER_GLYPH: usize = 9;
    const INDEX_PER_GLYPH: usize = 6;
}

impl PrimImpl for TextPrimitive {
    fn sizes(&self, texts: &[ExtractedText]) -> (usize, usize) {
        let index = self.id as usize;
        if index < texts.len() {
            let glyph_count = texts[index].glyphs.len();
            (glyph_count * Self::ITEM_PER_GLYPH, glyph_count * Self::INDEX_PER_GLYPH)
        } else {
            (0, 0)
        }
    }

    fn write(
        &self,
        texts: &[ExtractedText],
        prim: &mut [MaybeUninit<f32>],
        mut base_index: u32,
        idx: &mut [MaybeUninit<u32>],
    ) {
        let index = self.id as usize;
        let glyphs = &texts[index].glyphs;
        let glyph_count = glyphs.len();
        assert_eq!(glyph_count * Self::ITEM_PER_GLYPH, prim.len());
        assert_eq!(glyph_count * Self::INDEX_PER_GLYPH, idx.len());
        let mut ip = 0;
        let mut ii = 0;
        let inv_scale_factor = 1. / self.scale_factor;
        for i in 0..glyph_count {
            let x = self.rect.min.x + glyphs[i].offset.x;
            let y = self.rect.min.y + glyphs[i].offset.y;
            let w = glyphs[i].size.x;
            let h = glyphs[i].size.y;
            let uv_x = glyphs[i].uv_rect.min.x / 512.0;
            let uv_y = glyphs[i].uv_rect.min.y / 512.0;
            let uv_w = glyphs[i].uv_rect.max.x / 512.0 - uv_x;
            let uv_h = glyphs[i].uv_rect.max.y / 512.0 - uv_y;
            prim[ip + 0].write(x * inv_scale_factor);
            prim[ip + 1].write(y * inv_scale_factor);
            prim[ip + 2].write(w * inv_scale_factor);
            prim[ip + 3].write(h * inv_scale_factor);
            // FIXME - self.color vs. glyph.color ?!!!
            //prim[ip + 4].write(bytemuck::cast(self.color.as_linear_rgba_u32()));
            prim[ip + 4].write(bytemuck::cast(glyphs[i].color));
            prim[ip + 5].write(uv_x);
            prim[ip + 6].write(uv_y);
            prim[ip + 7].write(uv_w);
            prim[ip + 8].write(uv_h);
            ip += Self::ITEM_PER_GLYPH;
            for (i, corner) in [0, 2, 3, 0, 3, 1].iter().enumerate() {
                let index = base_index | corner << 24 | PRIM_RECT << 26;
                idx[ii + i].write(index);
            }
            ii += Self::INDEX_PER_GLYPH;
            base_index += Self::ITEM_PER_GLYPH as u32;
        }
    }
}

/// Drawing surface for 2D graphics.
///
/// If the component is attached to the same entity as an [`OrthographicProjection`],
/// then its dimensions are automatically computed and updated based on that projection.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Canvas {
    /// The canvas dimensions relative to its origin.
    rect: Rect,
    /// Optional background color to clear the canvas with.
    background_color: Option<Color>,
    /// Collection of drawn primitives.
    #[reflect(ignore)]
    primitives: Vec<Primitive>,
    /// Collection of allocated texts.
    #[reflect(ignore)]
    pub(crate) text_layouts: Vec<TextLayout>,
    /// Marker for text change updates.
    text_changed: bool,
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
    ///
    /// [`clear()`]: crate::canvas::Canvas::clear
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
        self.text_layouts.clear(); // FIXME - really?
        if let Some(color) = self.background_color {
            self.draw(RectPrimitive {
                rect: self.rect,
                color,
                ..default()
            });
        }
    }

    /// Draw a new primitive onto the canvas.
    ///
    /// This is a lower level entry point to canvas drawing; in general, you should
    /// prefer acquiring a [`RenderContext`] via [`render_context()`] and using it
    /// to draw primitives.
    ///
    /// [`render_context()`]: crate::canvas::Canvas::render_context
    pub fn draw(&mut self, prim: impl Into<Primitive>) {
        let prim = prim.into();
        if let Primitive::Text(text) = &prim {
            trace!("draw text #{} at rect={:?}", text.id, text.rect);
            self.text_changed = true;
            //let layout = &mut self.text_layouts[text.id.index()];
            //layout.used = true;
        }
        self.primitives.push(prim);
    }

    /// Acquire a new render context to draw on this canvas.
    pub fn render_context(&mut self) -> RenderContext {
        RenderContext::new(self)
    }

    pub(crate) fn finish(&mut self) {
        //
    }

    pub(crate) fn finish_layout(&mut self, mut layout: TextLayout) -> u32 {
        let id = self.text_layouts.len() as u32;
        trace!("finish_layout() for text #{}", id);
        layout.id = id;
        self.text_layouts.push(layout);
        id
    }

    pub(crate) fn take_buffer(&mut self) -> Vec<Primitive> {
        std::mem::take(&mut self.primitives)
    }

    pub(crate) fn text_layouts(&self) -> &[TextLayout] {
        &self.text_layouts[..]
    }

    pub(crate) fn text_changed(&self) -> bool {
        self.text_changed
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
