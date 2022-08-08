//! Basic quad and text drawing inside a `Canvas`.

use bevy::{
    prelude::*,
    render::settings::{PowerPreference, WgpuSettings},
    sprite::Rect,
};
//use bevy_inspector_egui::WorldInspectorPlugin;

use bevy_keith::*;

fn main() {
    App::default()
        .insert_resource(WindowDescriptor {
            title: "bevy_keith - quad".to_string(),
            //scale_factor_override: Some(1.0),
            ..Default::default()
        })
        .insert_resource(WgpuSettings {
            power_preference: PowerPreference::HighPerformance,
            ..default()
        })
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .insert_resource(bevy::log::LogSettings {
            level: bevy::log::Level::WARN,
            filter: "bevy_keith=trace".to_string(),
        })
        .add_plugins(DefaultPlugins)
        .add_system(bevy::window::close_on_esc)
        .add_plugin(KeithPlugin)
        //.add_plugin(WorldInspectorPlugin::new())
        .add_startup_system(setup)
        .add_system(run)
        .run();
}

#[derive(Component)]
struct MyRes {
    font: Handle<Font>,
    image: Handle<Image>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("FiraSans-Regular.ttf");
    let image = asset_server.load("uvdev.png");

    let mut canvas = Canvas::new(Rect {
        min: Vec2::splat(-400.),
        max: Vec2::splat(100.),
    });
    canvas.set_background_color(Some(Color::BEIGE));
    commands
        .spawn_bundle(Camera2dBundle::default())
        .insert(canvas)
        .insert(MyRes {
            font: font.clone(),
            image: image.clone(),
        });

    // commands.spawn_bundle(Text2dBundle {
    //     text: Text::from_section(
    //         "Hello World!".to_string(),
    //         TextStyle {
    //             font,
    //             font_size: 24.0,
    //             color: Color::BLACK,
    //         },
    //     )
    //     .with_alignment(TextAlignment {
    //         vertical: VerticalAlign::Bottom,
    //         horizontal: HorizontalAlign::Left,
    //     }),
    //     transform: Transform::from_translation(Vec3::new(0., -16., 0.)),
    //     ..default()
    // });

    // commands.spawn_bundle(SpriteBundle {
    //     sprite: Sprite {
    //         custom_size: Some(Vec2::splat(64.)),
    //         ..default()
    //     },
    //     texture: image,
    //     transform: Transform::from_translation(Vec3::new(0., -128., 0.)),
    //     ..default()
    // });
}

fn run(mut query: Query<(&mut Canvas, &MyRes)>) {
    let (mut canvas, my_res) = query.single_mut();
    canvas.clear();

    let mut ctx = canvas.render_context();

    //ctx.clear(None, Color::FUCHSIA);

    let brush = ctx.solid_brush(Color::BISQUE);
    let rect = Rect {
        min: Vec2::new(-10., -30.),
        max: Vec2::new(30., 130.),
    };
    ctx.fill(rect, &brush);

    let brush = ctx.solid_brush(Color::PINK);
    let rect = Rect {
        min: Vec2::ZERO,
        max: Vec2::splat(50.),
    };
    ctx.fill(rect, &brush);

    let text = ctx
        .new_layout("Hello World!")
        .color(Color::ORANGE_RED)
        .font(my_res.font.clone())
        .font_size(24.)
        .build();
    ctx.draw_text(text, Vec2::ZERO);

    let rect = Rect {
        min: Vec2::new(100., 150.),
        max: Vec2::new(116., 166.),
    };
    ctx.draw_image(rect, my_res.image.clone());

    let brush = ctx.solid_brush(Color::GREEN);
    for i in 0..=10 {
        ctx.line(Vec2::new(-200.5, 0.5 + i as f32 * 15.), Vec2::new(0.5, 0.5 + i as f32 * 40.), &brush, 1. + i as f32);
    }
}
