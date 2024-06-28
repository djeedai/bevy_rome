use std::sync::Arc;

use bevy::{
    asset::Assets,
    ecs::{
        entity::Entity,
        event::EventReader,
        system::{Local, Query, Res, ResMut},
    },
    image::Image,
    math::Vec2,
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
    sprite::{Anchor, DynamicTextureAtlasBuilder},
    text::{
        CosmicBuffer, CosmicFontSystem, Font, FontAtlasSet, FontSmoothing, GlyphAtlasInfo,
        GlyphAtlasLocation, PositionedGlyph, SwashCache, TextBounds, TextEntity, TextError,
        TextLayoutInfo, TextMeasureInfo,
    },
    utils::{HashMap, HashSet},
    window::{PrimaryWindow, Window, WindowScaleFactorChanged},
};
use cosmic_text::{Attrs, Buffer, CacheKey, Family, Metrics, Shaping, Wrap};
use smallvec::SmallVec;

use crate::Canvas;

/// Extension trait for [`Anchor`].
pub trait AnchorEx {
    /// Convert the [`Anchor`] value to a multiplier [`Vec2`], taking into
    /// account the Y down coordinate system of `bevy_keith` (as opposed to Y up
    /// for Bevy).
    ///
    /// ```
    /// # use crate::*;
    /// # use bevy::sprite::Anchor;
    /// assert_eq!(Anchor::BottomLeft.as_keith_vec(), Vec2::new(-0.5, 0.5));
    /// ```
    fn as_keith_vec(&self) -> Vec2;
}

impl AnchorEx for Anchor {
    fn as_keith_vec(&self) -> Vec2 {
        let mut a = self.as_vec();
        a.y = -a.y;
        a
    }
}

/// Unique global identifier of a text in a [`Canvas`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanvasTextId {
    /// The entity holding the [`Canvas`] component.
    canvas_entity: Entity,
    /// The local index of the text for that canvas.
    text_id: u32,
    // TODO - handle multi-window
}

impl CanvasTextId {
    /// Create a new [`CanvasTextId`] from raw parts.
    pub fn from_raw(canvas_entity: Entity, text_id: u32) -> Self {
        Self {
            canvas_entity,
            text_id,
        }
    }
}

/// Information about a font collected as part of preparing for text layout.
#[derive(Clone)]
struct FontFaceInfo {
    stretch: cosmic_text::fontdb::Stretch,
    style: cosmic_text::fontdb::Style,
    weight: cosmic_text::fontdb::Weight,
    family_name: Arc<str>,
}

/// Computed information for a text block.
///
/// See [`TextLayout`].
///
/// Automatically updated by 2d and UI text systems.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug, Default)]
pub struct ComputedTextBlock {
    /// Buffer for managing text layout and creating [`TextLayoutInfo`].
    ///
    /// This is private because buffer contents are always refreshed from ECS
    /// state when writing glyphs to `TextLayoutInfo`. If you want to
    /// control the buffer contents manually or use the `cosmic-text`
    /// editor, then you need to not use `TextLayout` and instead manually
    /// implement the conversion to `TextLayoutInfo`.
    #[reflect(ignore)]
    pub(crate) buffer: CosmicBuffer,
    /// Entities for all text spans in the block, including the root-level text.
    ///
    /// The [`TextEntity::depth`] field can be used to reconstruct the
    /// hierarchy.
    pub(crate) entities: SmallVec<[TextEntity; 1]>,
    /// Flag set when any change has been made to this block that should cause
    /// it to be rerendered.
    ///
    /// Includes:
    /// - [`TextLayout`] changes.
    /// - [`TextFont`] or `Text2d`/`Text`/`TextSpan` changes anywhere in the
    ///   block's entity hierarchy.
    // TODO: This encompasses both structural changes like font size or justification and
    // non-structural changes like text color and font smoothing. This field currently causes
    // UI to 'remeasure' text, even if the actual changes are non-structural and can be handled
    // by only rerendering and not remeasuring. A full solution would probably require
    // splitting TextLayout and TextFont into structural/non-structural components for more
    // granular change detection. A cost/benefit analysis is needed.
    pub(crate) needs_rerender: bool,
}

impl ComputedTextBlock {
    /// Accesses entities in this block.
    ///
    /// Can be used to look up [`TextFont`] components for glyphs in
    /// [`TextLayoutInfo`] using the `span_index` stored there.
    pub fn entities(&self) -> &[TextEntity] {
        &self.entities
    }

    /// Indicates if the text needs to be refreshed in [`TextLayoutInfo`].
    ///
    /// Updated automatically by [`detect_text_needs_rerender`] and cleared
    /// by [`TextPipeline`](crate::TextPipeline) methods.
    pub fn needs_rerender(&self) -> bool {
        self.needs_rerender
    }
}

impl Default for ComputedTextBlock {
    fn default() -> Self {
        Self {
            buffer: CosmicBuffer::default(),
            entities: SmallVec::default(),
            needs_rerender: true,
        }
    }
}

/// Component with text format settings for a block of text.
///
/// A block of text is composed of text spans, which each have a separate string
/// value and [`TextFont`]. Text spans associated with a text block are
/// collected into [`ComputedTextBlock`] for layout, and then inserted
/// to [`TextLayoutInfo`] for rendering.
///
/// See [`Text2d`](crate::Text2d) for the core component of 2d text, and `Text`
/// in `bevy_ui` for UI text.
#[derive(Component, Debug, Copy, Clone, Default, Reflect)]
#[reflect(Component, Default, Debug)]
#[require(ComputedTextBlock, TextLayoutInfo)]
pub struct TextLayout {
    /// The text's internal alignment.
    /// Should not affect its position within a container.
    pub justify: JustifyText,
    /// How the text should linebreak when running out of the bounds determined
    /// by `max_size`.
    pub linebreak: LineBreak,
}

impl TextLayout {
    /// Makes a new [`TextLayout`].
    pub const fn new(justify: JustifyText, linebreak: LineBreak) -> Self {
        Self { justify, linebreak }
    }

    /// Makes a new [`TextLayout`] with the specified [`JustifyText`].
    pub fn new_with_justify(justify: JustifyText) -> Self {
        Self::default().with_justify(justify)
    }

    /// Makes a new [`TextLayout`] with the specified [`LineBreak`].
    pub fn new_with_linebreak(linebreak: LineBreak) -> Self {
        Self::default().with_linebreak(linebreak)
    }

    /// Makes a new [`TextLayout`] with soft wrapping disabled.
    /// Hard wrapping, where text contains an explicit linebreak such as the
    /// escape sequence `\n`, will still occur.
    pub fn new_with_no_wrap() -> Self {
        Self::default().with_no_wrap()
    }

    /// Returns this [`TextLayout`] with the specified [`JustifyText`].
    pub const fn with_justify(mut self, justify: JustifyText) -> Self {
        self.justify = justify;
        self
    }

    /// Returns this [`TextLayout`] with the specified [`LineBreak`].
    pub const fn with_linebreak(mut self, linebreak: LineBreak) -> Self {
        self.linebreak = linebreak;
        self
    }

    /// Returns this [`TextLayout`] with soft wrapping disabled.
    /// Hard wrapping, where text contains an explicit linebreak such as the
    /// escape sequence `\n`, will still occur.
    pub const fn with_no_wrap(mut self) -> Self {
        self.linebreak = LineBreak::NoWrap;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct CachedGlyphKey {
    pub cache_key: cosmic_text::CacheKey,
    pub font_smoothing: FontSmoothing,
}

/// Workflow:
/// - `glyph_brush_layout::Layout::calculate_glyphs()` calculates the layout of
///   glyphs from text sections.
///   - `glyph_brush_layout::aligned_on_screen()` creates the actual
///     `ab_glyph::Glyph`.
/// - `FontArc::outline_glyph(ab_glyph::Glyph)` converts the glyph outlines into
///   a render-ready format.
/// - `Font::get_outlined_glyph_texture(ab_glyph::OutlinedGlyph)` converts the
///   glyph to texture image.
#[derive(Resource)]
pub struct KeithTextPipeline {
    /// Identifies a font [`ID`](cosmic_text::fontdb::ID) by its [`Font`] asset.
    map_handle_to_font_id: HashMap<AssetId<Font>, (cosmic_text::fontdb::ID, Arc<str>)>,

    /// A mapping between subpixel-offset glyphs and their
    /// [`GlyphAtlasLocation`].
    glyph_to_atlas_index: HashMap<CachedGlyphKey, GlyphAtlasLocation>,

    /// Rectangle packing allocator for the atlas.
    atlas_packer: DynamicTextureAtlasBuilder,

    /// Atlas layout.
    atlas_layout_handle: Handle<TextureAtlasLayout>,

    /// Handle of the atlas texture in `Assets<Image>`.
    // FIXME - Remove this in Bevy 0.14 the dynamic atlas builder doesn't need that deps.
    pub atlas_texture_handle: Handle<Image>,

    /// Buffered vec for collecting spans.
    ///
    /// See [this dark magic](https://users.rust-lang.org/t/how-to-cache-a-vectors-capacity/94478/10).
    spans_buffer: Vec<(usize, &'static str, &'static TextFont, FontFaceInfo)>,
    /// Buffered vec for collecting info for glyph assembly.
    glyph_info: Vec<(AssetId<Font>, FontSmoothing)>,
}

const DEBUG_FILL_ATLAS: bool = true;

impl FromWorld for KeithTextPipeline {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        let atlas_image = if DEBUG_FILL_ATLAS {
            let data: Vec<u8> = (0..1024)
                .map(|y| {
                    (0..1024)
                        .map(move |x| [(x / 4) as u8, (y / 4) as u8, 255u8, 255u8])
                        .flatten()
                })
                .flatten()
                .collect();
            Image::new(
                Extent3d {
                    width: 1024,
                    height: 1024,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                TextureFormat::Rgba8Unorm,
                // Need access from main world to update below, and render world to actually render
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            )
        } else {
            Image::new_fill(
                Extent3d {
                    width: 1024,
                    height: 1024,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[0, 0, 0, 0],
                TextureFormat::Rgba8Unorm,
                // Need access from main world to update below, and render world to actually render
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            )
        };
        let atlas_texture_handle = images.add(atlas_image);

        let mut texture_atlas_layouts = world.resource_mut::<Assets<TextureAtlasLayout>>();
        let initial_size = UVec2::splat(1024);
        let atlas_layout_handle =
            texture_atlas_layouts.add(TextureAtlasLayout::new_empty(initial_size));

        Self {
            map_handle_to_font_id: default(),
            glyph_to_atlas_index: default(),
            // The whole Bevy text pipeline relies on padding==1, and will insert blank spaces as
            // non-zero rects into the atlas thanks to that, which would otherwise cause some error
            // as there's no code to skip those.
            atlas_packer: DynamicTextureAtlasBuilder::new(initial_size, 1),
            atlas_layout_handle,
            atlas_texture_handle,
            spans_buffer: vec![],
            glyph_info: vec![],
        }
    }
}

impl KeithTextPipeline {
    /// Utilizes [`cosmic_text::Buffer`] to shape and layout text
    ///
    /// Negative or 0.0 font sizes will not be laid out.
    #[allow(clippy::too_many_arguments)]
    pub fn update_buffer<'a>(
        &mut self,
        fonts: &Assets<Font>,
        text_spans: impl Iterator<Item = (Entity, usize, &'a str, &'a TextFont, Color)>,
        linebreak: LineBreak,
        justify: JustifyText,
        bounds: TextBounds,
        scale_factor: f64,
        computed: &mut ComputedTextBlock,
        font_system: &mut CosmicFontSystem,
    ) -> Result<(), TextError> {
        let font_system = &mut font_system.0;

        // Collect span information into a vec. This is necessary because font loading
        // requires mut access to FontSystem, which the cosmic-text Buffer also
        // needs.
        let mut font_size: f32 = 0.;
        let mut spans: Vec<(usize, &str, &TextFont, FontFaceInfo, Color)> =
            core::mem::take(&mut self.spans_buffer)
                .into_iter()
                .map(|_| -> (usize, &str, &TextFont, FontFaceInfo, Color) { unreachable!() })
                .collect();

        computed.entities.clear();

        for (span_index, (entity, depth, span, text_font, color)) in text_spans.enumerate() {
            // Save this span entity in the computed text block.
            computed.entities.push(TextEntity { entity, depth });

            if span.is_empty() {
                continue;
            }

            // Return early if a font is not loaded yet.
            if !fonts.contains(text_font.font.id()) {
                spans.clear();
                self.spans_buffer = spans
                    .into_iter()
                    .map(
                        |_| -> (usize, &'static str, &'static TextFont, FontFaceInfo) {
                            unreachable!()
                        },
                    )
                    .collect();

                return Err(TextError::NoSuchFont);
            }

            // Get max font size for use in cosmic Metrics.
            font_size = font_size.max(text_font.font_size);

            // Skip spans that are zero-sized.
            if scale_factor <= 0.0 || text_font.font_size <= 0.0 {
                continue;
            }

            // Load Bevy fonts into cosmic-text's font system.
            let face_info = load_font_to_fontdb(
                text_font,
                font_system,
                &mut self.map_handle_to_font_id,
                fonts,
            );

            spans.push((span_index, span, text_font, face_info, color));
        }

        let line_height = font_size * 1.2;
        let mut metrics = Metrics::new(font_size, line_height).scale(scale_factor as f32);
        // Metrics of 0.0 cause `Buffer::set_metrics` to panic. We hack around this by
        // 'falling through' to call `Buffer::set_rich_text` with zero spans so
        // any cached text will be cleared without deallocating the buffer.
        metrics.font_size = metrics.font_size.max(0.000001);
        metrics.line_height = metrics.line_height.max(0.000001);

        // Map text sections to cosmic-text spans, and ignore sections with negative or
        // zero fontsizes, since they cannot be rendered by cosmic-text.
        //
        // The section index is stored in the metadata of the spans, and could be used
        // to look up the section the span came from and is not used internally
        // in cosmic-text.
        let spans_iter = spans
            .iter()
            .map(|(span_index, span, text_font, font_info, color)| {
                (
                    *span,
                    get_attrs(*span_index, text_font, *color, font_info, scale_factor),
                )
            });

        // Update the buffer.
        let buffer = &mut computed.buffer;
        buffer.set_metrics(font_system, metrics);
        buffer.set_size(font_system, bounds.width, bounds.height);

        buffer.set_wrap(
            font_system,
            match linebreak {
                LineBreak::WordBoundary => Wrap::Word,
                LineBreak::AnyCharacter => Wrap::Glyph,
                LineBreak::WordOrCharacter => Wrap::WordOrGlyph,
                LineBreak::NoWrap => Wrap::None,
            },
        );

        buffer.set_rich_text(font_system, spans_iter, Attrs::new(), Shaping::Advanced);

        // PERF: https://github.com/pop-os/cosmic-text/issues/166:
        // Setting alignment afterwards appears to invalidate some layouting performed
        // by `set_text` which is presumably not free?
        for buffer_line in buffer.lines.iter_mut() {
            buffer_line.set_align(Some(justify.into()));
        }
        buffer.shape_until_scroll(font_system, false);

        // Recover the spans buffer.
        spans.clear();
        self.spans_buffer = spans
            .into_iter()
            .map(|_| -> (usize, &'static str, &'static TextFont, FontFaceInfo) { unreachable!() })
            .collect();

        Ok(())
    }

    /// Queues text for rendering
    ///
    /// Produces a [`TextLayoutInfo`], containing [`PositionedGlyph`]s
    /// which contain information for rendering the text.
    #[allow(clippy::too_many_arguments)]
    pub fn queue_text<'a>(
        &mut self,
        layout_info: &mut TextLayoutInfo,
        fonts: &Assets<Font>,
        text_spans: impl Iterator<Item = (Entity, usize, &'a str, &'a TextFont, Color)>,
        scale_factor: f64,
        layout: &TextLayout,
        bounds: TextBounds,
        texture_atlases: &mut Assets<TextureAtlasLayout>,
        textures: &mut Assets<Image>,
        computed: &mut ComputedTextBlock,
        font_system: &mut CosmicFontSystem,
        swash_cache: &mut SwashCache,
    ) -> Result<(), TextError> {
        layout_info.glyphs.clear();
        layout_info.size = Default::default();

        // Clear this here at the focal point of text rendering to ensure the field's
        // lifecycle has strong boundaries
        computed.needs_rerender = false;

        // Extract font IDs for all spans
        let mut glyph_info = core::mem::take(&mut self.glyph_info);
        glyph_info.clear();
        let text_spans = text_spans.inspect(|(_, _, _, text_font, _)| {
            glyph_info.push((text_font.font.id(), text_font.font_smoothing));
        });

        // Calculate the shapes and layout of the text with Cosmic
        let update_result = self.update_buffer(
            fonts,
            text_spans,
            layout.linebreak,
            layout.justify,
            bounds,
            scale_factor,
            computed,
            font_system,
        );
        if let Err(err) = update_result {
            self.glyph_info = glyph_info;
            return Err(err);
        }

        // Compute the dimensions of the box containing the (multi-line) text
        let buffer = &mut computed.buffer;
        let box_size = buffer_dimensions(buffer);

        let result = buffer
            .layout_runs()
            .flat_map(|run| {
                run.glyphs
                    .iter()
                    .map(move |layout_glyph| (layout_glyph, run.line_y))
            })
            .try_for_each(|(layout_glyph, line_y)| {
                let mut temp_glyph;
                let span_index = layout_glyph.metadata;
                let font_id = glyph_info[span_index].0;
                let font_smoothing = glyph_info[span_index].1;

                let layout_glyph = if font_smoothing == FontSmoothing::None {
                    // If font smoothing is disabled, round the glyph positions and sizes,
                    // effectively discarding all subpixel layout.
                    temp_glyph = layout_glyph.clone();
                    temp_glyph.x = temp_glyph.x.round();
                    temp_glyph.y = temp_glyph.y.round();
                    temp_glyph.w = temp_glyph.w.round();
                    temp_glyph.x_offset = temp_glyph.x_offset.round();
                    temp_glyph.y_offset = temp_glyph.y_offset.round();
                    temp_glyph.line_height_opt = temp_glyph.line_height_opt.map(f32::round);

                    &temp_glyph
                } else {
                    layout_glyph
                };

                let physical_glyph = layout_glyph.physical((0., 0.), 1.);

                let atlas_info = self
                    .get_glyph_atlas_info(&physical_glyph.cache_key, font_id, font_smoothing)
                    .map(Ok)
                    .unwrap_or_else(|| {
                        self.add_glyph_to_atlas(
                            texture_atlases,
                            textures,
                            &mut font_system.0,
                            &mut swash_cache.0,
                            layout_glyph,
                            font_smoothing,
                        )
                    })?;

                let texture_atlas = texture_atlases.get(&atlas_info.texture_atlas).unwrap();
                let location = atlas_info.location;
                let glyph_rect = texture_atlas.textures[location.glyph_index];
                let left = location.offset.x as f32;
                let top = location.offset.y as f32;
                let glyph_size = UVec2::new(glyph_rect.width(), glyph_rect.height());

                // offset by half the size because the origin is center
                let x = left + physical_glyph.x as f32; // + glyph_size.x as f32 / 2.0;
                let y = line_y.round() + physical_glyph.y as f32 - top; // + glyph_size.y as f32;// / 2.0;
                let position = Vec2::new(x, y);

                // TODO: recreate the byte index, that keeps track of where a cursor is,
                // when glyphs are not limited to single byte representation, relevant for #1319
                let pos_glyph =
                    PositionedGlyph::new(position, glyph_size.as_vec2(), atlas_info, span_index);
                layout_info.glyphs.push(pos_glyph);
                Ok(())
            });

        // Return the scratch vec.
        self.glyph_info = glyph_info;

        // Check result.
        result?;

        layout_info.size = box_size;
        Ok(())
    }

    /// Queues text for measurement
    ///
    /// Produces a [`TextMeasureInfo`] which can be used by a layout system
    /// to measure the text area on demand.
    #[allow(clippy::too_many_arguments)]
    pub fn create_text_measure<'a>(
        &mut self,
        entity: Entity,
        fonts: &Assets<Font>,
        text_spans: impl Iterator<Item = (Entity, usize, &'a str, &'a TextFont, Color)>,
        scale_factor: f64,
        layout: &TextLayout,
        computed: &mut ComputedTextBlock,
        font_system: &mut CosmicFontSystem,
    ) -> Result<TextMeasureInfo, TextError> {
        const MIN_WIDTH_CONTENT_BOUNDS: TextBounds = TextBounds::new_horizontal(0.0);

        // Clear this here at the focal point of measured text rendering to ensure the
        // field's lifecycle has strong boundaries.
        computed.needs_rerender = false;

        self.update_buffer(
            fonts,
            text_spans,
            layout.linebreak,
            layout.justify,
            MIN_WIDTH_CONTENT_BOUNDS,
            scale_factor,
            computed,
            font_system,
        )?;

        let buffer = &mut computed.buffer;
        let min_width_content_size = buffer_dimensions(buffer);

        let max_width_content_size = {
            let font_system = &mut font_system.0;
            buffer.set_size(font_system, None, None);
            buffer_dimensions(buffer)
        };

        Ok(TextMeasureInfo {
            min: min_width_content_size,
            max: max_width_content_size,
            entity,
        })
    }

    /// Returns the [`cosmic_text::fontdb::ID`] for a given [`Font`] asset.
    pub fn get_font_id(&self, asset_id: AssetId<Font>) -> Option<cosmic_text::fontdb::ID> {
        self.map_handle_to_font_id
            .get(&asset_id)
            .cloned()
            .map(|(id, _)| id)
    }

    fn get_glyph_atlas_info(
        &mut self,
        cache_key: &CacheKey,
        font_id: AssetId<Font>,
        font_smoothing: FontSmoothing,
    ) -> Option<GlyphAtlasInfo> {
        assert!(self.map_handle_to_font_id.get(&font_id).is_some());
        self.glyph_to_atlas_index
            .get(&CachedGlyphKey {
                cache_key: *cache_key,
                font_smoothing,
            })
            .map(|location| GlyphAtlasInfo {
                texture: self.atlas_texture_handle.clone(),
                texture_atlas: self.atlas_layout_handle.clone(),
                location: *location,
            })
    }

    fn add_glyph_to_atlas(
        &mut self,
        atlas_layouts: &mut Assets<TextureAtlasLayout>,
        images: &mut Assets<Image>,
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
        layout_glyph: &cosmic_text::LayoutGlyph,
        font_smoothing: FontSmoothing,
    ) -> Result<GlyphAtlasInfo, TextError> {
        // Render the glyph to a texture image
        let physical_glyph = layout_glyph.physical((0., 0.), 1.0);
        let (glyph_texture, offset) = FontAtlasSet::get_outlined_glyph_texture(
            font_system,
            swash_cache,
            &physical_glyph,
            font_smoothing,
        )?;
        if glyph_texture.size() == UVec2::ZERO {
            // blank space etc. have no visual representation; skip them
        }

        // Pack the glyph's texture into the atlas
        let atlas_layout = atlas_layouts.get_mut(&self.atlas_layout_handle).unwrap();
        let atlas_texture = images.get_mut(&self.atlas_texture_handle).unwrap();
        let Some(glyph_index) =
            self.atlas_packer
                .add_texture(atlas_layout, &glyph_texture, atlas_texture)
        else {
            return Err(TextError::FailedToAddGlyph(layout_glyph.glyph_id));
        };
        let location = GlyphAtlasLocation {
            glyph_index,
            offset,
        };
        self.glyph_to_atlas_index.insert(
            CachedGlyphKey {
                cache_key: physical_glyph.cache_key,
                font_smoothing,
            },
            location,
        );

        Ok(GlyphAtlasInfo {
            texture: self.atlas_texture_handle.clone(),
            texture_atlas: self.atlas_layout_handle.clone(),
            location,
        })
    }
}

/// System running during the [`PostUpdate`] schedule of the main app to
/// process the glyphs of all texts of all [`Canvas`] components.
///
/// The system processes all glyphs of all drawn texts, and inserts the newly
/// needed glyph images into the texture atlas(es) used for later text
/// rendering.
///
/// It takes into account the scaling of the window the canvas is rendered onto,
/// adapting to scale changes.
///
/// [`PostUpdate`]: bevy::app::PostUpdate
pub fn process_glyphs(
    // Text items which should be reprocessed again, generally when the font hasn't loaded yet.
    // Mapped from the Entity containing the Canvas that owns the text.
    mut font_queue: Local<HashSet<Entity>>,
    mut images: ResMut<Assets<Image>>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    fonts: Res<Assets<Font>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    mut ev_window_scale_factor_changed: EventReader<WindowScaleFactorChanged>,
    //mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
    //mut font_atlas_set_storage: ResMut<FontAtlasSets>,
    mut text_pipeline: ResMut<KeithTextPipeline>,
    mut canvas_query: Query<(Entity, &mut Canvas)>,
    //text_settings: Res<TextSettings>,
    mut font_system: ResMut<CosmicFontSystem>,
    mut swash_cache: ResMut<SwashCache>,
) {
    trace!("process_glyphs");

    // We need to consume the entire iterator, hence `last`
    let scale_factor_changed = ev_window_scale_factor_changed.read().last().is_some();

    // TODO - handle multi-window
    let Ok(window) = q_window.get_single() else {
        return;
    };
    let scale_factor = window.scale_factor() as f64;
    let inv_scale_factor = 1. / scale_factor;

    // Loop on all existing canvases
    for (entity, mut canvas) in canvas_query.iter_mut() {
        // Check for something to do, if any of:
        // - the window scale factor changed
        // - the canvas has some texts
        // - any font not previously loaded is maybe now available
        if !scale_factor_changed && !canvas.has_text() && !font_queue.remove(&entity) {
            continue;
        }

        // Loop on all texts for the current canvas
        for text_layout in canvas.text_layouts_mut() {
            // Update the text glyphs, storing them into the font atlas(es) for later
            // rendering
            trace!(
                "Queue text: id={} anchor={:?} alignment={:?} bounds={:?}",
                text_layout.id,
                text_layout.anchor,
                text_layout.justify,
                text_layout.bounds
            );

            text_layout.bounds.width = text_layout.bounds.width.map(|w| w * scale_factor as f32);
            text_layout.bounds.height = text_layout.bounds.height.map(|h| h * scale_factor as f32);

            let mut text_layout_info = TextLayoutInfo::default();
            let mut computed = ComputedTextBlock::default();
            match text_pipeline.queue_text(
                &mut text_layout_info,
                &fonts,
                text_layout
                    .sections
                    .iter()
                    .map(|s| (s.entity, s.depth, &s.text[..], &s.font, s.color)),
                scale_factor,
                &TextLayout::new_with_justify(text_layout.justify),
                text_layout.bounds,
                &mut texture_atlas_layouts,
                &mut images,
                &mut computed,
                &mut font_system,
                &mut swash_cache,
            ) {
                Ok(()) => {
                    text_layout.calculated_size = Vec2::new(
                        scale_value(text_layout_info.size.x, inv_scale_factor),
                        scale_value(text_layout_info.size.y, inv_scale_factor),
                    );
                    text_layout.layout_info = Some(text_layout_info);
                }
                Err(TextError::NoSuchFont) => {
                    // There was an error processing the text layout, let's add this entity to the
                    // queue for further processing
                    font_queue.insert(entity);
                }
                Err(text_error) => error!("Failed to calculate layout for text: {:?}", text_error),
            }
        }
    }
}

pub(crate) fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}

fn load_font_to_fontdb(
    text_font: &TextFont,
    font_system: &mut cosmic_text::FontSystem,
    map_handle_to_font_id: &mut HashMap<AssetId<Font>, (cosmic_text::fontdb::ID, Arc<str>)>,
    fonts: &Assets<Font>,
) -> FontFaceInfo {
    let font_handle = text_font.font.clone();
    let (face_id, family_name) = map_handle_to_font_id
        .entry(font_handle.id())
        .or_insert_with(|| {
            let font = fonts.get(font_handle.id()).expect(
                "Tried getting a font that was not available, probably due to not being loaded yet",
            );
            let data = Arc::clone(&font.data);
            let ids = font_system
                .db_mut()
                .load_font_source(cosmic_text::fontdb::Source::Binary(data));

            // TODO: it is assumed this is the right font face
            let face_id = *ids.last().unwrap();
            let face = font_system.db().face(face_id).unwrap();
            let family_name = Arc::from(face.families[0].0.as_str());

            (face_id, family_name)
        });
    let face = font_system.db().face(*face_id).unwrap();

    FontFaceInfo {
        stretch: face.stretch,
        style: face.style,
        weight: face.weight,
        family_name: family_name.clone(),
    }
}

/// Translates [`TextFont`] to [`Attrs`].
fn get_attrs<'a>(
    span_index: usize,
    text_font: &TextFont,
    color: Color,
    face_info: &'a FontFaceInfo,
    scale_factor: f64,
) -> Attrs<'a> {
    let attrs = Attrs::new()
        .metadata(span_index)
        .family(Family::Name(&face_info.family_name))
        .stretch(face_info.stretch)
        .style(face_info.style)
        .weight(face_info.weight)
        .metrics(Metrics::relative(text_font.font_size, 1.2).scale(scale_factor as f32))
        .color(cosmic_text::Color(color.to_linear().as_u32()));
    attrs
}

/// Calculate the size of the text area for the given buffer.
fn buffer_dimensions(buffer: &Buffer) -> Vec2 {
    let (width, height) = buffer
        .layout_runs()
        .map(|run| (run.line_w, run.line_height))
        .reduce(|(w1, h1), (w2, h2)| (w1.max(w2), h1 + h2))
        .unwrap_or((0.0, 0.0));

    Vec2::new(width, height).ceil()
}
