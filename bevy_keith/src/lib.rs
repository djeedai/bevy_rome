#![allow(dead_code)]

mod canvas;
mod rect_ex;
mod render;
mod render_context;
mod text;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::render_context::RenderContext;
}

use render::{
    DrawPrimitive, ExtractedCanvases, ImageBindGroups, PrimitiveAssetEvents, PrimitiveMeta,
    PrimitivePipeline,
};

pub use canvas::{Canvas, Primitive};
pub use rect_ex::RectEx;
pub use render_context::RenderContext;
pub use text::{CanvasTextId, KeithTextPipeline};

use bevy::app::prelude::*;
use bevy::asset::{Assets, HandleUntyped};
use bevy::core_pipeline::core_2d::Transparent2d;
use bevy::ecs::schedule::SystemLabel;
use bevy::prelude::ParallelSystemDescriptorCoercion;
use bevy::reflect::TypeUuid;
use bevy::render::{
    render_phase::AddRenderCommand,
    render_resource::{Shader, SpecializedRenderPipelines},
    RenderApp, RenderStage,
};

#[derive(Default)]
pub struct KeithPlugin;

pub(crate) const PRIMITIVE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1713353953151292643);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
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
            .add_system_to_stage(
                CoreStage::PreUpdate,
                canvas::update_canvas_from_ortho_camera,
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                text::process_glyphs.label(KeithSystem::ProcessTextGlyphs), //.after(ModifiesWindows),
            );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<PrimitivePipeline>()
                .init_resource::<SpecializedRenderPipelines<PrimitivePipeline>>()
                .init_resource::<PrimitiveMeta>()
                .init_resource::<ExtractedCanvases>()
                .init_resource::<PrimitiveAssetEvents>()
                .add_render_command::<Transparent2d, DrawPrimitive>()
                .add_system_to_stage(
                    RenderStage::Extract,
                    render::extract_primitives.label(KeithSystem::ExtractPrimitives),
                )
                .add_system_to_stage(RenderStage::Extract, render::extract_primitive_events)
                .add_system_to_stage(RenderStage::Prepare, render::prepare_primitives)
                //.add_system_to_stage(RenderStage::Extract, text::extract_text_primitives)
                .add_system_to_stage(RenderStage::Queue, render::queue_primitives);
        };
    }
}
