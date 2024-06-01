//! Basic quad and text drawing inside a `Canvas`.

use bevy::{log::LogPlugin, math::Rect, prelude::*, window::PrimaryWindow};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

use bevy_keith::*;

fn main() {
    App::new()
        // Helper to exit with ESC key
        .add_systems(Update, bevy::window::close_on_esc)
        // Default plugins
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    level: bevy::log::Level::WARN,
                    filter: "quad=trace,bevy_keith=debug,bevy=info".to_string(),
                    update_subscriber: None,
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "bevy_keith - quad".to_string(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .add_plugins(KeithPlugin)
        .add_plugins(WorldInspectorPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, run)
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
        .spawn(Camera2dBundle::default())
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

fn draw_button(
    ctx: &mut RenderContext,
    rect: Rect,
    text: &str,
    font: Handle<Font>,
    cursor_pos: Vec2,
) {
    // Background
    let brush = if rect.contains(cursor_pos) {
        ctx.solid_brush(Color::rgb(0.7, 0.7, 0.7))
    } else {
        ctx.solid_brush(Color::rgb(0.6, 0.6, 0.6))
    };
    ctx.fill(rect, &brush);

    // Outline
    let brush = ctx.solid_brush(Color::rgb(0.5, 0.5, 0.5));
    ctx.stroke(rect, &brush, 1.);

    // Text
    let text = ctx
        .new_layout(text.to_owned())
        .color(Color::rgb(0.2, 0.2, 0.2))
        .font(font)
        .font_size(16.)
        .bounds(rect.size())
        .alignment(JustifyText::Center)
        .build();
    ctx.draw_text(text, (rect.min + rect.max) / 2.);
}

fn run(mut query: Query<(&mut Canvas, &MyRes)>, q_window: Query<&Window, With<PrimaryWindow>>) {
    let (mut canvas, my_res) = query.single_mut();
    canvas.clear();

    let mut ctx = canvas.render_context();

    let cursor_pos = if let Ok(window) = q_window.get_single() {
        window
            .cursor_position()
            .map(|v| Vec2::new(v.x - 1280. / 2., 720. / 2. - v.y)) // FIXME - cheap window-to-canvas hard-coded conversion
    } else {
        None
    }
    .unwrap_or(Vec2::NAN);

    // ctx.clear(None, Color::FUCHSIA);

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
        .font_size(16.)
        .build();
    ctx.draw_text(text, Vec2::new(100., -20.0));

    let rect = Rect {
        min: Vec2::new(100., 150.),
        max: Vec2::new(164., 214.),
    };
    ctx.draw_image(rect, my_res.image.clone());

    let brush = ctx.solid_brush(Color::GREEN);
    for i in 0..=10 {
        ctx.line(
            Vec2::new(-200.5, 0.5 + i as f32 * 15.),
            Vec2::new(0.5, 0.5 + i as f32 * 40.),
            &brush,
            1. + i as f32,
        );
    }

    // Rounded rect with border
    let rect = Rect::from_center_size(Vec2::new(300., 200.), Vec2::new(80., 40.));
    let brush = ctx.solid_brush(Color::rgb(0.7, 0.7, 0.7));
    let rrect = RoundedRect {
        rect,
        radii: Vec2::splat(4.),
    };
    ctx.fill(rrect, &brush);
    let brush = ctx.solid_brush(Color::rgb(0.6, 0.6, 0.6));
    let rrect = RoundedRect {
        rect: rect.inset(0.5),
        radii: Vec2::splat(4.5),
    };
    ctx.stroke(rrect, &brush, 1.);

    // Buttons
    let rect = Rect {
        min: Vec2::new(-200., -100.),
        max: Vec2::new(-80., -70.),
    };
    draw_button(&mut ctx, rect, "Submit", my_res.font.clone(), cursor_pos);
    let rect = Rect {
        min: Vec2::new(-200., -140.),
        max: Vec2::new(-80., -110.),
    };
    draw_button(&mut ctx, rect, "Cancel", my_res.font.clone(), cursor_pos);
    let rect = Rect {
        min: Vec2::new(-200., -180.),
        max: Vec2::new(-80., -150.),
    };
    draw_button(
        &mut ctx,
        rect,
        "This is a very long text that will not fit in the button",
        my_res.font.clone(),
        cursor_pos,
    );

    let s = "The quick brown fox jumps over the lazy dog THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG !£$%^&*()_}{][#';~@:";
    for (i, st) in s.as_bytes().chunks(10).enumerate() {
        let rect = Rect {
            min: Vec2::new(-400., -180. + i as f32 * 35.),
            max: Vec2::new(-280., -150. + i as f32 * 35.),
        };
        draw_button(
            &mut ctx,
            rect,
            &format!("Button #{} {}", i, String::from_utf8_lossy(st)),
            my_res.font.clone(),
            cursor_pos,
        );
    }
}
