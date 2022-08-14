//! Basic quad and text drawing inside a `Canvas`.

use bevy::{
    prelude::*,
    render::settings::{PowerPreference, WgpuSettings},
    sprite::Rect,
};
use bevy_inspector_egui::WorldInspectorPlugin;

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
            filter: "quad=trace,bevy_keith=debug".to_string(),
        })
        .add_plugins(DefaultPlugins)
        .add_system(bevy::window::close_on_esc)
        .add_plugin(KeithPlugin)
        .add_plugin(WorldInspectorPlugin::new())
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

trait RectEx {
    fn contains(&self, point: Vec2) -> bool;
}

impl RectEx for Rect {
    fn contains(&self, point: Vec2) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
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
    ctx.stroke(rect, &brush, 1.);

    // Text
    let text = ctx
        .new_layout(text.to_owned())
        .color(Color::rgb(0.2, 0.2, 0.2))
        .font(font)
        .font_size(16.)
        .bounds(rect.size())
        .alignment(TextAlignment {
            vertical: VerticalAlign::Center,
            horizontal: HorizontalAlign::Center,
        })
        .build();
    ctx.draw_text(text, (rect.min + rect.max) / 2.);
}

fn run(mut query: Query<(&mut Canvas, &MyRes)>, windows: Res<Windows>, cam: Query<&Camera>) {
    let (mut canvas, my_res) = query.single_mut();
    canvas.clear();

    let mut ctx = canvas.render_context();

    let cursor_pos = if let Some(window) = windows.get_primary() {
        window
            .cursor_position()
            .map(|v| v - Vec2::new(1280., 720.) / 2.) // FIXME - cheap window-to-canvas hard-coded conversion
    } else {
        None
    }
    .unwrap_or(Vec2::NAN);

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
        draw_button(&mut ctx, rect, &format!("Button #{} {}", i, String::from_utf8_lossy(st)), my_res.font.clone(), cursor_pos);
    }
}
