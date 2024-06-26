mod canvas;
mod render;
mod render_context;
mod shapes;
mod text;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::render_context::RenderContext;
}

use bevy::asset::load_internal_asset;
use bevy::core_pipeline::core_2d::Transparent2d;
use bevy::prelude::*;
use bevy::render::{
    render_phase::AddRenderCommand,
    render_resource::{Shader, SpecializedRenderPipelines},
    RenderApp,
};
use bevy::render::{Render, RenderSet};
pub use canvas::{Canvas, Primitive};
use render::{
    DrawPrimitive, ExtractedCanvases, ImageBindGroups, PrimitiveAssetEvents, PrimitiveMeta,
    PrimitivePipeline,
};
pub use render_context::RenderContext;
pub use shapes::*;
pub use text::{CanvasTextId, KeithTextPipeline};

/// Main Keith plugin.
#[derive(Default)]
pub struct KeithPlugin;

/// Reference to the primitive shader `prim.wgsl`, embedded in the code.
pub(crate) const PRIMITIVE_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(1713353953151292643);

/// System sets for Keith.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum KeithSystem {
    /// Label for [`text::process_glyphs()`].
    ProcessTextGlyphs,

    /// Spawn any [`Tiles`] or [`TileConfig`] component where missing.
    ///
    /// This executes as part of the [`PostUpdate`] schedule.
    SpawnMissingTilesComponents,

    /// Resize the [`Tiles`] component of a [`Canvas`] to accomodate the size of
    /// the render target of a [`Camera`].
    // FIXME - Currently a canvas always targets the full camera screen size.
    ResizeTilesToCameraRenderTarget,

    /// Label for [`render::extract_primitives()`].
    ExtractPrimitives,
}

impl Plugin for KeithPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PRIMITIVE_SHADER_HANDLE,
            "render/prim.wgsl",
            Shader::from_wgsl
        );

        app.init_resource::<KeithTextPipeline>()
            .add_systems(PreUpdate, canvas::update_canvas_from_ortho_camera)
            .add_systems(PostUpdate, text::process_glyphs)
            .configure_sets(
                PostUpdate,
                (
                    KeithSystem::SpawnMissingTilesComponents,
                    KeithSystem::ResizeTilesToCameraRenderTarget,
                )
                    .chain()
                    // We need the result of the positioned glyphs to be able to assign them to
                    // tiles
                    .after(text::process_glyphs),
            )
            .add_systems(
                PostUpdate,
                (
                    canvas::spawn_missing_tiles_components
                        .in_set(KeithSystem::SpawnMissingTilesComponents),
                    canvas::resize_tiles_to_camera_render_target
                        .in_set(KeithSystem::ResizeTilesToCameraRenderTarget)
                        .after(bevy::transform::TransformSystem::TransformPropagate)
                        .after(bevy::render::view::VisibilitySystems::CheckVisibility)
                        .after(bevy::render::camera::CameraUpdateSystem),
                    canvas::allocate_atlas_layouts,
                ),
            );
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<PrimitivePipeline>()
                .init_resource::<SpecializedRenderPipelines<PrimitivePipeline>>()
                .init_resource::<PrimitiveMeta>()
                .init_resource::<ExtractedCanvases>()
                .init_resource::<PrimitiveAssetEvents>()
                .add_render_command::<Transparent2d, DrawPrimitive>()
                .configure_sets(ExtractSchedule, KeithSystem::ExtractPrimitives)
                .edit_schedule(ExtractSchedule, |schedule| {
                    schedule.add_systems(
                        (
                            render::extract_primitives,
                            render::extract_primitive_events,
                            // text::extract_text_primitives
                        )
                            .in_set(KeithSystem::ExtractPrimitives)
                            .after(bevy::sprite::SpriteSystem::ExtractSprites),
                    );
                })
                .add_systems(
                    Render,
                    (
                        render::prepare_primitives
                            .in_set(RenderSet::PrepareAssets)
                            .after(KeithSystem::ExtractPrimitives)
                            .after(bevy::text::extract_text2d_sprite),
                        render::queue_primitives
                            .in_set(RenderSet::Queue)
                            .after(render::prepare_primitives)
                            .before(bevy::render::render_phase::sort_phase_system::<Transparent2d>),
                        render::prepare_bind_groups
                            .in_set(RenderSet::PrepareBindGroups)
                            .after(render::queue_primitives)
                            .after(bevy::render::render_asset::prepare_assets::<Image>),
                    ),
                );
        };
    }
}
