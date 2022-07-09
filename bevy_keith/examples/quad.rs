//! Basic quad drawing.

use bevy::{
    prelude::*,
    render::{
        camera::ScalingMode,
        settings::{PowerPreference, WgpuSettings},
    },
    sprite::Rect,
};
use bevy_inspector_egui::WorldInspectorPlugin;

use bevy_keith::*;

fn main() {
    App::default()
        .insert_resource(WgpuSettings {
            power_preference: PowerPreference::HighPerformance,
            ..default()
        })
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .insert_resource(bevy::log::LogSettings {
            level: bevy::log::Level::WARN,
            filter: "bevy_keith=trace,spawn=trace".to_string(),
        })
        .add_plugins(DefaultPlugins)
        .add_system(bevy::input::system::exit_on_esc_system)
        .add_plugin(KeithPlugin)
        .add_plugin(WorldInspectorPlugin::new())
        .add_startup_system(setup)
        .add_system(run)
        .run();
}

fn setup(mut commands: Commands) {
    let mut canvas = Canvas::new(Rect {
        min: Vec2::splat(-400.),
        max: Vec2::splat(100.),
    });
    canvas.set_background_color(Some(Color::FUCHSIA));
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(canvas);
}

fn run(mut query: Query<&mut Canvas>) {
    let mut canvas = query.single_mut();
    canvas.clear();

    let mut ctx = canvas.render_context();

    //ctx.clear(None, Color::FUCHSIA);

    let brush = ctx.solid_brush(Color::AQUAMARINE);
    let rect = Rect {
        min: Vec2::new(-10., -30.),
        max: Vec2::new(30., 130.),
    };
    ctx.fill(rect, &brush);

    let brush = ctx.solid_brush(Color::RED);
    let rect = Rect {
        min: Vec2::ZERO,
        max: Vec2::splat(50.),
    };
    ctx.fill(rect, &brush);

    // let brush = ctx.solid_brush(Color::GREEN);
    // let line = Line::new(Point::new(-10., -30.), Point::new(20., 100.));
    // ctx.stroke(line, &brush, 13.);
}
