#![allow(dead_code)]

mod canvas;
mod render;
mod render_context;
mod shapes;
mod text;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::render_context::RenderContext;
}

use bevy::prelude::*;
use bevy::render::RenderSet;
use bevy::sprite::SpriteSystem;
use render::{
    DrawPrimitive, ExtractedCanvases, ImageBindGroups, PrimitiveAssetEvents, PrimitiveMeta,
    PrimitivePipeline,
};

pub use canvas::{Canvas, Primitive};
pub use render_context::RenderContext;
pub use shapes::*;
pub use text::{CanvasTextId, KeithTextPipeline};

use bevy::asset::{Assets, HandleUntyped};
use bevy::core_pipeline::core_2d::Transparent2d;
use bevy::reflect::TypeUuid;
use bevy::render::{
    render_phase::AddRenderCommand,
    render_resource::{Shader, SpecializedRenderPipelines},
    RenderApp,
};

#[derive(Default)]
pub struct KeithPlugin;

pub(crate) const PRIMITIVE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1713353953151292643);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum KeithSystem {
    /// Label for [`text::process_glyphs()`].
    ProcessTextGlyphs,
    /// Label for [`render::extract_primitives()`].
    ExtractPrimitives,
}

impl Plugin for KeithPlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world.resource_mut::<Assets<Shader>>();
        let primitives_shader = Shader::from_wgsl(include_str!("render/prim.wgsl"));
        shaders.set_untracked(PRIMITIVE_SHADER_HANDLE, primitives_shader);

        app.register_type::<Canvas>()
            .init_resource::<KeithTextPipeline>()
            .add_system(canvas::update_canvas_from_ortho_camera.in_base_set(CoreSet::PreUpdate))
            .add_system(
                text::process_glyphs
                    //.label(KeithSystem::ProcessTextGlyphs)
                    .in_base_set(CoreSet::PostUpdate),
            ); //.after(ModifiesWindows),

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<PrimitivePipeline>()
                .init_resource::<SpecializedRenderPipelines<PrimitivePipeline>>()
                .init_resource::<PrimitiveMeta>()
                .init_resource::<ExtractedCanvases>()
                .init_resource::<PrimitiveAssetEvents>()
                .add_render_command::<Transparent2d, DrawPrimitive>()
                .add_system(
                    // Must be after VisibilityPropagate, which is in CoreSet::PostUpdate, so render's extract is OK
                    render::extract_primitives
                        .in_set(KeithSystem::ExtractPrimitives)
                        .after(SpriteSystem::ExtractSprites) // for TextureAtlas
                        .in_schedule(ExtractSchedule),
                )
                .add_system(render::extract_primitive_events.in_schedule(ExtractSchedule))
                //.add_system(text::extract_text_primitives.in_schedule(ExtractSchedule))
                .add_system(render::prepare_primitives.in_set(RenderSet::Prepare))
                .add_system(render::queue_primitives.in_set(RenderSet::Queue));
        };
    }
}
