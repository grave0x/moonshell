//! Moonshell splash screen.
//!
//! A horde of instanced dots streams along the P0 spike's sine corridor —
//! the same `path_pos` math and the same one-draw-call instanced renderer
//! that hit 100k entities @ 60 fps. No per-dot assets or bespoke code: the
//! visual is pure data (`progress` + `lane`), mirroring "modding is editing
//! data files".
//!
//! Lifecycle: splash camera + dots + title render first; any key/click or a
//! 3 s timeout tears them down and hands off to the Hub (battle camera + hub).

use bevy::input::keyboard::KeyCode;
use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::render::view::NoIndirectDrawing;

use crate::content::Content;
use crate::sim::Battle;
use crate::ui::{spawn_hub, GameState, Phase};
use crate::{spawn_camera, CameraConfig};

/// Sine-corridor world size — mirrors the P0 spike's `WORLD_W` / `WORLD_H`.
const WORLD_W: f32 = 1600.0;
const WORLD_H: f32 = 900.0;
const SPLASH_DOTS: usize = 6000;
const SPLASH_SECONDS: f32 = 3.0;

/// One streaming dot. `progress` walks 0..=1 along the corridor; `lane`
/// spreads the horde laterally so the dots form a band, not a line.
#[derive(Component)]
pub struct SplashDot {
    pub progress: f32,
    pub lane: f32,
    pub speed: f32,
    pub seed: u32,
}

/// Marker on the splash's own camera (torn down before the battle camera).
#[derive(Component)]
pub struct SplashCamera;

/// Marker on the splash UI root (torn down on hand-off).
#[derive(Component)]
pub struct SplashRoot;

pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (spawn_splash_camera, spawn_splash_dots, spawn_splash_ui).chain(),
        )
        .add_systems(Update, (advance_splash, dismiss_splash).chain());
    }
}

/// The P0 spike's wavy corridor: two sine terms over progress, plus a lane
/// offset. Kept identical to `spike/src/main.rs::path_pos` on purpose.
fn path_pos(progress: f32, lane: f32) -> Vec2 {
    let x = progress * WORLD_W;
    let y = WORLD_H * 0.5
        + (progress * std::f32::consts::TAU * 2.5).sin() * 140.0
        + (progress * std::f32::consts::TAU * 11.0).cos() * 30.0
        + lane * 16.0;
    Vec2::new(x, y)
}

fn spawn_splash_camera(mut commands: Commands) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = (WORLD_W / 960.0).max(WORLD_H / 540.0);
    commands.spawn((
        Camera2d,
        Camera {
            order: -1, // render behind the battle camera during the one-frame hand-off
            ..default()
        },
        Projection::Orthographic(proj),
        Transform::from_xyz(WORLD_W * 0.5, WORLD_H * 0.5, 100.0),
        NoIndirectDrawing,
        SplashCamera,
    ));
}

fn spawn_splash_dots(mut commands: Commands) {
    for i in 0..SPLASH_DOTS {
        let progress = i as f32 / SPLASH_DOTS as f32;
        let lane = (i % 7) as f32 - 3.0;
        let speed = 0.10 + 0.08 * ((i % 5) as f32 / 5.0);
        let p = path_pos(progress, lane);
        commands.spawn((
            SplashDot {
                progress,
                lane,
                speed,
                seed: i as u32,
            },
            Transform::from_translation(p.extend(0.0)),
        ));
    }
}

fn spawn_splash_ui(mut commands: Commands) {
    commands
        .spawn((
            SplashRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Px(10.0),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                Text::new("MOONSHELL"),
                TextFont::from_font_size(64.0),
                TextColor(Color::srgb(0.88, 0.92, 0.96)),
            ));
            p.spawn((
                Text::new("modding is editing data files"),
                TextFont::from_font_size(22.0),
                TextColor(Color::srgb(0.5, 0.78, 0.82)),
            ));
            p.spawn((
                Text::new("horde-scale tower defense · 100k orcs @ 60 fps"),
                TextFont::from_font_size(15.0),
                TextColor(Color::srgb(0.45, 0.5, 0.55)),
            ));
            p.spawn((
                Text::new("press any key"),
                TextFont::from_font_size(14.0),
                TextColor(Color::srgb(0.35, 0.4, 0.45)),
            ));
        });
}

fn advance_splash(time: Res<Time>, mut q: Query<(&mut Transform, &mut SplashDot)>) {
    let dt = time.delta_secs();
    for (mut tf, mut dot) in &mut q {
        dot.progress += dot.speed * dt;
        if dot.progress > 1.0 {
            dot.progress -= 1.0;
        }
        let p = path_pos(dot.progress, dot.lane);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
    }
}

#[allow(clippy::too_many_arguments)]
fn dismiss_splash(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut elapsed: Local<f32>,
    camera_cfg: Res<CameraConfig>,
    content: Res<Content>,
    battle: Res<Battle>,
    mut state: ResMut<GameState>,
    splash_cam: Query<Entity, With<SplashCamera>>,
    dots: Query<Entity, With<SplashDot>>,
    root: Query<Entity, With<SplashRoot>>,
) {
    *elapsed += time.delta_secs();
    let skip = keys.get_just_pressed().next().is_some()
        || mouse.get_just_pressed().next().is_some();
    if *elapsed < SPLASH_SECONDS && !skip {
        return;
    }

    for e in splash_cam.iter().chain(dots.iter()).chain(root.iter()) {
        commands.entity(e).despawn();
    }

    spawn_camera(commands.reborrow(), camera_cfg);
    if std::env::var("MOONSHELL_AUTOPLAY").is_ok() {
        // Headless/CI mode: skip the hub, drop straight into the battle so the
        // whole loop (battle -> win/lose -> hub respawn) can be verified
        // without input.
        info!("autoplay: skipping hub, starting battle");
        state.phase = Phase::Battle;
    } else {
        state.phase = Phase::Hub;
        spawn_hub(commands.reborrow(), content, battle);
    }
}
