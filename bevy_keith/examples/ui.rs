//! Basic quad and text drawing inside a `Canvas`.

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::render::camera::ScalingMode;
use bevy::{log::LogPlugin, math::Rect, prelude::*, sprite::Anchor, window::PrimaryWindow};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_keith::*;

fn main() {
    App::new()
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        // Helper to exit with ESC key
        .add_systems(Update, bevy::window::close_on_esc)
        // Default plugins
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    level: bevy::log::Level::WARN,
                    filter: "ui=trace,bevy_keith=trace,bevy=info".to_string(),
                    update_subscriber: None,
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "bevy_keith - ui".to_string(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::PURPLE))
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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    text_pipeline: Res<KeithTextPipeline>,
) {
    let font = asset_server.load("FiraSans-Regular.ttf");
    let image = asset_server.load("uvdev.png");

    let mut canvas = Canvas::new(Rect {
        min: Vec2::splat(-400.),
        max: Vec2::splat(100.),
    });
    canvas.background_color = Some(Color::BEIGE);
    commands
        .spawn(Camera2dBundle {
            projection: OrthographicProjection {
                // Set viewport origin at bottom left corner (Bevy) and top left corner (Keith).
                // Keith uses an inverted Y down coordinate system.
                viewport_origin: Vec2::ZERO,
                // Scale viewport to match 1:1 the window pixel size.
                scaling_mode: ScalingMode::WindowSize(1.),
                ..default()
            },
            ..default()
        })
        .insert(canvas)
        .insert(MyRes {
            font: font.clone(),
            image: image.clone(),
        });

    // Display the text pipeline's glyph atlas as a debug visualization
    commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgba_u8(0, 0, 0, 128),
                custom_size: Some(Vec2::splat(512.)),
                ..default()
            },
            transform: Transform::from_xyz(400., 0., 0.),
            ..default()
        })
        .with_children(|p| {
            p.spawn(SpriteBundle {
                texture: text_pipeline.atlas_texture_handle.clone(),
                sprite: Sprite {
                    custom_size: Some(Vec2::splat(512.)),
                    ..default()
                },
                ..default()
            });
        });
}

fn draw_menu(
    ctx: &mut RenderContext,
    rect: Rect,
    entries: &[(&str, f32)],
    font: &Handle<Font>,
    cursor_pos: Vec2,
) {
    // Background
    let brush = if rect.contains(cursor_pos) {
        ctx.solid_brush(Color::rgb(0.7, 0.7, 0.7))
    } else {
        ctx.solid_brush(Color::rgb(0.6, 0.6, 0.6))
    };
    ctx.fill(rect, &brush);

    // Entries
    let mut x = 10.;
    for &(txt, offset) in entries {
        let text = ctx
            .new_layout(txt.to_string())
            .color(Color::BLACK)
            .font(font.clone())
            .font_size(16.)
            .anchor(Anchor::BottomLeft)
            .build();
        ctx.draw_text(text, Vec2::new(x, 12.0));
        x += offset;
    }
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
    //ctx.stroke(rect, &brush, 1.);

    // Text
    let text = ctx
        .new_layout(text.to_owned())
        .color(Color::rgb(0.2, 0.2, 0.2))
        .font(font)
        .font_size(16.)
        .bounds(rect.size())
        .alignment(JustifyText::Center)
        .anchor(Anchor::Center)
        .build();
    ctx.draw_text(text, rect.center());
}

fn run(mut query: Query<(&mut Canvas, &MyRes)>, q_window: Query<&Window, With<PrimaryWindow>>) {
    let (mut canvas, my_res) = query.single_mut();
    canvas.clear();

    let mut ctx = canvas.render_context();

    let cursor_pos = if let Ok(window) = q_window.get_single() {
        window
            .cursor_position()
            // FIXME - cheap window-to-canvas hard-coded conversion
            .map(|v| Vec2::new(v.x, v.y))
    } else {
        None
    }
    .unwrap_or(Vec2::NAN);
    //trace!("cursor_pos={cursor_pos}");

    let rect = Rect {
        min: Vec2::new(0., 0.),
        max: Vec2::new(1280., 24.),
    };
    // draw_menu(
    //     &mut ctx,
    //     rect,
    //     &[("File", 70.), ("Edit", 75.), ("Selection", 140.), ("View", 85.),
    // ("Window", 120.), ("Help", 70.)],     &my_res.font,
    //     cursor_pos,
    // );

    draw_button(
        &mut ctx,
        Rect::new(8., 32., 108., 56.),
        "Run",
        my_res.font.clone(),
        cursor_pos,
    );
}
