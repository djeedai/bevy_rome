use bevy::{
    asset::Assets,
    ecs::{
        entity::Entity,
        event::EventReader,
        query::Changed,
        system::{Local, Query, Res, ResMut},
    },
    math::{Size, Vec2},
    render::texture::Image,
    sprite::TextureAtlas,
    text::{Font, FontAtlasSet, TextAlignment, TextError, TextPipeline, TextSection},
    utils::HashSet,
    window::{WindowId, WindowScaleFactorChanged, Windows},
};

use crate::Canvas;

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

pub type KeithTextPipeline = TextPipeline<CanvasTextId>;

/// System running during the [`CoreStage::PostUpdate`] stage of the main app to process
/// the glyphs of all texts of all [`Canvas`] components.
///
/// The system processes all glyphs of all drawn texts, and inserts the newly needed glyph
/// images into the texture atlas(es) used for later text rendering.
///
/// It takes into account the scaling of the window the canvas is rendered onto, adapting
/// to scale changes.
///
/// [`CoreStage::PostUpdate`]: bevy::app::CoreStage::PostUpdate
pub fn process_glyphs(
    // Text items which should be reprocessed again, generally when the font hasn't loaded yet.
    // Mapped from the Entity containing the Canvas that owns the text.
    mut font_queue: Local<HashSet<Entity>>,
    mut textures: ResMut<Assets<Image>>,
    fonts: Res<Assets<Font>>,
    windows: Res<Windows>,
    mut scale_factor_changed: EventReader<WindowScaleFactorChanged>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut font_atlas_set_storage: ResMut<Assets<FontAtlasSet>>,
    mut text_pipeline: ResMut<KeithTextPipeline>,
    mut canvas_query: Query<(Entity, &mut Canvas)>,
) {
    // We need to consume the entire iterator, hence `last`
    let factor_changed = scale_factor_changed.iter().last().is_some();

    // TODO - handle multi-window
    let scale_factor = windows.scale_factor(WindowId::primary());

    // Loop on all existing canvases
    for (entity, mut canvas) in canvas_query.iter_mut() {
        // Check for something to do, if any of:
        // - the window scale factor changed
        // - any of the texts of the canvas changed
        // - any font not previously loaded is maybe now available
        if !factor_changed && !canvas.text_changed() && !font_queue.remove(&entity) {
            continue;
        }

        // let text_bounds = match maybe_bounds {
        //     Some(bounds) => Vec2::new(
        //         scale_value(bounds.size.x, scale_factor),
        //         scale_value(bounds.size.y, scale_factor),
        //     ),
        //     None => Vec2::new(f32::MAX, f32::MAX),
        // };

        // Loop on all texts for the current canvas
        for text in canvas.text_layouts() {
            let text_id = CanvasTextId::from_raw(entity, text.id);

            // Update the text glyphs, storing them into the font atlas(es) for later rendering
            match text_pipeline.queue_text(
                text_id,
                &fonts,
                &text.sections,
                scale_factor,
                text.alignment,
                text.bounds,
                &mut *font_atlas_set_storage,
                &mut *texture_atlases,
                &mut *textures,
            ) {
                Err(TextError::NoSuchFont) => {
                    // There was an error looking for the text font, add the canvas entity to the
                    // queue for later re-try (next frame)
                    font_queue.insert(entity);
                }
                Err(e @ TextError::FailedToAddGlyph(_)) => {
                    panic!("Fatal error when processing text: {}.", e);
                }
                Ok(()) => {
                    let _text_layout_info = text_pipeline.get_glyphs(&text_id).expect(
                        "Failed to get glyphs from the pipeline that have just been computed",
                    );
                    // calculated_size.size = Vec2::new(
                    //     scale_value(text_layout_info.size.x, 1. / scale_factor),
                    //     scale_value(text_layout_info.size.y, 1. / scale_factor),
                    // );
                }
            }
        }
    }
}

// pub fn extract_text_primitives(
//     mut render_world: ResMut<RenderWorld>,
//     texture_atlases: Res<Assets<TextureAtlas>>,
//     text_pipeline: Res<KeithTextPipeline>,
//     windows: Res<Windows>,
//     text2d_query: Query<(Entity, &Visibility, &Text, &GlobalTransform, &Text2dSize)>,
// ) {
//     let mut extracted_sprites = render_world.resource_mut::<ExtractedSprites>();

//     let scale_factor = windows.scale_factor(WindowId::primary()) as f32;

//     for (entity, visibility, text, transform, calculated_size) in text2d_query.iter() {
//         if !visibility.is_visible {
//             continue;
//         }
//         let (width, height) = (calculated_size.size.x, calculated_size.size.y);

//         if let Some(text_layout) = text_pipeline.get_glyphs(&entity) {
//             let text_glyphs = &text_layout.glyphs;
//             let alignment_offset = match text.alignment.vertical {
//                 VerticalAlign::Top => Vec3::new(0.0, -height, 0.0),
//                 VerticalAlign::Center => Vec3::new(0.0, -height * 0.5, 0.0),
//                 VerticalAlign::Bottom => Vec3::ZERO,
//             } + match text.alignment.horizontal {
//                 HorizontalAlign::Left => Vec3::ZERO,
//                 HorizontalAlign::Center => Vec3::new(-width * 0.5, 0.0, 0.0),
//                 HorizontalAlign::Right => Vec3::new(-width, 0.0, 0.0),
//             };

//             let mut text_transform = *transform;
//             text_transform.scale /= scale_factor;

//             for text_glyph in text_glyphs {
//                 let color = text.sections[text_glyph.section_index]
//                     .style
//                     .color
//                     .as_rgba_linear();
//                 let atlas = texture_atlases
//                     .get(&text_glyph.atlas_info.texture_atlas)
//                     .unwrap();
//                 let handle = atlas.texture.clone_weak();
//                 let index = text_glyph.atlas_info.glyph_index as usize;
//                 let rect = Some(atlas.textures[index]);

//                 let glyph_transform = Transform::from_translation(
//                     alignment_offset * scale_factor + text_glyph.position.extend(0.),
//                 );

//                 let transform = text_transform.mul_transform(glyph_transform);

//                 extracted_sprites.sprites.push(ExtractedSprite {
//                     transform,
//                     color,
//                     rect,
//                     custom_size: None,
//                     image_handle_id: handle.id,
//                     flip_x: false,
//                     flip_y: false,
//                     anchor: Anchor::Center.as_vec(),
//                 });
//             }
//         }
//     }
// }

pub(crate) fn scale_value(value: f32, factor: f64) -> f32 {
    (value as f64 * factor) as f32
}
