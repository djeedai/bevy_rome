use bevy::ecs::{entity::Entity, component::Component};

use crate::Name;

enum SignedIntegralType {
    Int8, Int16, Int32, Int64, Int128,
}

enum UnsignedIntegralType {
    Uint8, Uint16, Uint32, Uint64, Uint128,
}

enum FloatingType {
    Float32, Float64,
}

enum SimpleType {
    Bool,
    Int8, Int16, Int32, Int64, Int128,
    Uint8, Uint16, Uint32, Uint64, Uint128,
    Float32, Float64,
}

enum BuiltInType {
    Bool,
    Int8, Int16, Int32, Int64, Int128,
    Uint8, Uint16, Uint32, Uint64, Uint128,
    Float32, Float64,
    Name,
    Enum(UnsignedIntegralType), // C-style enum, not Rust-style
    Array(AnyType),
    Dict(AnyType), // key type is Name
    ObjectRef(Option<Entity>),
}

enum AnyType {
    BuiltIn(Box<BuiltInType>), // to avoid circular ref
    Custom(Entity), // with GameType component
}

/// Representation of a type of the Game.
#[derive(Component)]
pub(crate) struct GameType {
    /// Type name.
    name: Name,
    /// Collection of instances of this type.
    instances: Vec<Entity>,
}

impl GameType {
    pub fn new(name: Name) -> Self {
        Self {
            name,
            instances: vec![],
        }
    }

    pub fn instances(&self) -> &[Entity] {
        &self.instances[..]
    }

    pub fn instances_mut(&mut self) -> &mut [Entity] {
        &mut self.instances[..]
    }
}
