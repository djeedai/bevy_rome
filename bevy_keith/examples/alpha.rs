//! Test for various alpha blending cases.

use bevy::render::camera::ScalingMode;
use bevy::{log::LogPlugin, math::Rect, prelude::*, window::PrimaryWindow};
use bevy_keith::*;

fn main() {
    App::new()
        .add_systems(Update, bevy::window::close_on_esc)
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    level: bevy::log::Level::WARN,
                    filter: "ui=trace,bevy_keith=warn,bevy=info".to_string(),
                    update_subscriber: None,
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "bevy_keith - alpha".to_string(),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::NONE))
        .add_plugins(KeithPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, run)
        .run();
}

#[derive(Component)]
struct MyRes {
    pub font: Handle<Font>,
    pub image: Handle<Image>,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("FiraSans-Regular.ttf");
    let image = asset_server.load("uvdev.png");

    let mut canvas = Canvas::new(Rect {
        min: Vec2::splat(-400.),
        max: Vec2::splat(100.),
    });
    canvas.background_color = None;
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

    let red = ctx.solid_brush(Color::rgba_linear(1.0, 0.0, 0.0, 1.0));
    let green = ctx.solid_brush(Color::rgba_linear(0.0, 1.0, 0.0, 1.0));
    let red50 = ctx.solid_brush(Color::rgba_linear(1.0, 0.0, 0.0, 0.5));
    let green50 = ctx.solid_brush(Color::rgba_linear(0.0, 1.0, 0.0, 0.5));

    let img_rect = Rect::from_center_size(Vec2::splat(10.), Vec2::splat(5.));
    let text = ctx
        .new_layout("text")
        .color(Color::rgb(1., 1., 1.))
        .font(my_res.font.clone())
        .font_size(16.)
        .alignment(JustifyText::Center)
        .build();

    let mut p = Vec2::new(100., 200.);
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red);
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &red,
    );

    p.x += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red);
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &red50,
    );

    p.x += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red50);
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &red50,
    );

    p.x += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red);
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &green,
    );

    p.x += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red);
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &green50,
    );

    p.x += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red50);
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &green50,
    );

    // Row #2 - force write to render target by drawing an image, which will force a
    // separate draw call

    p.x = 100.;
    let delta = Vec2::new(50., 80.);

    p.y += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red);
    ctx.draw_text(text, p - Vec2::Y * 30.);
    ctx.draw_image(img_rect, my_res.image.clone(), ImageScaling::default());
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &red,
    );
    ctx.draw_text(text, p + delta);

    p.x += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red);
    ctx.draw_text(text, p - Vec2::Y * 30.);
    ctx.draw_image(img_rect, my_res.image.clone(), ImageScaling::default());
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &red50,
    );
    ctx.draw_text(text, p + delta);

    p.x += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red50);
    ctx.draw_text(text, p - Vec2::Y * 30.);
    ctx.draw_image(img_rect, my_res.image.clone(), ImageScaling::default());
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &red50,
    );
    ctx.draw_text(text, p + delta);

    p.x += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red);
    ctx.draw_text(text, p - Vec2::Y * 30.);
    ctx.draw_image(img_rect, my_res.image.clone(), ImageScaling::default());
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &green,
    );
    ctx.draw_text(text, p + delta);

    p.x += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red);
    ctx.draw_text(text, p - Vec2::Y * 30.);
    ctx.draw_image(img_rect, my_res.image.clone(), ImageScaling::default());
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &green50,
    );
    ctx.draw_text(text, p + delta);

    p.x += 200.;
    ctx.fill(Rect::from_center_size(p, Vec2::splat(100.)), &red50);
    ctx.draw_text(text, p - Vec2::Y * 30.);
    ctx.draw_image(img_rect, my_res.image.clone(), ImageScaling::default());
    ctx.fill(
        Rect::from_center_size(p + Vec2::splat(50.), Vec2::splat(100.)),
        &green50,
    );
    ctx.draw_text(text, p + delta);
}
