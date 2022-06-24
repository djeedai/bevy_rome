//! Basic quad drawing.

use bevy::{
    prelude::*,
    render::{
        camera::ScalingMode,
        render_resource::WgpuFeatures,
        settings::{PowerPreference, WgpuSettings},
    },
    sprite::MaterialMesh2dBundle,
};
use bevy_inspector_egui::WorldInspectorPlugin;

use bevy_piet::*;

fn main() {
    App::default()
        .insert_resource(WgpuSettings {
            power_preference: PowerPreference::HighPerformance,
            ..default()
        })
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .insert_resource(bevy::log::LogSettings {
            level: bevy::log::Level::WARN,
            filter: "bevy_piet=trace,spawn=trace".to_string(),
        })
        .add_plugins(DefaultPlugins)
        .add_system(bevy::input::system::exit_on_esc_system)
        .add_plugin(PietPlugin)
        .add_plugin(WorldInspectorPlugin::new())
        .add_startup_system(setup)
        //.add_system(run)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut camera = OrthographicCameraBundle::new_2d();
    // camera.orthographic_projection.scale = 1.0;
    // camera.orthographic_projection.scaling_mode = ScalingMode::FixedVertical;
    // camera.transform.translation.z = camera.orthographic_projection.far / 2.0;
    commands.spawn_bundle(camera);

    let mut canvas = PietCanvas::default();

    canvas.quads_vec().push(Quad {
        rect: bevy::sprite::Rect {
            min: Vec2::ZERO,
            max: Vec2::new(100., 50.),
        },
        color: Color::RED,
        flip_x: false,
        flip_y: false,
    });

    commands
        .spawn_bundle((Transform::default(), GlobalTransform::default(), canvas))
        .insert(Name::new("canvas"));
}

fn run() {

}