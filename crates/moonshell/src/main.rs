//! Moonshell — P1 core-loop prototype (M1: engine skeleton + content load).
//!
//! Runs the sample mod's Sector 1: towers from YAML render as instanced quads,
//! critter orcs spawn per the wave schedule and follow a flow field built from
//! the map's route corridor. No tower firing yet (M2).

mod content;
mod render;
mod sim;

use bevy::a11y::AccessibilityPlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::camera::CameraPlugin;
use bevy::input::InputPlugin;
use bevy::input_focus::{InputDispatchPlugin, InputFocusPlugin};
use bevy::log::LogPlugin;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::render::view::NoIndirectDrawing;
use bevy::render::RenderPlugin;
use bevy::sprite::SpritePlugin;
use bevy::sprite_render::SpriteRenderPlugin;
use bevy::transform::TransformPlugin;
use bevy::window::{PresentMode, WindowResolution};
use bevy::winit::WinitPlugin;

use content::load_mods;
use render::InstancedRenderPlugin;
use sim::BattlePlugin;

pub const ORC_SIZE: f32 = 6.0;

fn main() {
    let mods_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "mods".to_string());
    let content = match load_mods(std::path::Path::new(&mods_dir)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("ERROR loading mods from {mods_dir}: {e}");
            std::process::exit(1);
        }
    };
    let map = content
        .get_map("sector_1_1")
        .expect("mods/maps/sector_1_1.yaml must exist");

    let (minx, miny, maxx, maxy) = sim::world_bounds(map);
    let world_w = (maxx - minx).max(1.0);
    let world_h = (maxy - miny).max(1.0);
    let center = Vec2::new((minx + maxx) / 2.0, (miny + maxy) / 2.0);
    let zoom = (world_w / 960.0).max(world_h / 540.0).max(0.1);

    info!(
        "Moonshell P1-M1 | mods={mods_dir} map={} world {world_w:.0}x{world_h:.0} center {center:?} zoom {zoom:.2}",
        map.id
    );

    App::new()
        .add_plugins((
            MinimalPlugins,
            LogPlugin::default(),
            TransformPlugin,
            InputPlugin,
            InputFocusPlugin,
            InputDispatchPlugin,
            WindowPlugin {
                primary_window: Some(Window {
                    title: format!("Moonshell — P1 prototype ({})", map.name),
                    resolution: WindowResolution::new(960, 540),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            },
            AccessibilityPlugin,
            AssetPlugin::default(),
            WinitPlugin::default(),
        ))
        .add_plugins((
            RenderPlugin::default(),
            ImagePlugin::default(),
            MeshPlugin,
            CameraPlugin::default(),
            PipelinedRenderingPlugin::default(),
            CorePipelinePlugin::default(),
            SpritePlugin::default(),
            SpriteRenderPlugin,
            InstancedRenderPlugin,
            BattlePlugin { content },
        ))
        .insert_resource(CameraConfig { center, zoom })
        .add_systems(Startup, (spawn_camera, render::spawn_proxies))
        .add_systems(Update, sim::hud_status)
        .run();
}

#[derive(Resource)]
pub struct CameraConfig {
    pub center: Vec2,
    pub zoom: f32,
}

fn spawn_camera(mut commands: Commands, cfg: Res<CameraConfig>) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = cfg.zoom;
    commands.spawn((
        Camera2d,
        Camera::default(),
        Projection::Orthographic(proj),
        Transform::from_xyz(cfg.center.x, cfg.center.y, 100.0),
        NoIndirectDrawing,
    ));
}
