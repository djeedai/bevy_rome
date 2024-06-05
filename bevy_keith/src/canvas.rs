use std::mem::MaybeUninit;

use bevy::{
    asset::AssetId,
    ecs::{
        component::Component,
        entity::Entity,
        query::{With, Without},
        reflect::ReflectComponent,
        system::{Commands, Query},
    },
    log::trace,
    math::{bounding::Aabb2d, Rect, UVec2, Vec2, Vec3},
    prelude::{BVec2, OrthographicProjection},
    reflect::Reflect,
    render::{camera::Camera, color::Color, primitives::Frustum, texture::Image},
    transform::components::GlobalTransform,
    utils::default,
};
use bytemuck::{Pod, Zeroable};

use crate::{
    render::ExtractedText,
    render_context::{RenderContext, TextLayout},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PrimitiveInfo {
    pub row_count: u32,
}

/// Implementation trait for primitives.
pub(crate) trait PrimImpl {
    /// Get the size of the primitive and index buffers, in number of elements.
    fn info(&self, texts: &[ExtractedText]) -> PrimitiveInfo;

    /// Write the primitive and index buffers into the provided slices.
    ///
    /// The `scale_factor` is a scaling for text glyphs only.
    fn write(&self, texts: &[ExtractedText], prim: &mut [MaybeUninit<f32>], scale_factor: f32);
}

/// Kind of primitives understood by the GPU shader.
///
/// # Note
///
/// The enum values must be kept in sync with the values inside the primitive
/// shader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GpuPrimitiveKind {
    /// Axis-aligned rectangle, possibly textured.
    Rect = 0,
    /// Text glyph. Same as `Rect`, but samples from texture's alpha instead of
    /// RGB, and is always textured.
    Glyph = 1,
    /// Line segment.
    Line = 2,
    /// Quarter pie.
    QuarterPie = 3,
}

/// Drawing primitives.
#[derive(Debug, Clone, Copy)]
pub enum Primitive {
    Line(LinePrimitive),
    Rect(RectPrimitive),
    Text(TextPrimitive),
    QuarterPie(QuarterPiePrimitive),
}

impl Primitive {
    pub fn gpu_kind(&self) -> GpuPrimitiveKind {
        match self {
            Primitive::Line(_) => GpuPrimitiveKind::Line,
            Primitive::Rect(_) => GpuPrimitiveKind::Rect,
            Primitive::Text(_) => GpuPrimitiveKind::Glyph,
            Primitive::QuarterPie(_) => GpuPrimitiveKind::QuarterPie,
        }
    }

    pub fn aabb(&self) -> Aabb2d {
        match self {
            Primitive::Line(l) => l.aabb(),
            Primitive::Rect(r) => r.aabb(),
            Primitive::Text(t) => t.aabb(),
            Primitive::QuarterPie(q) => q.aabb(),
        }
    }

    pub fn is_textured(&self) -> bool {
        match self {
            Primitive::Line(_) => false,
            Primitive::Rect(r) => r.is_textured(),
            Primitive::Text(_) => true,
            Primitive::QuarterPie(_) => false,
        }
    }
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

impl From<QuarterPiePrimitive> for Primitive {
    fn from(qpie: QuarterPiePrimitive) -> Self {
        Self::QuarterPie(qpie)
    }
}

impl PrimImpl for Primitive {
    fn info(&self, texts: &[ExtractedText]) -> PrimitiveInfo {
        match &self {
            Primitive::Line(l) => l.info(texts),
            Primitive::Rect(r) => r.info(texts),
            Primitive::Text(t) => t.info(texts),
            Primitive::QuarterPie(q) => q.info(texts),
        }
    }

    fn write(&self, texts: &[ExtractedText], prim: &mut [MaybeUninit<f32>], scale_factor: f32) {
        match &self {
            Primitive::Line(l) => l.write(texts, prim, scale_factor),
            Primitive::Rect(r) => r.write(texts, prim, scale_factor),
            Primitive::Text(t) => t.write(texts, prim, scale_factor),
            Primitive::QuarterPie(q) => q.write(texts, prim, scale_factor),
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

impl LinePrimitive {
    pub fn aabb(&self) -> Aabb2d {
        let dir = (self.end - self.start).normalize();
        let tg = Vec2::new(-dir.y, dir.x);
        let e = self.thickness / 2.;
        let p0 = self.start + tg * e;
        let p1 = self.start - tg * e;
        let p2 = self.end + tg * e;
        let p3 = self.end - tg * e;
        let min = p0.min(p1).min(p2).min(p3);
        let max = p0.max(p1).max(p2).max(p3);
        Aabb2d { min, max }
    }
}

impl PrimImpl for LinePrimitive {
    fn info(&self, _texts: &[ExtractedText]) -> PrimitiveInfo {
        PrimitiveInfo { row_count: 6 }
    }

    fn write(&self, _texts: &[ExtractedText], prim: &mut [MaybeUninit<f32>], _scale_factor: f32) {
        assert_eq!(6, prim.len());
        prim[0].write(self.start.x);
        prim[1].write(self.start.y);
        prim[2].write(self.end.x);
        prim[3].write(self.end.y);
        prim[4].write(bytemuck::cast(self.color.as_linear_rgba_u32()));
        prim[5].write(self.thickness);
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RectPrimitive {
    /// Position and size of the rectangle in its canvas space.
    ///
    /// For rounded rectangles, this is the AABB (the radius is included).
    pub rect: Rect,
    /// Rounded corners radius.
    pub radius: f32,
    /// Uniform rectangle color.
    pub color: Color,
    /// Flip the image (if any) along the horizontal axis.
    pub flip_x: bool,
    /// Flip the image (if any) along the vertical axis.
    pub flip_y: bool,
    /// Optional handle to the image used for texturing the rectangle.
    /// This uses `HandleId` to retain the `Copy` trait.
    pub image: Option<AssetId<Image>>, // Handle<Image>
}

impl RectPrimitive {
    /// Number of primitive buffer rows (4 bytes) per primitive.
    const ROW_COUNT: u32 = 6;
    /// Number of primitive buffer rows (4 bytes) per primitive when textured.
    const ROW_COUNT_TEX: u32 = 10;

    pub fn aabb(&self) -> Aabb2d {
        Aabb2d {
            min: self.rect.min,
            max: self.rect.max,
        }
    }

    pub fn is_textured(&self) -> bool {
        self.image.is_some()
    }

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
        }
    }

    fn write(&self, _texts: &[ExtractedText], prim: &mut [MaybeUninit<f32>], _scale_factor: f32) {
        assert_eq!(
            self.row_count() as usize,
            prim.len(),
            "Invalid buffer size {} to write RectPrimitive (needs {})",
            prim.len(),
            self.row_count()
        );
        let half_min = self.rect.min * 0.5;
        let half_max = self.rect.max * 0.5;
        let center = half_min + half_max;
        let half_size = half_max - half_min;
        prim[0].write(center.x);
        prim[1].write(center.y);
        prim[2].write(half_size.x);
        prim[3].write(half_size.y);
        prim[4].write(self.radius);
        prim[5].write(bytemuck::cast(self.color.as_linear_rgba_u32()));
        if self.image.is_some() {
            prim[6].write(0.5);
            prim[7].write(0.5);
            prim[8].write(1. / 16.); // FIXME - hardcoded image size + mapping (scale 1:1, fit to rect, etc.)
            prim[9].write(1. / 16.); // FIXME - hardcoded image size + mapping
                                     // (scale 1:1, fit to rect, etc.)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TextPrimitive {
    /// Unique ID of the text inside its owner [`Canvas`].
    pub id: u32,
    pub rect: Rect,
}

impl TextPrimitive {
    /// Number of elements used by each single glyph in the primitive element
    /// buffer.
    pub const ROW_PER_GLYPH: u32 = 9;

    pub fn aabb(&self) -> Aabb2d {
        // TODO : verify what is self.rect, and that it bounds the text glyphs indeed
        Aabb2d {
            min: self.rect.min,
            max: self.rect.max,
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
            }
        } else {
            PrimitiveInfo { row_count: 0 }
        }
    }

    fn write(&self, texts: &[ExtractedText], prim: &mut [MaybeUninit<f32>], scale_factor: f32) {
        let index = self.id as usize;
        let glyphs = &texts[index].glyphs;
        let glyph_count = glyphs.len();
        assert_eq!(glyph_count * Self::ROW_PER_GLYPH as usize, prim.len());
        let mut ip = 0;
        let inv_scale_factor = 1. / scale_factor;
        for i in 0..glyph_count {
            let x = glyphs[i].offset.x;
            let y = glyphs[i].offset.y;
            let w = glyphs[i].size.x;
            let h = glyphs[i].size.y;
            // Glyph position is center of rect, we need bottom-left corner
            let x = x - w / 2.;
            let y = y - h / 2.;
            let uv_x = glyphs[i].uv_rect.min.x / 512.0;
            let uv_y = glyphs[i].uv_rect.min.y / 512.0;
            let uv_w = glyphs[i].uv_rect.max.x / 512.0 - uv_x;
            let uv_h = glyphs[i].uv_rect.max.y / 512.0 - uv_y;
            // Glyph UV is flipped vertically
            let uv_y = uv_y + uv_h;
            let uv_h = -uv_h;
            prim[ip + 0].write(self.rect.min.x + x * inv_scale_factor);
            prim[ip + 1].write(self.rect.min.y + y * inv_scale_factor);
            prim[ip + 2].write(w * inv_scale_factor);
            prim[ip + 3].write(h * inv_scale_factor);
            prim[ip + 4].write(bytemuck::cast(glyphs[i].color));
            prim[ip + 5].write(uv_x);
            prim[ip + 6].write(uv_y);
            prim[ip + 7].write(uv_w);
            prim[ip + 8].write(uv_h);
            ip += Self::ROW_PER_GLYPH as usize;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct QuarterPiePrimitive {
    /// Origin of the pie.
    pub origin: Vec2,
    /// Radii of the (elliptical) pie.
    pub radii: Vec2,
    /// Uniform rectangle color.
    pub color: Color,
    /// Flip the quarter pie along the horizontal axis.
    pub flip_x: bool,
    /// Flip the quarter pie along the vertical axis.
    pub flip_y: bool,
}

impl Default for QuarterPiePrimitive {
    fn default() -> Self {
        Self {
            origin: Vec2::ZERO,
            radii: Vec2::ONE,
            color: Color::default(),
            flip_x: false,
            flip_y: false,
        }
    }
}

impl QuarterPiePrimitive {
    /// Number of primitive buffer rows (4 bytes) per primitive.
    const ROW_COUNT: u32 = 5;

    /// Number of indices per primitive (2 triangles).
    const INDEX_COUNT: u32 = 6;

    pub fn aabb(&self) -> Aabb2d {
        Aabb2d {
            min: self.origin - self.radii,
            max: self.origin + self.radii,
        }
    }

    /// The pie center.
    pub fn center(&self) -> Vec3 {
        self.origin.extend(0.)
    }

    #[inline]
    const fn row_count(&self) -> u32 {
        Self::ROW_COUNT
    }
}

impl PrimImpl for QuarterPiePrimitive {
    fn info(&self, _texts: &[ExtractedText]) -> PrimitiveInfo {
        PrimitiveInfo {
            row_count: self.row_count(),
        }
    }

    fn write(&self, _texts: &[ExtractedText], prim: &mut [MaybeUninit<f32>], _scale_factor: f32) {
        assert_eq!(self.row_count() as usize, prim.len());
        let radii_mask = BVec2::new(self.flip_x, self.flip_y);
        let signed_radii = Vec2::select(radii_mask, -self.radii, self.radii);
        prim[0].write(self.origin.x);
        prim[1].write(self.origin.y);
        prim[2].write(signed_radii.x);
        prim[3].write(signed_radii.y);
        prim[4].write(bytemuck::cast(self.color.as_linear_rgba_u32()));
    }
}

/// Drawing surface for 2D graphics.
///
/// This component should attached to the same entity as a [`Camera`] and an
/// [`OrthographicProjection`].
///
/// By default the dimensions of the canvas are automatically computed and
/// updated based on that projection.
#[derive(Component, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct Canvas {
    /// The canvas dimensions relative to its origin.
    rect: Rect,
    /// Optional background color to clear the canvas with.
    ///
    /// This only has an effect starting from the next [`clear()`] call. If a
    /// background color is set, it's used to clear the canvas each frame.
    /// Otherwise, the canvas retains its default transparent black color (0.0,
    /// 0.0, 0.0, 0.0).
    ///
    /// [`clear()`]: crate::Canvas::clear
    pub background_color: Option<Color>,
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

    /// Change the dimensions of the canvas.
    ///
    /// This is called automatically if the [`Canvas`] is on the same entity as
    /// an [`OrthographicProjection`].
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

    /// Clear the canvas, discarding all primitives previously drawn on it.
    pub fn clear(&mut self) {
        self.primitives.clear();
        self.text_layouts.clear(); // FIXME - really?
                                   // if let Some(color) = self.background_color
                                   // {
                                   //     self.draw(RectPrimitive {
                                   //         rect: self.rect,
                                   //         color,
                                   //         ..default()
                                   //     });
                                   // }
    }

    /// Draw a new primitive onto the canvas.
    ///
    /// This is a lower level entry point to canvas drawing; in general, you
    /// should prefer acquiring a [`RenderContext`] via [`render_context()`]
    /// and using it to draw primitives.
    ///
    /// [`render_context()`]: crate::canvas::Canvas::render_context
    pub fn draw(&mut self, prim: impl Into<Primitive>) {
        let prim = prim.into();
        if let Primitive::Text(text) = &prim {
            trace!("draw text #{} at rect={:?}", text.id, text.rect);
            self.text_changed = true;
            // let layout = &mut self.text_layouts[text.id.index()];
            // layout.used = true;
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

/// Update the dimensions of any [`Canvas`] component attached to the same
/// entity as as an [`OrthographicProjection`] component.
///
/// This runs in the [`PreUpdate`] schedule.
///
/// [`PreUpdate`]: bevy::app::PreUpdate
pub fn update_canvas_from_ortho_camera(mut query: Query<(&mut Canvas, &OrthographicProjection)>) {
    trace!("PreUpdate: update_canvas_from_ortho_camera()");
    for (mut canvas, ortho) in query.iter_mut() {
        trace!("ortho canvas rect = {:?}", ortho.area);
        canvas.set_rect(ortho.area);
    }
}

#[derive(Default, Clone, Copy, Component)]
pub struct TileConfig {}

#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct OffsetAndCount {
    /// Base index into [`Tiles::primitives`].
    pub offset: u32,
    /// Number of consecutive primitive offsets in [`Tiles::primitives`].
    pub count: u32,
}

/// Compacted primitive index and kind.
#[derive(Debug, Default, Clone, Copy, Pod, Zeroable)]
#[repr(transparent)]
pub struct PrimitiveIndexAndKind(pub u32);

impl PrimitiveIndexAndKind {
    pub fn new(index: u32, kind: GpuPrimitiveKind, textured: bool) -> Self {
        let textured = (textured as u32) << 31;
        let value = (index & 0x0FFF_FFFF) | (kind as u32) << 28 | textured;
        Self(value)
    }
}

#[derive(Default, Clone, Component)]
pub struct Tiles {
    /// Tile size, in pixels.
    pub(crate) tile_size: UVec2,
    /// Number of tiles.
    ///
    /// 4K, 8x8 => 129'600 tiles
    /// 1080p, 8x8 => 32'400 tiles
    pub(crate) dimensions: UVec2,
    /// Flattened list of primitive indices for each tile. The start of a tile
    /// is at element [`OffsetAndCount::offset`], and the tile contains
    /// [`OffsetAndCount::count`] consecutive primitive offsets, each offset
    /// being the start of the primitive into the primitive buffer of the
    /// canvas.
    pub(crate) primitives: Vec<PrimitiveIndexAndKind>,
    /// Offset and count of primitives per tile, into [`Tiles::primitives`].
    pub(crate) offset_and_count: Vec<OffsetAndCount>,
}

impl Tiles {
    pub fn update_size(&mut self, screen_size: UVec2) {
        // We force a 8x8 pixel tile, which works well with 32- and 64- waves.
        self.tile_size = UVec2::new(8, 8);

        self.dimensions = (screen_size.as_vec2() / self.tile_size.as_vec2())
            .ceil()
            .as_uvec2();

        assert!(self.dimensions.x * self.tile_size.x >= screen_size.x);
        assert!(self.dimensions.y * self.tile_size.y >= screen_size.y);

        self.primitives.clear();
        self.offset_and_count.clear();
        self.offset_and_count
            .reserve(self.dimensions.x as usize * self.dimensions.y as usize);
    }
}

/// Ensure any active [`Camera`] component with a [`Canvas`] component also has
/// associated [`TileConfig`] and [`Tiles`] components.
pub fn add_tiles(
    mut commands: Commands,
    cameras: Query<(Entity, Option<&TileConfig>, &Camera), (With<Canvas>, Without<Tiles>)>,
) {
    for (entity, config, camera) in &cameras {
        if !camera.is_active {
            continue;
        }

        let config = config.copied().unwrap_or_default();
        commands.entity(entity).insert((Tiles::default(), config));
    }
}

pub fn assign_primitives_to_tiles(
    mut views: Query<(
        Entity,
        &Canvas,
        &GlobalTransform,
        &Camera,
        &OrthographicProjection,
        &Frustum,
        &TileConfig,
        &mut Tiles,
    )>,
) {
    // Loop on all camera views
    for (_view_entity, _canvas, _camera_transform, camera, _proj, _frustum, _tile_config, tiles) in
        &mut views
    {
        let Some(screen_size) = camera.physical_viewport_size() else {
            continue;
        };

        // Resize tile storage to fit the viewport size
        let tiles = tiles.into_inner();
        tiles.update_size(screen_size);
    }
}
