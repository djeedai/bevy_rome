mod piet_canvas;
mod render;
mod render_context;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::render_context::BevyRenderContext;
}

use render::{DrawPiet, ImageBindGroups, QuadMeta};

pub use piet_canvas::PietCanvas;
pub use render_context::BevyRenderContext;

use bevy::app::prelude::*;
use bevy::asset::{AddAsset, Assets, HandleUntyped};
use bevy::core_pipeline::Transparent2d;
use bevy::ecs::schedule::SystemLabel;
use bevy::reflect::TypeUuid;
use bevy::render::{
    render_phase::AddRenderCommand,
    render_resource::{Shader, SpecializedRenderPipelines},
    RenderApp, RenderStage,
};

#[derive(Default)]
pub struct PietPlugin;

pub(crate) const QUAD_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2763343953151592643);

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum PietSystem {
    ExtractSprites,
}

impl Plugin for PietPlugin {
    fn build(&self, app: &mut App) {
        // let mut shaders = app.world.resource_mut::<Assets<Shader>>();
        // let sprite_shader = Shader::from_wgsl(include_str!("render/piet.wgsl"));
        // shaders.set_untracked(QUAD_SHADER_HANDLE, sprite_shader);

        app.register_type::<PietCanvas>();

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                //.init_resource::<SpritePipeline>()
                //.init_resource::<SpecializedRenderPipelines<SpritePipeline>>()
                .init_resource::<QuadMeta>()
                //.init_resource::<ExtractedSprites>()
                //.init_resource::<SpriteAssetEvents>()
                .add_render_command::<Transparent2d, DrawPiet>()
                // .add_system_to_stage(
                //     RenderStage::Extract,
                //     render::extract_quads.label(PietSystem::ExtractQuads),
                // )
                //.add_system_to_stage(RenderStage::Extract, render::extract_sprite_events)
                .add_system_to_stage(RenderStage::Queue, render::queue_quads);
        };
    }
}
