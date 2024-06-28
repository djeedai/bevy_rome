//! Basic quad and text drawing inside a `Canvas`.

use bevy::app::AppExit;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::diagnostic::LogDiagnosticsPlugin;
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
                    filter: "quad=trace,bevy_keith=warn,bevy=info".to_string(),
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

#[derive(Component, Default)]
struct MenuState {
    font: Handle<Font>,
    image: Handle<Image>,
    audio_open: bool,
    graphics_open: bool,
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
        .spawn(Camera2dBundle::default())
        .insert(canvas)
        .insert(MenuState {
            font: font.clone(),
            image: image.clone(),
            ..default()
        });
}

fn draw_button(
    ctx: &mut RenderContext,
    rect: Rect,
    text: &str,
    font: Handle<Font>,
    mouse_state: &MouseState,
) -> bool {
    // Shape
    let rounded_rect = RoundedRect { rect, radius: 8. };

    // Background
    let contains = rect.contains(mouse_state.cursor_pos);
    let brush = if contains {
        if mouse_state.pressed {
            ctx.solid_brush(Color::rgb(0.8, 0.8, 0.8))
        } else {
            ctx.solid_brush(Color::rgb(0.7, 0.7, 0.7))
        }
    } else {
        ctx.solid_brush(Color::rgb(0.6, 0.6, 0.6))
    };
    ctx.fill(rounded_rect.clone(), &brush);

    // Outline
    let brush = ctx.solid_brush(Color::rgb(0.5, 0.5, 0.5));
    ctx.stroke(rounded_rect, &brush, 1.);

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

    contains
}

struct MouseState {
    pub cursor_pos: Vec2,
    pub pressed: bool,
    pub just_pressed: bool,
    pub just_released: bool,
}

fn run(
    time: Res<Time>,
    mut query: Query<(&mut Canvas, &mut MenuState)>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    in_mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut ev_app_exit: EventWriter<AppExit>,
) {
    let (mut canvas, mut menu_state) = query.single_mut();
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
    let mouse_state = MouseState {
        cursor_pos,
        pressed: in_mouse_buttons.pressed(MouseButton::Left),
        just_pressed: in_mouse_buttons.just_pressed(MouseButton::Left),
        just_released: in_mouse_buttons.just_released(MouseButton::Left),
    };
    //trace!("cursor_pos={cursor_pos}");

    // ctx.clear(None, Color::FUCHSIA);

    let brush = ctx.solid_brush(Color::GREEN);
    let delta = time.elapsed_seconds().sin() * 15. + 30.;
    for i in 0..=10 {
        ctx.line(
            Vec2::new(-200.5, 0.5 + i as f32 * 15.),
            Vec2::new(0.5, 0.5 + i as f32 * delta),
            &brush,
            1. + i as f32,
        );
    }

    let color = Color::hsl((time.elapsed_seconds() / 3.).fract() * 360., 0.5, 0.5);
    let text = ctx
        .new_layout("bevy_keith")
        .color(color)
        .font(menu_state.font.clone())
        .font_size(128.)
        .build();
    ctx.draw_text(text, Vec2::new(-350., 300.0));

    // Main menu buttons
    let rect = Rect {
        min: Vec2::new(-100., -200.),
        max: Vec2::new(100., -170.),
    };
    if draw_button(
        &mut ctx,
        rect,
        "New Game",
        menu_state.font.clone(),
        &mouse_state,
    ) && mouse_state.pressed
    {
        let rect = Rect {
            min: Vec2::new(100., 150.),
            max: Vec2::new(164., 214.),
        };
        ctx.draw_image(rect, menu_state.image.clone());
    }
    let rect = Rect {
        min: Vec2::new(-100., -160.),
        max: Vec2::new(100., -130.),
    };
    if draw_button(
        &mut ctx,
        rect,
        "Audio Settings",
        menu_state.font.clone(),
        &mouse_state,
    ) && mouse_state.just_released
    {
        menu_state.audio_open = !menu_state.audio_open;
    }
    let rect = Rect {
        min: Vec2::new(-100., -120.),
        max: Vec2::new(100., -90.),
    };
    if draw_button(
        &mut ctx,
        rect,
        "Graphics Settings",
        menu_state.font.clone(),
        &mouse_state,
    ) && mouse_state.just_released
    {
        menu_state.graphics_open = !menu_state.graphics_open;
    }
    let rect = Rect {
        min: Vec2::new(-100., -80.),
        max: Vec2::new(100., -50.),
    };
    if draw_button(
        &mut ctx,
        rect,
        "Exit",
        menu_state.font.clone(),
        &mouse_state,
    ) {
        let rect = Rect {
            min: mouse_state.cursor_pos,
            max: mouse_state.cursor_pos + Vec2::new(200., 20.),
        };
        let rrect = RoundedRect { rect, radius: 10. };
        let brush = ctx.solid_brush(Color::rgb(0.8, 0.8, 0.8));
        ctx.fill(rrect, &brush);

        let text = ctx
            .new_layout("Release the Left Mouse Button while inside the menu to exit the app.")
            .color(Color::BLACK)
            .font(menu_state.font.clone())
            .font_size(10.)
            .anchor(Anchor::CenterLeft)
            .bounds(rect.size())
            .build();
        ctx.draw_text(text, rect.center() + Vec2::X * (8. - rect.half_size().x));

        if mouse_state.just_released {
            ev_app_exit.send(AppExit);
        }
    }
}
