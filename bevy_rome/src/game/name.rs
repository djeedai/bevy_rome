use std::ops::{Deref, DerefMut};

use bevy::reflect::Reflect;

#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Reflect)] // ideally, Copy too
pub struct Name(pub String);

impl Name {
    pub fn str(&self) -> &str {
        &self.0
    }
}

impl Deref for Name {
    type Target = String;

    fn deref(&self) -> &String {
        &self.0
    }
}

impl DerefMut for Name {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}
