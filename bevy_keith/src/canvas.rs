use std::mem::MaybeUninit;

use bevy::{
    asset::HandleId,
    core::{Pod, Zeroable},
    ecs::{component::Component, reflect::ReflectComponent, system::Query},
    log::trace,
    math::{Vec2, Vec3},
    prelude::OrthographicProjection,
    reflect::Reflect,
    render::color::Color,
    sprite::Rect,
    utils::default,
};

use crate::{
    render::{ExtractedGlyph, ExtractedText},
    render_context::{RenderContext, TextLayout},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PrimitiveInfo {
    pub row_count: u32,
    pub index_count: u32,
}

fn to_array(r: &Rect) -> [f32; 4] {
    let mut arr = [0.; 4];
    arr[..2].copy_from_slice(&r.min.to_array());
    arr[2..].copy_from_slice(&r.max.to_array());
    arr
}

enum GpuPrimitive {
    Rect(GpuRect),
    Line(GpuLine),
}

/// GPU representation into a primitive buffer of a [`RectPrimitive`].
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct GpuRect {
    rect: [f32; 4],
    color: u32,
}

impl From<&RectPrimitive> for GpuRect {
    fn from(r: &RectPrimitive) -> GpuRect {
        GpuRect {
            rect: to_array(&r.rect),
            color: r.color.as_linear_rgba_u32(),
        }
    }
}

/// GPU representation into a primitive buffer of a [`LinePrimitive`].
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct GpuLine {
    start: [f32; 2],
    end: [f32; 2],
    color: u32,
    thickness: f32,
}

impl From<&LinePrimitive> for GpuLine {
    fn from(l: &LinePrimitive) -> GpuLine {
        GpuLine {
            start: l.start.to_array(),
            end: l.end.to_array(),
            color: l.color.as_linear_rgba_u32(),
            thickness: l.thickness,
        }
    }
}

/// Stream of GPU primitives and their associated drawing indices.
pub(crate) struct PrimitiveStream {
    /// Buffer of primitives, encoded as [`f32`] for easy shader access.
    primitives: Vec<f32>,
    /// Buffer of indices for drawing the primitives.
    indices: Vec<u32>,
}

impl PrimitiveStream {
    /// Write a primitive into the stream.
    pub fn write<T: Pod>(&mut self, blob: &T, corners: &[u8], kind: GpuPrimitiveKind, texture_index: u8) {
        // Write the indices to draw the primitive
        let base_index = self.primitives.len() as u32;
        self.indices.extend(corners.iter().map(|&c| GpuIndex::new(base_index, c, kind, texture_index).raw()));

        // Write the primitive itself
        let raw: &[u8] = bytemuck::bytes_of(blob);
        let raw: &[f32] = bytemuck::cast_slice(raw);
        self.primitives.extend_from_slice(raw);
    }
}

/// Implementation trait for primitives.
pub(crate) trait PrimImpl {
    /// Get the size of the primitive and index buffers, in number of elements.
    fn info(&self, texts: &[ExtractedText]) -> PrimitiveInfo;

    /// Write the primitive and index buffers into the provided slices.
    ///
    /// The `scale_factor` is a scaling for text glyphs only.
    fn write(
        &self,
        texts: &[ExtractedText],
        prim: &mut [MaybeUninit<f32>],
        base_index: u32,
        idx: &mut [MaybeUninit<u32>],
        scale_factor: f32,
    );

    /// Write the primitive into the given stream.
    fn write_stream(&self, stream: &mut PrimitiveStream);
}

/// Kind of primitives understood by the GPU shader.
///
/// # Note
///
/// The enum values must be kept in sync with the values inside the primitive shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum GpuPrimitiveKind {
    /// Axis-aligned rectangle, possibly textured.
    Rect = 0,
    /// Line segment.
    Line = 1,
}

/// Encoded vertex index passed to the GPU shader.
///
/// # Note
///
/// The encoding must be kept in sync with the values inside the primitive shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GpuIndex(u32);

impl GpuIndex {
    /// Create a new encoded index from a base primitive buffer index, a corner specification,
    /// and a kind of primitive to draw.
    ///
    ///  31                                           0
    /// [ ttt | kkk |  cc  | bbbbbbbb bbbbbbbb bbbbbbb ]
    ///   Tex   Kind Corner          Base index
    #[inline]
    pub fn new(base_index: u32, corner: u8, kind: GpuPrimitiveKind, texture_index: u8) -> Self {
        GpuIndex(
            base_index
                | ((corner as u32) << 24)
                | ((kind as u32) << 26)
                | ((texture_index as u32) << 29),
        )
    }

    /// Get the raw encoded index value.
    #[inline]
    pub fn raw(&self) -> u32 {
        self.0
    }
}

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
    fn info(&self, texts: &[ExtractedText]) -> PrimitiveInfo {
        match &self {
            Primitive::Line(l) => l.info(texts),
            Primitive::Rect(r) => r.info(texts),
            Primitive::Text(t) => t.info(texts),
        }
    }

    fn write(
        &self,
        texts: &[ExtractedText],
        prim: &mut [MaybeUninit<f32>],
        offset: u32,
        idx: &mut [MaybeUninit<u32>],
        scale_factor: f32,
    ) {
        match &self {
            Primitive::Line(l) => l.write(texts, prim, offset, idx, scale_factor),
            Primitive::Rect(r) => r.write(texts, prim, offset, idx, scale_factor),
            Primitive::Text(t) => t.write(texts, prim, offset, idx, scale_factor),
        };
    }

    fn write_stream(&self, stream: &mut PrimitiveStream) {
        match &self {
            Primitive::Line(l) => l.write_stream(stream),
            Primitive::Rect(r) => r.write_stream(stream),
            Primitive::Text(t) => t.write_stream(stream),
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
    fn info(&self, _texts: &[ExtractedText]) -> PrimitiveInfo {
        PrimitiveInfo {
            row_count: 6,
            index_count: 6,
        }
    }

    fn write(
        &self,
        _texts: &[ExtractedText],
        prim: &mut [MaybeUninit<f32>],
        base_index: u32,
        idx: &mut [MaybeUninit<u32>],
        _scale_factor: f32,
    ) {
        assert_eq!(6, prim.len());
        prim[0].write(self.start.x);
        prim[1].write(self.start.y);
        prim[2].write(self.end.x);
        prim[3].write(self.end.y);
        prim[4].write(bytemuck::cast(self.color.as_linear_rgba_u32()));
        prim[5].write(self.thickness);
        assert_eq!(6, idx.len());
        for (i, corner) in [0, 2, 3, 0, 3, 1].iter().enumerate() {
            let index = GpuIndex::new(base_index, *corner as u8, GpuPrimitiveKind::Line, 0);
            idx[i].write(index.raw());
        }
    }

    #[inline]
    fn write_stream(&self, stream: &mut PrimitiveStream) {
        let gpu: GpuLine = self.into();
        stream.write(&gpu, &[0, 2, 3, 0, 3, 1], GpuPrimitiveKind::Line, 0);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RectPrimitive {
    /// Position and size of the rectangle in its canvas space.
    pub rect: Rect,
    /// Uniform rectangle color.
    pub color: Color,
    /// Flip the image (if any) along the horizontal axis.
    pub flip_x: bool,
    /// Flip the image (if any) along the vertical axis.
    pub flip_y: bool,
    /// Optional handle to the image used for texturing the rectangle.
    /// This uses `HandleId` to retain the `Copy` trait.
    pub image: Option<HandleId>, // Handle<Image>
}

impl Default for RectPrimitive {
    fn default() -> Self {
        Self {
            rect: Rect::default(),
            color: Color::default(),
            flip_x: false,
            flip_y: false,
            image: None,
        }
    }
}

impl RectPrimitive {
    /// Number of primitive buffer rows (4 bytes) per primitive.
    const ROW_COUNT: u32 = 5;
    /// Number of primitive buffer rows (4 bytes) per primitive when textured.
    const ROW_COUNT_TEX: u32 = 9;

    /// Number of indices per primitive (2 triangles).
    const INDEX_COUNT: u32 = 6;

    pub fn center(&self) -> Vec3 {
        let c = (self.rect.min + self.rect.max) * 0.5;
        Vec3::new(c.x, c.y, 0.)
    }

    #[inline]
    const fn row_count(&self) -> u32 {
        if self.image.is_some() {
            Self::ROW_COUNT_TEX
        } else {
            Self::ROW_COUNT
        }
    }
}

impl PrimImpl for RectPrimitive {
    fn info(&self, _texts: &[ExtractedText]) -> PrimitiveInfo {
        PrimitiveInfo {
            row_count: self.row_count(),
            index_count: Self::INDEX_COUNT,
        }
    }

    fn write(
        &self,
        _texts: &[ExtractedText],
        prim: &mut [MaybeUninit<f32>],
        base_index: u32,
        idx: &mut [MaybeUninit<u32>],
        _scale_factor: f32,
    ) {
        assert_eq!(self.row_count() as usize, prim.len());
        prim[0].write(self.rect.min.x);
        prim[1].write(self.rect.min.y);
        prim[2].write(self.rect.max.x - self.rect.min.x);
        prim[3].write(self.rect.max.y - self.rect.min.y);
        prim[4].write(bytemuck::cast(self.color.as_linear_rgba_u32()));
        if self.image.is_some() {
            prim[5].write(0.);
            prim[6].write(1.);
            prim[7].write(1.);
            prim[8].write(-1.);
        }
        assert_eq!(6, idx.len());
        for (i, corner) in [0, 2, 3, 0, 3, 1].iter().enumerate() {
            let index = GpuIndex::new(base_index, *corner as u8, GpuPrimitiveKind::Rect, 0);
            idx[i].write(index.raw());
        }
    }

    #[inline]
    fn write_stream(&self, stream: &mut PrimitiveStream) {
        let gpu: GpuRect = self.into();
        stream.write(&gpu, &[0, 2, 3, 0, 3, 1], GpuPrimitiveKind::Rect, 0);
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextPrimitive {
    /// Unique ID of the text inside its owner [`Canvas`].
    pub id: u32,
    pub rect: Rect,
}

impl TextPrimitive {
    pub const ROW_PER_GLYPH: u32 = 9;
    pub const INDEX_PER_GLYPH: u32 = 6;

    pub fn bounding_rect(&self, glyph: &ExtractedGlyph, inv_scale_factor: f32) -> Rect {
        // Glyph position is center of rect, we need bottom-left corner for min
        let min = glyph.offset - glyph.size / 2.;
        let min = self.rect.min + min * inv_scale_factor;
        Rect {
            min,
            max: min + glyph.size * inv_scale_factor,
        }
    }
}

impl PrimImpl for TextPrimitive {
    fn info(&self, texts: &[ExtractedText]) -> PrimitiveInfo {
        let index = self.id as usize;
        if index < texts.len() {
            let glyph_count = texts[index].glyphs.len() as u32;
            PrimitiveInfo {
                row_count: glyph_count * Self::ROW_PER_GLYPH,
                index_count: glyph_count * Self::INDEX_PER_GLYPH,
            }
        } else {
            PrimitiveInfo {
                row_count: 0,
                index_count: 0,
            }
        }
    }

    fn write(
        &self,
        texts: &[ExtractedText],
        prim: &mut [MaybeUninit<f32>],
        mut base_index: u32,
        idx: &mut [MaybeUninit<u32>],
        scale_factor: f32,
    ) {
        let index = self.id as usize;
        let glyphs = &texts[index].glyphs;
        let glyph_count = glyphs.len();
        assert_eq!(glyph_count * Self::ROW_PER_GLYPH as usize, prim.len());
        assert_eq!(glyph_count * Self::INDEX_PER_GLYPH as usize, idx.len());
        let mut ip = 0;
        let mut ii = 0;
        let inv_scale_factor = 1. / scale_factor;
        for i in 0..glyph_count {
            let pos = glyphs[i].offset;
            let size = glyphs[i].size;
            // Glyph position is center of rect, we need bottom-left corner
            let pos = pos - size / 2.;
            let pos = self.rect.min + pos * inv_scale_factor;
            let size = size * inv_scale_factor;
            let mut uv_pos = glyphs[i].uv_rect.min / 512.0;
            let mut uv_size = glyphs[i].uv_rect.max / 512.0 - uv_pos;
            // Glyph UV is flipped vertically
            uv_pos.y = uv_pos.y + uv_size.y;
            uv_size.y = -uv_size.y;
            prim[ip + 0].write(pos.x);
            prim[ip + 1].write(pos.y);
            prim[ip + 2].write(size.x);
            prim[ip + 3].write(size.y);
            prim[ip + 4].write(bytemuck::cast(glyphs[i].color));
            prim[ip + 5].write(uv_pos.x);
            prim[ip + 6].write(uv_pos.y);
            prim[ip + 7].write(uv_size.x);
            prim[ip + 8].write(uv_size.y);
            ip += Self::ROW_PER_GLYPH as usize;
            for (i, corner) in [0, 2, 3, 0, 3, 1].iter().enumerate() {
                let index = GpuIndex::new(base_index, *corner as u8, GpuPrimitiveKind::Glyph, 0);
                idx[ii + i].write(index.raw());
            }
            ii += Self::INDEX_PER_GLYPH as usize;
            base_index += Self::ROW_PER_GLYPH;
        }
    }

    fn write_stream(&self, stream: &mut PrimitiveStream) {
        TODO - we should write only a sub-primitive here, because some glyphs might get grouped differently if inside a different atlas texture...
        let gpu: GpuRect = self.into();
        stream.write(&gpu);
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

    // Currently unused; see buffer()
    pub(crate) fn take_buffer(&mut self) -> Vec<Primitive> {
        std::mem::take(&mut self.primitives)
    }

    // Workaround for Extract phase without mut access to MainWorld Canvas
    pub(crate) fn buffer(&self) -> &Vec<Primitive> {
        &self.primitives
    }

    pub(crate) fn text_layouts(&self) -> &[TextLayout] {
        &self.text_layouts[..]
    }

    pub(crate) fn text_layouts_mut(&mut self) -> &mut [TextLayout] {
        &mut self.text_layouts[..]
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
