use bevy::{ecs::component::Component, reflect::Reflect};

#[derive(Component, Debug, Default, Clone, Reflect)]
pub struct PietCanvas;
