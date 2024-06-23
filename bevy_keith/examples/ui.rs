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
                    filter: "ui=trace,bevy_keith=warn,bevy=info".to_string(),
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
        //.add_plugins(WorldInspectorPlugin::default())
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
    anchor: Anchor,
    justify: JustifyText,
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
        .anchor(anchor)
        .alignment(justify)
        .build();
    let anchor = anchor.as_vec();
    let anchor = Vec2::new(anchor.x, -anchor.y);
    let text_pos = anchor * rect.size() + rect.center();
    ctx.draw_text(text, text_pos);

    // Text origin
    let brush = ctx.solid_brush(Color::BLUE);
    ctx.line(text_pos - Vec2::X * 3., text_pos + Vec2::X * 3., &brush, 1.);
    ctx.line(text_pos - Vec2::Y * 3., text_pos + Vec2::Y * 3., &brush, 1.);
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

    let anchors = [
        [Anchor::TopLeft, Anchor::TopCenter, Anchor::TopRight],
        [Anchor::CenterLeft, Anchor::Center, Anchor::CenterRight],
        [
            Anchor::BottomLeft,
            Anchor::BottomCenter,
            Anchor::BottomRight,
        ],
    ];
    let size = Vec2::new(100., 32.);
    // justify
    for k in -1i32..=1i32 {
        // anchor Y
        for j in -1i32..=1i32 {
            // anchor X
            for i in -1i32..=1i32 {
                //TEMP
                // if k != -1 || i != -1 || j != 0 {
                //     continue;
                // }

                let origin = Vec2::new(
                    i as f32 * 110. + 200. + (k + 1) as f32 * 400.,
                    j as f32 * 50. + 200.,
                );
                let anchor = anchors[(j + 1) as usize][(i + 1) as usize];
                let justify = match k {
                    -1 => JustifyText::Left,
                    0 => JustifyText::Center,
                    1 => JustifyText::Right,
                    _ => unimplemented!(),
                };
                draw_button(
                    &mut ctx,
                    Rect::from_center_size(origin, size),
                    "Run as fast as you can",
                    my_res.font.clone(),
                    cursor_pos,
                    anchor,
                    justify,
                );

                // // Anchor
                // let brush = ctx.solid_brush(Color::RED);
                // let pos = anchor.as_vec();
                // let pos = Vec2::new(pos.x, -pos.y);
                // let pos = origin + pos * size;
                // ctx.line(pos - Vec2::X * 3., pos + Vec2::X * 3., &brush, 1.);
                // ctx.line(pos - Vec2::Y * 3., pos + Vec2::Y * 3., &brush, 1.);
            }
        }
    }
}
