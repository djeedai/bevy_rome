//! Basic quad and text drawing inside a `Canvas`.

use bevy::{
    log::LogPlugin, math::Rect, prelude::*, sprite::Anchor, text::Text2dBounds,
    window::PrimaryWindow,
};
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
                    filter: "text=trace,bevy_keith=info".to_string(),
                    update_subscriber: None,
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "bevy_keith - text".to_string(),
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
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("FiraSans-Regular.ttf");

    let mut canvas = Canvas::new(Rect {
        min: Vec2::splat(-400.),
        max: Vec2::splat(100.),
    });
    canvas.background_color = Some(Color::BEIGE);
    commands
        .spawn(Camera2dBundle::default())
        .insert(canvas)
        .insert(MyRes { font: font.clone() });

    // commands.spawn(Text2dBundle {
    //     text: Text::from_section(
    //         "The quick brown fox jumps over the lazy dog.",
    //         TextStyle {
    //             font: font.clone(),
    //             font_size: 16.,
    //             color: Color::BLACK,
    //         },
    //     ),
    //     text_2d_bounds: Text2dBounds {
    //         size: Vec2::new(100., 300.),
    //     },

    //     ..default()
    // });
}

fn draw_boxed_text(
    ctx: &mut RenderContext,
    rect: Rect,
    text: &str,
    font: Handle<Font>,
    cursor_pos: Vec2,
    anchor: Anchor,
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
        .anchor(anchor)
        .alignment(JustifyText::Center)
        .build();
    ctx.draw_text(text, (rect.min + rect.max) / 2.);
}

fn draw_anchored_text(
    ctx: &mut RenderContext,
    pos: Vec2,
    text: &str,
    font: Handle<Font>,
    anchor: Anchor,
) {
    let size = Vec2::new(120., 40.);

    // Background
    let brush = ctx.solid_brush(Color::rgba(0.6, 0.6, 0.6, 0.2));
    ctx.fill(
        Rect::from_center_size(pos - anchor.as_vec() * size, size),
        &brush,
    );

    // Anchor
    let brush = ctx.solid_brush(Color::RED);
    ctx.line(pos - Vec2::X * 3., pos + Vec2::X * 3., &brush, 1.);
    ctx.line(pos - Vec2::Y * 3., pos + Vec2::Y * 3., &brush, 1.);

    // Text
    let text = ctx
        .new_layout(text.to_owned())
        .color(Color::rgb(0.2, 0.2, 0.2))
        .font(font)
        .font_size(16.)
        .bounds(size)
        .anchor(anchor)
        .alignment(JustifyText::Left)
        .build();
    ctx.draw_text(text, pos);
}

fn run(mut query: Query<(&mut Canvas, &MyRes)>, q_window: Query<&Window, With<PrimaryWindow>>) {
    let (mut canvas, my_res) = query.single_mut();
    canvas.clear();

    let mut ctx = canvas.render_context();

    let cursor_pos = if let Ok(window) = q_window.get_single() {
        window
            .cursor_position()
            // FIXME - cheap window-to-canvas hard-coded conversion
            .map(|v| Vec2::new(v.x - 1280. / 2., 720. / 2. - v.y))
    } else {
        None
    }
    .unwrap_or(Vec2::NAN);

    // Anchor
    for (anchor, anchor_name) in [
        // (Anchor::TopLeft, "TopLeft"),
        // (Anchor::TopCenter, "TopCenter"),
        // (Anchor::TopRight, "TopRight"),
        // (Anchor::CenterLeft, "CenterLeft"),
        // (Anchor::Center, "Center"),
        // (Anchor::CenterRight, "CenterRight"),
        // (Anchor::BottomLeft, "BottomLeft"),
        // (Anchor::BottomCenter, "BottomCenter"),
        // (Anchor::BottomRight, "BottomRight"),
        (
            Anchor::TopLeft,
            "The quick brown fox jumps over the lazy dog.",
        ),
        (
            Anchor::TopCenter,
            "The quick brown fox jumps over the lazy dog.",
        ),
        (
            Anchor::TopRight,
            "The quick brown fox jumps over the lazy dog.",
        ),
        (
            Anchor::CenterLeft,
            "The quick brown fox jumps over the lazy dog.",
        ),
        (
            Anchor::Center,
            "The quick brown fox jumps over the lazy dog.",
        ),
        (
            Anchor::CenterRight,
            "The quick brown fox jumps over the lazy dog.",
        ),
        (
            Anchor::BottomLeft,
            "The quick brown fox jumps over the lazy dog.",
        ),
        (
            Anchor::BottomCenter,
            "The quick brown fox jumps over the lazy dog.",
        ),
        (
            Anchor::BottomRight,
            "The quick brown fox jumps over the lazy dog.",
        ),
    ] {
        // let pos = anchor.as_vec() * Vec2::new(280., 80.);
        let pos = anchor.as_vec() * Vec2::new(400., 200.);
        draw_anchored_text(&mut ctx, pos, anchor_name, my_res.font.clone(), anchor);
    }

    // Layout
    // for (j, a) in [Anchor::TopLeft, Anchor::TopCenter, Anchor::TopRight]
    //     .iter()
    //     .enumerate()
    // {
    //     let y = (j - 1) as f32 * 100.;
    //     for (i, w) in [30., 50., 70.].iter().enumerate() {
    //         let x = (i - 1) as f32 * 100.;
    //         let rect = Rect::from_center_size(Vec2::new(x, y), Vec2::new(*w,
    // 40.));         draw_boxed_text(
    //             &mut ctx,
    //             rect,
    //             "Submit",
    //             my_res.font.clone(),
    //             cursor_pos,
    //             a.clone(),
    //         );
    //     }
    // }

    // let rect = Rect {
    //     min: Vec2::new(-200., -140.),
    //     max: Vec2::new(-80., -110.),
    // };
    // draw_boxed_text(&mut ctx, rect, "Cancel", my_res.font.clone(),
    // cursor_pos); let rect = Rect {
    //     min: Vec2::new(-200., -180.),
    //     max: Vec2::new(-80., -150.),
    // };
    // draw_boxed_text(
    //     &mut ctx,
    //     rect,
    //     "This is a very long text that will not fit in the button",
    //     my_res.font.clone(),
    //     cursor_pos,
    // );
}
