//! Basic quad and text drawing inside a `Canvas`.

use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
use bevy::{
    color::palettes::css::*, log::LogPlugin, math::Rect, prelude::*, sprite::Anchor,
    window::PrimaryWindow,
};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_keith::*;

mod utils;
use utils::*;

fn main() {
    App::new()
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::default())
        // Helper to exit with ESC key
        .add_systems(Update, close_on_esc)
        // Default plugins
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    level: bevy::log::Level::WARN,
                    filter: "quad=trace,bevy_keith=warn,bevy=info".to_string(),
                    custom_layer: |_| None,
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "bevy_keith - quad".to_string(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(DARK_GRAY.into()))
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
    canvas.background_color = Some(BEIGE.into());
    commands.spawn((
        Camera2d::default(),
        canvas,
        MyRes {
            font: font.clone(),
            image: image.clone(),
        },
    ));

    // Display the text pipeline's glyph atlas as a debug visualization
    commands
        .spawn((
            Sprite {
                color: Color::srgba_u8(0, 0, 0, 128),
                custom_size: Some(Vec2::splat(512.)),
                ..default()
            },
            Transform::from_xyz(400., 0., 0.),
        ))
        .with_children(|p| {
            p.spawn(Sprite {
                image: text_pipeline.atlas_texture_handle.clone(),
                custom_size: Some(Vec2::splat(512.)),
                ..default()
            });
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
    let (brush, border_brush) = if rect.contains(cursor_pos) {
        let border_brush = ctx.solid_brush(Color::rgb(0.6, 0.6, 1.0));
        (ctx.solid_brush(Color::rgb(0.7, 0.7, 0.7)), border_brush)
    } else {
        let border_brush = ctx.solid_brush(Color::rgb(0.8, 0.8, 0.9));
        (ctx.solid_brush(Color::rgb(0.6, 0.6, 0.6)), border_brush)
    };
    ctx.fill(rect, &brush).border(&border_brush, 2.);

    // Text
    let text = ctx
        .new_layout(text.to_owned())
        .color(Color::srgb(0.2, 0.2, 0.2))
        .font(font)
        .font_size(12.)
        .bounds(rect.size())
        .anchor(Anchor::Center)
        .alignment(JustifyText::Center)
        .build();
    let pos = (rect.min + rect.max) / 2.;
    ctx.draw_text(text, pos);

    // Text Anchor
    let brush = ctx.solid_brush(RED.into());
    ctx.line(pos - Vec2::X * 3., pos + Vec2::X * 3., &brush, 1.);
    ctx.line(pos - Vec2::Y * 3., pos + Vec2::Y * 3., &brush, 1.);
}

fn run(
    time: Res<Time>,
    mut query: Query<(&Camera, &mut Canvas, &MyRes)>,
    q_window: Query<&Window, With<PrimaryWindow>>,
) {
    let (camera, mut canvas, my_res) = query.single_mut();
    canvas.clear();

    let mut ctx = canvas.render_context();

    let cursor_pos = if let Ok(window) = q_window.get_single() {
        window
            .cursor_position()
            // FIXME - cheap window-to-canvas hard-coded conversion
            .map(|v| Vec2::new(v.x - 1280. / 2., v.y - 720. / 2.))
    } else {
        None
    }
    .unwrap_or(Vec2::NAN);
    //trace!("cursor_pos={cursor_pos}");

    // ctx.clear(None, FUCHSIA.into());

    let border_brush = ctx.solid_brush(DARK_GREEN.into());
    let brush = ctx.solid_brush(BISQUE.into());
    let rect = Rect {
        min: Vec2::new(-10., -30.),
        max: Vec2::new(30., 130.),
    };
    let border_width = (time.elapsed_secs() * 2.).sin() * 4. + 4.;
    ctx.fill(rect, &brush).border(&border_brush, border_width);

    let radius = time.elapsed_secs().sin() * 8. + 8.;
    let brush = ctx.solid_brush(PINK.into());
    let rounded_rect = RoundedRect {
        rect: Rect {
            min: Vec2::new(30., -132.),
            max: Vec2::new(62., -100.),
        },
        radius,
    };
    ctx.fill(rounded_rect, &brush);

    let text = ctx
        .new_layout("Hello World!")
        .color(TEAL.into())
        .font(my_res.font.clone())
        .font_size(32.)
        .anchor(Anchor::BottomLeft)
        .build();
    ctx.draw_text(text, Vec2::ZERO);

    let rect = Rect {
        min: Vec2::new(100., 150.),
        max: Vec2::new(164., 214.),
    };
    ctx.draw_image(rect, my_res.image.clone(), ImageScaling::default());

    let brush = ctx.solid_brush(LAWN_GREEN.into());
    let delta = time.elapsed_secs().sin() * 15. + 30.;
    for i in 0..=10 {
        ctx.line(
            Vec2::new(-200.5, 0.5 + i as f32 * 15.),
            Vec2::new(0.5, 0.5 + i as f32 * delta),
            &brush,
            1. + i as f32,
        )
        .border(&border_brush, border_width);
    }

    let color = Color::oklch(0.5, 0.7, (time.elapsed_secs() / 3.).fract() * 360.);
    let text = ctx
        .new_layout("bevy_keith")
        .color(color)
        .font(my_res.font.clone())
        .font_size(128.)
        .anchor(Anchor::BottomLeft)
        .build();
    let canvas_size = camera.logical_target_size().unwrap();
    let bottom_left = Vec2::new(-canvas_size.x / 2., canvas_size.y / 2.);
    ctx.draw_text(text, bottom_left);

    // // Rounded rect with border
    // let rect = Rect::from_center_size(Vec2::new(300., 200.), Vec2::new(80.,
    // 40.)); let brush = ctx.solid_brush(Color::srgb(0.7, 0.7, 0.7));
    // let rrect = RoundedRect {
    //     rect,
    //     radius: 4.,
    // };
    // ctx.fill(rrect, &brush);
    // let brush = ctx.solid_brush(Color::srgb(0.6, 0.6, 0.6));
    // let rrect = RoundedRect {
    //     rect: rect.inset(0.5),
    //     radius: 4.5,
    // };
    // ctx.stroke(rrect, &brush, 1.);

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

    let s = "The quick brown fox jumps over the lazy dog THE QUICK BROWN FOX
    JUMPS OVER THE LAZY DOG !£$%^&*()_}{][#';~@:";
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
