mod name;
mod types;
mod world;

use world::{AddTypesEvent, GameWorld};

pub use name::Name;

use bevy::app::{prelude::*, AppLabel};
use bevy::asset::{Assets, HandleUntyped};
use bevy::ecs::schedule::SystemLabel;
use bevy::prelude::ParallelSystemDescriptorCoercion;
use bevy::reflect::TypeUuid;

#[derive(Default)]
pub struct GamePlugin;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum GameSystems {
    /// Label for [`text::process_glyphs()`].
    ProcessTextGlyphs,
    /// Label for [`render::extract_primitives()`].
    ExtractPrimitives,
}

/// Label for the game world sub-app.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct GameApp;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<AddTypesEvent>()
            .init_resource::<GameWorld>();

        let mut game_app = App::empty();
        game_app
            .add_event::<AddTypesEvent>()
            .add_system_to_stage(CoreStage::First, world::add_types);

        app.add_sub_app(GameApp, game_app, move |editor_world, game_app| {
            #[cfg(feature = "trace")]
            let _render_span = bevy_utils::tracing::info_span!("renderer subapp").entered();
            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "reserve_and_flush").entered();

                // reserve all existing app entities for use in game_app
                // they can only be spawned using `get_or_spawn()`
                let meta_len = editor_world.entities().meta_len();
                game_app.world.entities().reserve_entities(meta_len as u32);

                // flushing as "invalid" ensures that app world entities aren't added as "empty archetype" entities by default
                // these entities cannot be accessed without spawning directly onto them
                // this _only_ works as expected because clear_entities() is called at the end of every frame.
                unsafe { game_app.world.entities_mut() }.flush_as_invalid();
            }

            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "extract").entered();

                // extract
                extract(editor_world, game_app);
            }

            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "prepare").entered();

                // prepare
                let prepare = game_app
                    .schedule
                    .get_stage_mut::<SystemStage>(&RenderStage::Prepare)
                    .unwrap();
                prepare.run(&mut game_app.world);
            }

            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "queue").entered();

                // queue
                let queue = game_app
                    .schedule
                    .get_stage_mut::<SystemStage>(&RenderStage::Queue)
                    .unwrap();
                queue.run(&mut game_app.world);
            }

            {
                #[cfg(feature = "trace")]
                let _stage_span = bevy_utils::tracing::info_span!("stage", name = "sort").entered();

                // phase sort
                let phase_sort = game_app
                    .schedule
                    .get_stage_mut::<SystemStage>(&RenderStage::PhaseSort)
                    .unwrap();
                phase_sort.run(&mut game_app.world);
            }

            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "render").entered();

                // render
                let render = game_app
                    .schedule
                    .get_stage_mut::<SystemStage>(&RenderStage::Render)
                    .unwrap();
                render.run(&mut game_app.world);
            }

            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "cleanup").entered();

                // cleanup
                let cleanup = game_app
                    .schedule
                    .get_stage_mut::<SystemStage>(&RenderStage::Cleanup)
                    .unwrap();
                cleanup.run(&mut game_app.world);
            }
            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "clear_entities").entered();

                game_app.world.clear_entities();
            }
        });
    }
}
