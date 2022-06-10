use bevy::ecs::{
    component::{Component, ComponentId},
    entity::Entity,
    world::{Mut, World},
};
use bevy::math::Vec3;
use bevy::reflect::{Reflect, ReflectRef};
use bevy::transform::components::Transform;
use serde::{Deserialize, Serialize, Serializer};
use std::any::{Any, TypeId};
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

use super::error::Error;

#[derive(Serialize, Deserialize)]
struct DiffTarget {
    entity: Entity,
    component: usize, //ComponentId,
}

impl DiffTarget {
    pub fn resolve<'w, T: Component>(&self, world: &'w World) -> Option<&'w T> {
        if let Some(entity) = world.get_entity(self.entity) {
            // TODO - Validate self.component!
            entity.get::<T>()
        } else {
            None
        }
    }

    // pub fn resolve_mut<'w, T: Component>(&self, world: &'w mut World) -> Option<Mut<'w, T>> {
    //     if let Some(mut entity) = world.get_entity_mut(self.entity) {
    //         // TODO - Validate self.component!
    //         entity.get_mut::<T>()
    //     } else {
    //         None
    //     }
    // }
}

#[derive(Serialize, Deserialize)]
struct DiffData {
    path: String,
    data: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
enum DiffContent {
    Single(DiffData),
    Dual(DiffData, DiffData),
}

#[derive(Serialize, Deserialize)]
struct Diff {
    target: DiffTarget,
    content: DiffContent,
}

impl Diff {
    pub fn make<T: Reflect>(base: &T, curr: &T) -> Diff {
        match base.reflect_ref() {
            ReflectRef::Struct(s) => {
                println!("struct {}", s.type_name());
                for f in s.iter_fields() {
                    println!("{:?}", f);
                }
            },
            _ => ()
        }
        Diff {
            target: DiffTarget {
                entity: Entity::from_raw(0),
                component: 0,
            },
            content: DiffContent::Single(DiffData {
                path: "".to_string(),
                data: vec![],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Reflect)]
    struct S {
        f: f32,
        i: i32,
    }

    #[test]
    fn diff_make() {
        let base = S { f: 3., i: -42 };
        let curr = S { f: 5., i: -420 };
        let diff = Diff::make(&base, &curr);
    }
}
