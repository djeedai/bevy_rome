#![allow(unused)]

use std::{fmt::Display, num::NonZeroU8};

use bevy::{
    log::LogPlugin,
    prelude::*,
    render::{settings::WgpuSettings, RenderPlugin},
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use crate::prelude::*;

/// Helper system to enable closing the example application by pressing the
/// escape key (ESC).
pub fn close_on_esc(mut ev_app_exit: EventWriter<AppExit>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::Escape) {
        ev_app_exit.send(AppExit::Success);
    }
}
