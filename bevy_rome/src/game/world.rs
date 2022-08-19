use bevy::{
    ecs::{
        event::EventReader,
        query::Changed,
        system::{Commands, Query},
        world::World,
    },
    log::debug,
    reflect::Reflect,
};

use crate::{game::types::GameType, Name};

#[derive(Debug, Default)]
pub struct GameWorld {
    world: World,
}

impl GameWorld {
    pub fn new() -> Self {
        GameWorld {
            world: World::new(),
        }
    }
}

#[derive(Reflect)]
pub(crate) struct AddTypesEvent {
    name: Name,
}

/// Insert into the [`GameWorld`] new types from [`AddTypesEvent`] events.
pub(crate) fn add_types(mut commands: Commands, mut events: EventReader<AddTypesEvent>) {
    for event in events.iter() {
        debug!("Add type '{}' to game world", event.name.str());
        commands.spawn().insert(GameType::new(event.name.clone()));
    }
}

fn apply_migration_rules(mut query: Query<&mut GameType, Changed<GameType>>) {
    for mut game_type in query.iter_mut() {
        // apply migration rules to all instances of this type
        for instance in game_type.instances_mut() { // FIXME - this loop triggers change detection
             // [...]
        }
    }
}
