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
use bevy::render::{Render, RenderSet};
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
        let primitives_shader =
            Shader::from_wgsl(include_str!("render/prim.wgsl"), "bevy_keith/prim.wgsl");
        shaders.set_untracked(PRIMITIVE_SHADER_HANDLE, primitives_shader);

        app.register_type::<Canvas>()
            .init_resource::<KeithTextPipeline>()
            .add_systems(PreUpdate, canvas::update_canvas_from_ortho_camera)
            .add_systems(PostUpdate, text::process_glyphs); //.after(ModifiesWindows),

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<PrimitivePipeline>()
                .init_resource::<SpecializedRenderPipelines<PrimitivePipeline>>()
                .init_resource::<PrimitiveMeta>()
                .init_resource::<ExtractedCanvases>()
                .init_resource::<PrimitiveAssetEvents>()
                .add_render_command::<Transparent2d, DrawPrimitive>()
                .configure_set(ExtractSchedule, KeithSystem::ExtractPrimitives)
                .edit_schedule(ExtractSchedule, |schedule| {
                    schedule.add_systems(
                        (
                            render::extract_primitives,
                            render::extract_primitive_events, /*, text::extract_text_primitives*/
                        )
                            .in_set(KeithSystem::ExtractPrimitives)
                            .after(SpriteSystem::ExtractSprites),
                    );
                })
                .add_systems(
                    Render,
                    render::prepare_primitives.in_set(RenderSet::Prepare),
                )
                .add_systems(Render, render::queue_primitives.in_set(RenderSet::Queue));
        };
    }
}
