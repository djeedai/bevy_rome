use ab_glyph::{Font as _, ScaleFont as _};
use bevy::{
    asset::Assets,
    ecs::{
        entity::Entity,
        event::EventReader,
        system::{Local, Query, Res, ResMut},
    },
    math::Vec2,
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::Image,
    },
    sprite::DynamicTextureAtlasBuilder,
    text::{BreakLineOn, Font, GlyphAtlasInfo, PositionedGlyph, TextError, TextLayoutInfo},
    utils::{FloatOrd, HashMap, HashSet},
    window::{PrimaryWindow, Window, WindowScaleFactorChanged},
};
use glyph_brush_layout::GlyphPositioner as _;

use crate::{render_context::TextLayout, Canvas};

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

    /// Get the text local index as an array index (`usize`).
    pub(crate) fn index(&self) -> usize {
        self.text_id as usize
    }
}

/// A glyph cached inside an atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ScaledGlyph {
    /// ID of the glyph in the font.
    pub glyph_id: ab_glyph::GlyphId,
    /// Font size, in pixels.
    pub font_size: FloatOrd,
}

#[derive(Debug, Clone, Copy)]
struct AtlasGlyph {
    /// Index of the glyph into the [`TextureAtlasLayout`].
    pub glyph_index: usize,

    /// Typographic bounds relative to the glyph origin ("pen position").
    /// `bounds.min` represents the offset from the lower left corner of the glyph texture stored in the atlas to the glyph origin.
    pub bounds: Rect,

    /// Size of the glyph texture, in pixels.
    pub px_size: Vec2,
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
    /// Map from a Bevy font handle to an internal font ID of the layouter.
    font_map: HashMap<AssetId<Font>, glyph_brush_layout::FontId>,

    /// Fonts in use in the atlas. Handles are strong to keep the font alive.
    font_handles: Vec<Handle<Font>>,

    /// Fonts in use in the atlas.
    fonts: Vec<ab_glyph::FontArc>,

    /// Map from a glyph to its index in the atlas.
    glyphs: HashMap<ScaledGlyph, AtlasGlyph>,

    /// Rectangle packing allocator for the atlas.
    atlas_packer: DynamicTextureAtlasBuilder,

    /// Atlas layout.
    atlas_layout_handle: Handle<TextureAtlasLayout>,

    /// Handle of the atlas texture in `Assets<Image>`.
    // FIXME - Remove this in Bevy 0.14 the dynamic atlas builder doesn't need that deps.
    pub atlas_texture_handle: Handle<Image>,
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
        let atlas_layout_handle =
            texture_atlas_layouts.add(TextureAtlasLayout::new_empty(Vec2::splat(1024.)));

        let initial_size = Vec2::splat(1024.0);
        Self {
            font_map: default(),
            font_handles: vec![],
            fonts: vec![],
            glyphs: default(),
            atlas_packer: DynamicTextureAtlasBuilder::new(initial_size, 0),
            atlas_layout_handle,
            atlas_texture_handle,
        }
    }
}

impl KeithTextPipeline {
    pub fn calc_layout(
        &mut self,
        fonts: &Assets<Font>,
        images: &mut Assets<Image>,
        texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
        text_layout: &mut TextLayout,
    ) -> Result<TextLayoutInfo, TextError> {
        trace!("calc_layout()");

        let scale_factor = 1.; // TODO

        let atlas_layout = texture_atlas_layouts
            .get_mut(&self.atlas_layout_handle)
            .unwrap();

        // Resolve all fonts for all sections of the input text, and map those sections
        // to internal SectionText for glyph_brush_layout
        let mut scaled_fonts = Vec::with_capacity(text_layout.sections.len());
        let sections = text_layout
            .sections
            .iter()
            .map(|section| {
                let font = fonts
                    .get(&section.style.font)
                    .ok_or(TextError::NoSuchFont)?;

                let font_id = self.get_or_insert_font_id(&section.style.font, font);
                let font_size = scale_value(section.style.font_size, scale_factor);

                scaled_fonts.push(ab_glyph::Font::as_scaled(&font.font, font_size));

                let section = glyph_brush_layout::SectionText {
                    text: &section.value,
                    scale: ab_glyph::PxScale::from(font_size),
                    font_id,
                };

                Ok(section)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Layout all glyphs with glyph_brush_layout
        let geom = glyph_brush_layout::SectionGeometry {
            bounds: (text_layout.bounds.x, text_layout.bounds.y),
            ..Default::default()
        };
        let line_breaker: glyph_brush_layout::BuiltInLineBreaker = BreakLineOn::NoWrap.into();
        let section_glyphs = glyph_brush_layout::Layout::default()
            .h_align(text_layout.alignment.into())
            .line_breaker(line_breaker) // TODO - could make custom
            .calculate_glyphs(&self.fonts, &geom, &sections);

        // Copied from Bevy...
        let logical_size =
            Self::calc_logical_size(&section_glyphs, |index| scaled_fonts[index]).size();

        // Raster all glyphs and insert them into the atlas
        let mut text_layout_info = TextLayoutInfo {
            logical_size,
            ..default()
        };
        for section_glyph in section_glyphs {
            let glyph_brush_layout::SectionGlyph {
                section_index,
                byte_index,
                glyph,
                font_id,
            } = section_glyph;

            let position = Vec2::new(glyph.position.x, glyph.position.y);
            let scale = Vec2::new(glyph.scale.x, glyph.scale.y);

            let section = sections[section_index];
            let font_size = section.scale.y.round(); // FIXME - simple hack to avoid many glyphs of "about" the same size
            let scaled_glyph = ScaledGlyph {
                glyph_id: glyph.id,
                font_size: FloatOrd(font_size),
            };

            trace!(
                "Glyph #{:?} pos={:?} scale={:?} font_id={:?} font_size={:?}",
                glyph.id,
                position,
                scale,
                font_id,
                font_size
            );

            // Resolve glyph in atlas
            let atlas_glyph = if let Some(atlas_glyph) = self.glyphs.get(&scaled_glyph) {
                trace!(
                    "  -> Already present in atlas at index #{} (px_size:{:?})",
                    atlas_glyph.glyph_index,
                    atlas_glyph.px_size,
                );
                *atlas_glyph
            } else {
                let glyph_id = glyph.id;

                // Glyph not present in atlas, adding it now
                if let Some(outlined_glyph) = self.fonts[section.font_id.0].outline_glyph(glyph) {
                    // Get the rectangle bounds of this glyph. This is the rectangle centered at the "pen position", from which all typographic quantities like h-advance and ascent/descent are calculated. Generally bounds.min is small but non-zero (especially if there's a descent).
                    let bounds = outlined_glyph.px_bounds();

                    // Raster the glyph into an Image
                    let glyph_texture = Font::get_outlined_glyph_texture(outlined_glyph);

                    // Place the glyph into the atlas if needed, and get back info about where
                    let Some(glyph_index) = self.atlas_packer.add_texture(
                        atlas_layout,
                        images,
                        &glyph_texture,
                        &self.atlas_texture_handle,
                    ) else {
                        warn!("Atlas full!");
                        continue;
                    };

                    let tex_rect = atlas_layout.textures[glyph_index];

                    // Bounds are the pixel-rounded position where we should draw the texture, relative to the origin
                    // of the entire section. glyph.position contains the origin of the glyph itself. To reuse the glyphs,
                    // we store relative bounds, and ignore the sub-pixel delta between multiple glyph instances.
                    let mut bounds =
                        Rect::new(bounds.min.x, bounds.min.y, bounds.max.x, bounds.max.y);
                    bounds.min.x -= position.x;
                    bounds.min.y -= position.y;
                    bounds.max.x -= position.x;
                    bounds.max.y -= position.y;

                    let px_size = tex_rect.size();
                    let atlas_glyph = AtlasGlyph {
                        glyph_index,
                        bounds,
                        px_size,
                    };

                    self.glyphs.insert(scaled_glyph, atlas_glyph);
                    debug!("  -> Inserted new glyph #{glyph_id:?} at index {glyph_index} into atlas. bounds={bounds:?} (px_size:{px_size:?})");

                    atlas_glyph
                } else {
                    // This generally happens for e.g. the blank space character, which has no glyph.
                    continue;
                }
            };

            let size = atlas_glyph.px_size;

            // Restore glyph position from glyph origin relative to section origin + glyph offset from its own origin
            let mut position = Vec2::new(
                position.x + atlas_glyph.bounds.min.x,
                position.y + atlas_glyph.bounds.min.y,
            );

            // ab_glyph always inserts a 1-pixel padding around glyphs it rasterizes, so the actual texture is larger
            position -= 1.0;

            text_layout_info.glyphs.push(PositionedGlyph {
                position,
                size,
                atlas_info: GlyphAtlasInfo {
                    texture_atlas: self.atlas_layout_handle.clone(),
                    texture: self.atlas_texture_handle.clone(),
                    glyph_index: atlas_glyph.glyph_index,
                },
                section_index,
                byte_index,
            });
        }

        return Ok(text_layout_info);
    }

    fn get_or_insert_font_id(
        &mut self,
        handle: &Handle<Font>,
        font: &Font,
    ) -> glyph_brush_layout::FontId {
        *self.font_map.entry(handle.id()).or_insert_with(|| {
            let id = self.fonts.len();
            self.fonts.push(font.font.clone());
            self.font_handles.push(handle.clone());
            glyph_brush_layout::FontId(id)
        })
    }

    // Copied from Bevy...
    /// Calculate the logical size of a text section.
    ///
    /// Note that the size includes some small padding corresponding to the bearings around the glyph.
    /// This is because the resulting size is aimed at anchoring the text, and therefore needs to account
    /// for the full typographical size of the glyph, which is visually more pleasing than the tight pixel
    /// bounds of the rasterized glyph.
    fn calc_logical_size<T>(
        section_glyphs: &[glyph_brush_layout::SectionGlyph],
        get_scaled_font: impl Fn(usize) -> ab_glyph::PxScaleFont<T>,
    ) -> Rect
    where
        T: ab_glyph::Font,
    {
        let mut text_bounds = Rect {
            min: Vec2::splat(f32::MAX),
            max: Vec2::splat(f32::MIN),
        };

        for sg in section_glyphs {
            let scaled_font = get_scaled_font(sg.section_index);
            let glyph = &sg.glyph;
            text_bounds = text_bounds.union(Rect {
                // FIXME - This 0.0 is slightly incorrect, only works because in general position.y == ascent
                min: Vec2::new(glyph.position.x, 0.),
                max: Vec2::new(
                    glyph.position.x + scaled_font.h_advance(glyph.id),
                    // Descent is below the baseline, which is what the position references.
                    // So we need to add it. And it's negative so we subtract to get its size.
                    glyph.position.y - scaled_font.descent(),
                ),
            });
        }

        text_bounds
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
                text_layout.alignment,
                text_layout.bounds
            );

            match text_pipeline.calc_layout(
                &fonts,
                &mut images,
                &mut texture_atlas_layouts,
                text_layout,
            ) {
                Ok(text_layout_info) => {
                    text_layout.calculated_size = Vec2::new(
                        scale_value(text_layout_info.logical_size.x, inv_scale_factor),
                        scale_value(text_layout_info.logical_size.y, inv_scale_factor),
                    );
                    text_layout.layout_info = Some(text_layout_info);
                }
                Err(text_error) => error!("Failed to calculate layout for text: {:?}", text_error),
            }

            // match text_pipeline.queue_text(
            //     &fonts,
            //     &text.sections,
            //     scale_factor as f32,
            //     text.alignment,
            //     BreakLineOn::WordBoundary, // TODO - configurable
            //     text.bounds * scale_factor as f32,
            //     &mut font_atlas_set_storage,
            //     &mut texture_atlases,
            //     &mut images,
            //     text_settings.as_ref(),
            //     YAxisOrientation::BottomToTop, // TODO - configurable
            // ) {
            //     Err(TextError::NoSuchFont) => {
            //         // There was an error looking for the text font, add the canvas entity to the
            //         // queue for later re-try (next frame)
            //         font_queue.insert(entity);

            //         text.layout_info = None;
            //     }
            //     Err(e @ TextError::FailedToAddGlyph(_)) => {
            //         panic!("Fatal error when processing text: {}.", e);
            //     }
            //     Ok(text_layout_info) => {
            //         text.calculated_size = Vec2::new(
            //             scale_value(text_layout_info.logical_size.x, inv_scale_factor),
            //             scale_value(text_layout_info.logical_size.y, inv_scale_factor),
            //         );
            //         text.layout_info = Some(text_layout_info);
            //     }
            // }
        }
    }
}

pub(crate) fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}
