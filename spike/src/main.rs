//! Moonshell P0 performance spike.
//!
//! Question: can Bevy 0.19 move AND render 100k entities at 60fps on mid hardware?
//!
//! Usage:
//!   moonshell-spike sim [ENTITIES] [SECONDS]      # ECS + logic only, no rendering
//!   moonshell-spike sprites [ENTITIES] [SECONDS]  # ECS + logic + Bevy 2D sprites
//!   moonshell-spike flow [ENTITIES] [SECONDS]     # flow-field pathfinding, no rendering
//!   moonshell-spike flow-sprites [ENTITIES] [SECONDS] [ZOOM]  # flow field + rendering
//!   moonshell-spike instanced [ENTITIES] [SECONDS] [ZOOM]     # ONE instanced draw call
//!   moonshell-spike flow-instanced [ENTITIES] [SECONDS] [ZOOM]  # flow field + one instanced draw call
//!
//! The first WARMUP_SECONDS of any run are excluded from the measurement window.

use bevy::prelude::*;
use bevy::a11y::AccessibilityPlugin;
use bevy::app::{AppExit, Plugin, ScheduleRunnerPlugin};
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::camera::CameraPlugin;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::core_2d::Transparent2d;
use bevy::ecs::message::MessageWriter;
use bevy::ecs::query::QueryItem;
use bevy::ecs::system::{
    lifetimeless::{Read, SRes},
    SystemParamItem,
};
use bevy::image::ImagePlugin;
use bevy::input::InputPlugin;
use bevy::input_focus::{InputDispatchPlugin, InputFocusPlugin};
use bevy::log::LogPlugin;
use bevy::mesh::MeshPlugin;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::mesh::{allocator::MeshAllocator, RenderMesh, RenderMeshBufferInfo};
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_phase::{
    AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
    RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
};
use bevy::render::render_resource::*;
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::sync_component::SyncComponent;
use bevy::render::sync_world::MainEntity;
use bevy::render::view::{ExtractedView, NoIndirectDrawing};
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use bevy::render::RenderPlugin;
use bevy::sprite::SpritePlugin;
use bevy::sprite_render::{
    init_mesh_2d_pipeline, Mesh2dPipeline, Mesh2dPipelineKey, RenderMesh2dInstance,
    RenderMesh2dInstances, SetMesh2dBindGroup, SetMesh2dViewBindGroup, ViewKeyCache,
};
use bevy::sprite_render::SpriteRenderPlugin;
use bevy::math::FloatOrd;
use bevy::mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout};
use bevy::shader::Shader;
use bevy::time::TimePlugin;
use bevy::transform::TransformPlugin;
use bevy::window::{PresentMode, PrimaryWindow, WindowResolution};
use bevy::winit::WinitPlugin;
use std::time::{Duration, Instant};

const DEFAULT_ENTITIES: usize = 100_000;
const DEFAULT_SECONDS: f64 = 15.0;
const WARMUP_SECONDS: f64 = 4.0;

// Battle-field-sized world; orcs stream from the cave (left) to the base (right).
const WORLD_W: f32 = 1600.0;
const WORLD_H: f32 = 900.0;
const ORC_SIZE: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Sim,
    Sprites,
    Flow,
    FlowSprites,
    Instanced,
    FlowInstanced,
}

impl Mode {
    fn name(&self) -> &'static str {
        match self {
            Mode::Sim => "sim",
            Mode::Sprites => "sprites",
            Mode::Flow => "flow",
            Mode::FlowSprites => "flow-sprites",
            Mode::Instanced => "instanced",
            Mode::FlowInstanced => "flow-instanced",
        }
    }
}

/// Runtime configuration (CLI args mirrored into a resource so systems can read it).
#[derive(Resource)]
pub struct SpikeConfig {
    pub entities: usize,
    pub seconds: f64,
    pub zoom: f32,
    /// Fixed simulation tick rate in Hz. 0 = variable dt (run as fast as possible).
    pub tick_rate: u32,
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// One orc. `progress` walks 0..=1 along the path; `lane` spreads the horde
/// laterally so sprites don't stack into a single column.
#[derive(Component)]
pub struct Orc {
    pub progress: f32,
    pub speed: f32, // path-fraction per second
    pub lane: f32,  // lateral offset, px
    pub hp: f32,
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

#[derive(Resource)]
pub struct SpikeStats {
    pub mode: Mode,
    pub entities: usize,
    pub seconds: f64,
    pub frame_ms: Vec<f32>,
    pub t0: Instant,
    pub measure_started: Option<Instant>,
    pub spawn_started: Option<Instant>,
    pub spawn_ms: Option<f64>,
    pub last_print: Instant,
    pub extra: String,
    pub tick_rate: u32,
    pub done: bool,
}

impl SpikeStats {
    fn with_ticks(mut self, ticks: u32) -> Self {
        self.tick_rate = ticks;
        self
    }
}

impl SpikeStats {
    fn new(mode: Mode, entities: usize, seconds: f64) -> Self {
        Self {
            mode,
            entities,
            seconds,
            frame_ms: Vec::with_capacity((seconds * 240.0) as usize),
            t0: Instant::now(),
            measure_started: None,
            spawn_started: None,
            spawn_ms: None,
            last_print: Instant::now(),
            extra: String::new(),
            tick_rate: 0,
            done: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Simulation
// ---------------------------------------------------------------------------

/// Path through the lunar sector: a wavy corridor built from two sine terms
/// (cheap, representative per-entity motion cost).
fn path_pos(progress: f32, lane: f32) -> Vec2 {
    let x = progress * WORLD_W;
    let y = WORLD_H * 0.5
        + (progress * std::f32::consts::TAU * 2.5).sin() * 140.0
        + (progress * std::f32::consts::TAU * 11.0).cos() * 30.0
        + lane * 16.0;
    Vec2::new(x, y)
}

/// Cheap deterministic palette: greens/greys by index (~1% silver).
fn orcid_color(i: usize) -> Color {
    if i % 97 == 0 {
        Color::srgb(0.75, 0.78, 0.82) // silver
    } else {
        let g = 0.55 + 0.25 * ((i % 5) as f32 / 5.0);
        Color::srgb(0.12, g, 0.22)
    }
}

fn spawn_orcs(
    mut commands: Commands,
    mut stats: ResMut<SpikeStats>,
    cfg: Res<SpikeConfig>,
) {
    stats.spawn_started = Some(Instant::now());
    let n = cfg.entities;

    let mut batch: Vec<(Transform, Sprite, Orc)> = Vec::with_capacity(n);
    for i in 0..n {
        let progress = (i as f32 + 0.5) / n as f32;
        let speed = 0.020 + (i % 7) as f32 * 0.0012; // 2.0%..2.7% path/s
        let lane = (i % 11) as f32 - 5.0;
        let p = path_pos(progress, lane);
        batch.push((
            Transform::from_translation(p.extend(0.0)),
            Sprite::from_color(orcid_color(i), Vec2::splat(ORC_SIZE)),
            Orc {
                progress,
                speed,
                lane,
                hp: 1.0 + (i % 3) as f32,
            },
        ));
    }
    commands.spawn_batch(batch);
    stats.spawn_ms = Some(stats.spawn_started.unwrap().elapsed().as_secs_f64() * 1000.0);
    info!(
        "queued spawn of {n} orcs in {:.1} ms",
        stats.spawn_ms.unwrap()
    );
}

fn step_orcs(dt: f32, q: &mut Query<(&mut Transform, &mut Orc)>) {
    for (mut tf, mut orc) in q {
        orc.progress += orc.speed * dt;
        if orc.progress > 1.0 {
            orc.progress -= 1.0; // wrapped: the horde keeps streaming
        }
        let p = path_pos(orc.progress, orc.lane);
        tf.translation.x = p.x;
        tf.translation.y = p.y;
    }
}

fn move_orcs(time: Res<Time>, cfg: Res<SpikeConfig>, mut q: Query<(&mut Transform, &mut Orc)>) {
    let frame_dt = time.delta_secs_f64() as f32;
    if cfg.tick_rate == 0 {
        step_orcs(frame_dt, &mut q);
    } else {
        fixed_step(frame_dt, cfg.tick_rate, &mut q, |dt, q| step_orcs(dt, q));
    }
}

// ---------------------------------------------------------------------------
// Pathfinding: flow field (horde-scale routing) + A* reference benchmark
// ---------------------------------------------------------------------------
//
// The game's routing question at 100k orcs is NOT "100k A* queries/frame".
// The horde-scale answer is a flow field: build a direction grid once per map
// change (Dijkstra from the base), then every orc does ONE grid lookup per
// frame. This mode measures (a) field build cost, (b) 100k follow-the-field
// lookups per frame, and (c) an A* single-query benchmark so the comparison
// is on paper: per-entity A* at 100k is obviously infeasible, flow field is not.

const CELL: f32 = 16.0; // grid cell size, world px
const GRID_W: usize = 100; // 1600 px
const GRID_H: usize = 57; // 912 px (covers 900 px world)

#[derive(Component)]
pub struct FlowOrc {
    pub pos: Vec2,
    pub speed: f32, // px/s
    pub seed: u32,  // deterministic respawn lane
}

#[derive(Resource)]
pub struct FlowGrid {
    pub w: usize,
    pub h: usize,
    pub dist: Vec<f32>,
    pub flow: Vec<Vec2>,
    pub build_ms: f64,
}

/// Obstacles in cell coordinates (x0, y0, x1, y1, exclusive), plus one blob.
/// They force a serpentine route through the sector (crater walls).
fn obstacle_cells() -> (Vec<(i32, i32, i32, i32)>, (i32, i32, i32)) {
    let rects = vec![
        (25, 0, 27, 19),   // upper-left wall
        (25, 38, 27, 57),  // lower-left wall
        (50, 6, 52, 16),   // upper-mid wall
        (50, 36, 52, 57),  // lower-mid wall
    ];
    let blob = (70, 28, 7); // center-right crater (cx, cy, radius cells)
    (rects, blob)
}

fn cell_passable(w: usize, h: usize, cx: i32, cy: i32, rects: &[(i32, i32, i32, i32)], blob: &(i32, i32, i32)) -> bool {
    if cx < 0 || cy < 0 || cx >= w as i32 || cy >= h as i32 {
        return false;
    }
    for &(x0, y0, x1, y1) in rects {
        if cx >= x0 && cx < x1 && cy >= y0 && cy < y1 {
            return false;
        }
    }
    let (bx, by, br) = *blob;
    let dx = cx - bx;
    let dy = cy - by;
    if dx * dx + dy * dy < br * br {
        return false;
    }
    true
}

fn build_flow_grid() -> FlowGrid {
    let t0 = Instant::now();
    let (rects, blob) = obstacle_cells();
    let w = GRID_W;
    let h = GRID_H;
    let n = w * h;
    let mut dist = vec![f32::INFINITY; n];
    let mut heap: std::collections::BinaryHeap<std::cmp::Reverse<(i32, usize)>> =
        std::collections::BinaryHeap::new();

    // Multi-source: every passable cell on the right edge is the base.
    for y in 0..h as i32 {
        if cell_passable(w, h, w as i32 - 1, y, &rects, &blob) {
            let idx = y as usize * w + (w - 1);
            dist[idx] = 0.0;
            heap.push(std::cmp::Reverse((0, idx)));
        }
    }

    const DX: [i32; 4] = [1, -1, 0, 0];
    const DY: [i32; 4] = [0, 0, 1, -1];
    while let Some(std::cmp::Reverse((d, idx))) = heap.pop() {
        if d as f32 > dist[idx] {
            continue;
        }
        let cx = (idx % w) as i32;
        let cy = (idx / w) as i32;
        for k in 0..4 {
            let nx = cx + DX[k];
            let ny = cy + DY[k];
            if !cell_passable(w, h, nx, ny, &rects, &blob) {
                continue;
            }
            let nidx = ny as usize * w + nx as usize;
            let nd = d + 1;
            if (nd as f32) < dist[nidx] {
                dist[nidx] = nd as f32;
                heap.push(std::cmp::Reverse((nd, nidx)));
            }
        }
    }

    // Flow: move toward the neighbor with the smallest distance; default east.
    let mut flow = vec![Vec2::ZERO; n];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let idx = (y as usize) * w + (x as usize);
            if !cell_passable(w, h, x, y, &rects, &blob) {
                continue;
            }
            let mut best = dist[idx];
            let mut dir = Vec2::new(1.0, 0.0);
            for k in 0..4 {
                let nx = x + DX[k];
                let ny = y + DY[k];
                if !cell_passable(w, h, nx, ny, &rects, &blob) {
                    continue;
                }
                let nd = dist[ny as usize * w + nx as usize];
                if nd < best {
                    best = nd;
                    dir = Vec2::new(DX[k] as f32, DY[k] as f32);
                }
            }
            flow[idx] = dir.normalize_or_zero();
        }
    }
    let build_ms = t0.elapsed().as_secs_f64() * 1000.0;
    FlowGrid {
        w,
        h,
        dist,
        flow,
        build_ms,
    }
}

impl FlowGrid {
    fn idx(&self, x: f32, y: f32) -> usize {
        let cx = (x / CELL).clamp(0.0, (self.w - 1) as f32) as usize;
        let cy = (y / CELL).clamp(0.0, (self.h - 1) as f32) as usize;
        cy * self.w + cx
    }

    fn direction_at(&self, pos: Vec2) -> Vec2 {
        let d = self.flow[self.idx(pos.x, pos.y)];
        if d.length_squared() > 0.0 {
            d
        } else {
            Vec2::new(1.0, 0.0) // in an obstacle: drift east; perf test, not gameplay
        }
    }
}

/// Classic 4-directional A* on the same grid. Returns path cost (steps).
fn astar_cost(sx: i32, sy: i32, rects: &[(i32, i32, i32, i32)], blob: &(i32, i32, i32)) -> Option<u32> {
    let w = GRID_W as i32;
    let h = GRID_H as i32;
    if !cell_passable(GRID_W, GRID_H, sx, sy, rects, blob) {
        return None;
    }
    let goal_x = w - 1;
    let mut g = vec![u32::MAX; (w * h) as usize];
    let mut heap: std::collections::BinaryHeap<std::cmp::Reverse<(u32, i32, i32)>> =
        std::collections::BinaryHeap::new();
    let start = (sy * w + sx) as usize;
    g[start] = 0;
    let h_fn = |x: i32, _y: i32| -> u32 { (goal_x - x).unsigned_abs() };
    heap.push(std::cmp::Reverse((h_fn(sx, sy), sx, sy)));
    const DX: [i32; 4] = [1, -1, 0, 0];
    const DY: [i32; 4] = [0, 0, 1, -1];
    while let Some(std::cmp::Reverse((_, x, y))) = heap.pop() {
        if x == goal_x {
            return Some(g[(y * w + x) as usize]);
        }
        let cur = g[(y * w + x) as usize];
        for k in 0..4 {
            let nx = x + DX[k];
            let ny = y + DY[k];
            if !cell_passable(GRID_W, GRID_H, nx, ny, rects, blob) {
                continue;
            }
            let nidx = (ny * w + nx) as usize;
            if cur + 1 < g[nidx] {
                g[nidx] = cur + 1;
                heap.push(std::cmp::Reverse((cur + 1 + h_fn(nx, ny), nx, ny)));
            }
        }
    }
    None
}

/// Reference benchmark: cost of ONE A* query on the same grid (would be per
/// orc per frame in a naive design). Prints avg/min/max across `queries` runs.
fn bench_astar(queries: usize) -> (f64, f64, f64) {
    let (rects, blob) = obstacle_cells();
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rnd = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut times = Vec::with_capacity(queries);
    let mut done = 0;
    while done < queries {
        let sx = ((rnd() % (GRID_W as u64 / 2)) as i32).max(0);
        let sy = ((rnd() % GRID_H as u64) as i32).max(0);
        if !cell_passable(GRID_W, GRID_H, sx, sy, &rects, &blob) {
            continue;
        }
        let t0 = Instant::now();
        let _ = astar_cost(sx, sy, &rects, &blob);
        times.push(t0.elapsed().as_secs_f64() * 1e6);
        done += 1;
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    (avg, times[0], times[times.len() - 1])
}

fn setup_flow(mut commands: Commands, mut stats: ResMut<SpikeStats>) {
    let grid = build_flow_grid();
    info!(
        "flow grid {}x{} built in {:.2} ms ({} obstacles)",
        grid.w,
        grid.h,
        grid.build_ms,
        obstacle_cells().0.len() + 1
    );
    let (avg, mn, mx) = bench_astar(200);
    info!(
        "A* reference: avg {:.1} us/query, min {:.1} us, max {:.1} us — at 100k orcs/frame that would be {:.2} s/frame (infeasible); flow field replaces it with one table lookup",
        avg, mn, mx, avg * 1e-6 * 100_000.0
    );
    stats.extra = format!("grid_ms={:.2} astar_us={:.1}", grid.build_ms, avg);
    commands.insert_resource(grid);
}

fn spawn_flow_orcs(
    mut commands: Commands,
    mut stats: ResMut<SpikeStats>,
    cfg: Res<SpikeConfig>,
) {
    stats.spawn_started = Some(Instant::now());
    let n = cfg.entities;
    let mut batch: Vec<(Transform, Sprite, FlowOrc)> = Vec::with_capacity(n);
    for i in 0..n {
        let y = 30.0 + (i % GRID_H) as f32 * 15.0; // spread over the cave mouth
        let speed = 60.0 + (i % 9) as f32 * 5.0; // px/s
        batch.push((
            Transform::from_translation(Vec3::new(8.0, y, 0.0)),
            Sprite::from_color(orcid_color(i), Vec2::splat(ORC_SIZE)),
            FlowOrc {
                pos: Vec2::new(8.0, y),
                speed,
                seed: i as u32,
            },
        ));
    }
    commands.spawn_batch(batch);
    stats.spawn_ms = Some(stats.spawn_started.unwrap().elapsed().as_secs_f64() * 1000.0);
    info!(
        "queued spawn of {n} flow orcs in {:.1} ms",
        stats.spawn_ms.unwrap()
    );
}

/// Spawn 100k flow-field orcs WITHOUT per-entity sprites — the instanced proxy
/// draws them (same layout/movement as `flow-sprites`, one instanced draw call).
fn spawn_flow_orcs_plain(mut commands: Commands, mut stats: ResMut<SpikeStats>, cfg: Res<SpikeConfig>) {
    stats.spawn_started = Some(Instant::now());
    let n = cfg.entities;
    let mut batch: Vec<(Transform, FlowOrc)> = Vec::with_capacity(n);
    for i in 0..n {
        let y = 30.0 + (i % GRID_H) as f32 * 15.0; // spread over the cave mouth
        let speed = 60.0 + (i % 9) as f32 * 5.0; // px/s
        batch.push((
            Transform::from_translation(Vec3::new(8.0, y, 0.0)),
            FlowOrc {
                pos: Vec2::new(8.0, y),
                speed,
                seed: i as u32,
            },
        ));
    }
    commands.spawn_batch(batch);
    stats.spawn_ms = Some(stats.spawn_started.unwrap().elapsed().as_secs_f64() * 1000.0);
    info!(
        "queued spawn of {n} flow orcs (instanced) in {:.1} ms",
        stats.spawn_ms.unwrap()
    );
}

fn step_flow_orcs(
    dt: f32,
    grid: &FlowGrid,
    q: &mut Query<(&mut Transform, &mut FlowOrc)>,
) {
    for (mut tf, mut orc) in q {
        let d = grid.direction_at(orc.pos);
        let speed = orc.speed;
        orc.pos += d * speed * dt;
        if orc.pos.x > WORLD_W - 16.0 {
            // reached the base → re-stream from the cave
            orc.pos = Vec2::new(8.0, 30.0 + (orc.seed % GRID_H as u32) as f32 * 15.0);
        }
        tf.translation.x = orc.pos.x;
        tf.translation.y = orc.pos.y;
    }
}

fn move_flow_orcs(
    time: Res<Time>,
    cfg: Res<SpikeConfig>,
    grid: Res<FlowGrid>,
    mut q: Query<(&mut Transform, &mut FlowOrc)>,
) {
    let frame_dt = time.delta_secs_f64() as f32;
    if cfg.tick_rate == 0 {
        step_flow_orcs(frame_dt, &grid, &mut q);
    } else {
        fixed_step(frame_dt, cfg.tick_rate, &mut q, |dt, q| step_flow_orcs(dt, &grid, q));
    }
}

/// Run the sim at a fixed tick rate, sub-stepping the frame dt. Caps at 8
/// sub-steps/frame so a hitch doesn't spiral the sim.
fn fixed_step<F, Q>(frame_dt: f32, rate: u32, q: &mut Q, mut step: F)
where
    F: FnMut(f32, &mut Q),
{
    let fixed = 1.0 / rate as f32;
    let mut acc = frame_dt;
    let mut n = 0;
    while acc >= fixed && n < 8 {
        step(fixed, q);
        acc -= fixed;
        n += 1;
    }
}


// ---------------------------------------------------------------------------
// Instanced rendering (the P0 architecture answer)
// ---------------------------------------------------------------------------
//
// The naive per-entity `Sprite` path fails the 100k @ 60 fps gate: per-entity
// extraction on the CPU is the limiter (GPU sits at 19-38% util). The fix this
// mode measures: keep 100k `Orc` entities in the ECS for logic, but render them
// through ONE instanced quad draw call. A main-world system writes 100k
// instances (32 B each) into a Vec; the render world uploads that as a single
// vertex buffer (step mode = Instance) and issues one draw(0..6, 0..N).

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceData {
    pub pos_size: [f32; 4], // x, y, w, h (center-anchored)
    pub color: [f32; 4],    // rgba
}

/// Lives on ONE proxy entity in the main world; rewritten every frame from the
/// orc query, then extracted to the render world.
#[derive(Component, Deref, DerefMut)]
pub struct InstanceMaterialData(pub Vec<InstanceData>);

impl SyncComponent for InstanceMaterialData {
    type Target = Self;
}

impl ExtractComponent for InstanceMaterialData {
    type QueryData = &'static InstanceMaterialData;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(InstanceMaterialData(item.0.clone()))
    }
}

/// Render-world component holding the uploaded GPU instance buffer.
#[derive(Component)]
pub struct OrcInstanceBuffer {
    pub buffer: Buffer,
    pub length: usize,
}

#[derive(Resource)]
pub struct OrcInstancedPipeline {
    shader: Handle<Shader>,
    mesh2d_pipeline: Mesh2dPipeline,
}

impl SpecializedMeshPipeline for OrcInstancedPipeline {
    type Key = Mesh2dPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh2d_pipeline.specialize(key, layout)?;

        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: size_of::<InstanceData>() as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 3, // mesh uses 0 (position), 1-2 (normal/uv)
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: VertexFormat::Float32x4.size(),
                    shader_location: 4,
                },
            ],
        });
        descriptor.fragment.as_mut().unwrap().shader = self.shader.clone();
        Ok(descriptor)
    }
}

fn init_orc_instanced_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh2d_pipeline: Res<Mesh2dPipeline>,
) {
    commands.insert_resource(OrcInstancedPipeline {
        shader: asset_server.add(Shader::from_wgsl(
            include_str!("orc_instanced.wgsl"),
            String::from("orc_instanced.wgsl"),
        )),
        mesh2d_pipeline: mesh2d_pipeline.clone(),
    });
}

fn queue_orc_instances(
    transparent_2d_draw_functions: Res<DrawFunctions<Transparent2d>>,
    orc_pipeline: Res<OrcInstancedPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<OrcInstancedPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    render_mesh2d_instances: Res<RenderMesh2dInstances>,
    material_meshes: Query<(Entity, &MainEntity, &InstanceMaterialData)>,
    mut transparent_2d_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>,
    views: Query<(&MainEntity, &ExtractedView)>,
    view_key_cache: Res<ViewKeyCache>,
    mut tick: Local<u32>,
) {
    let draw_custom = transparent_2d_draw_functions.read().id::<DrawOrcInstancedCmd>();
    *tick += 1;

    for (view_entity, view) in &views {
        let Some(phase) = transparent_2d_phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(&view_key) = view_key_cache.get(view_entity) else {
            continue;
        };
        for (entity, main_entity, inst_data) in &material_meshes {
            if *tick % 120 == 0 && std::env::var("MOONSHELL_DEBUG").is_ok() {
                info!("queue: {} instances available", inst_data.0.len());
            }
            let Some(RenderMesh2dInstance { mesh_asset_id, .. }) =
                render_mesh2d_instances.get(main_entity)
            else {
                continue;
            };
            let Some(mesh) = meshes.get(*mesh_asset_id) else {
                continue;
            };
            let key = view_key
                | Mesh2dPipelineKey::from_primitive_topology_and_strip_index(
                    mesh.primitive_topology(),
                    mesh.index_format(),
                );
            let pipeline = pipelines
                .specialize(&pipeline_cache, &orc_pipeline, key, &mesh.layout)
                .unwrap();
            phase.add_retained(Transparent2d {
                sort_key: FloatOrd(0.0),
                entity: (entity, *main_entity),
                pipeline,
                draw_function: draw_custom,
                batch_range: 0..1,
                extracted_index: 0,
                extra_index: PhaseItemExtraIndex::None,
                indexed: false,
            });
        }
    }
}

fn prepare_orc_instance_buffers(
    mut commands: Commands,
    mut query: Query<(Entity, &InstanceMaterialData, Option<&mut OrcInstanceBuffer>)>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    for (entity, instance_data, existing) in &mut query {
        let bytes = bytemuck::cast_slice(&instance_data.0);
        let len = instance_data.0.len();
        match existing {
            // Reuse the persistent buffer when it is big enough — no per-frame
            // GPU allocation, just one 3.2 MB write.
            Some(mut buf) if buf.buffer.size() >= bytes.len() as u64 => {
                render_queue.write_buffer(&buf.buffer, 0, bytes);
                buf.length = len;
            }
            _ => {
                let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("orc instance buffer"),
                    contents: bytes,
                    usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
                });
                commands.entity(entity).insert(OrcInstanceBuffer { buffer, length: len });
            }
        }
    }
}

type DrawOrcInstancedCmd = (SetItemPipeline, SetMesh2dViewBindGroup<0>, SetMesh2dBindGroup<1>, DrawOrcInstanced);

pub struct DrawOrcInstanced;

impl<P: PhaseItem> RenderCommand<P> for DrawOrcInstanced {
    type Param = (
        SRes<RenderAssets<RenderMesh>>,
        SRes<RenderMesh2dInstances>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = Read<OrcInstanceBuffer>;

    fn render<'w>(
        item: &P,
        _view: (),
        instance_buffer: Option<&'w OrcInstanceBuffer>,
        (meshes, render_mesh2d_instances, mesh_allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let meshes = meshes.into_inner();
        let render_mesh2d_instances = render_mesh2d_instances.into_inner();
        let mesh_allocator = mesh_allocator.into_inner();

        let Some(RenderMesh2dInstance { mesh_asset_id, .. }) =
            render_mesh2d_instances.get(&item.main_entity())
        else {
            return RenderCommandResult::Skip;
        };
        let Some(gpu_mesh) = meshes.get(*mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_buffer_slice) = mesh_allocator.mesh_vertex_slice(mesh_asset_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(instance_buffer) = instance_buffer else {
            return RenderCommandResult::Skip;
        };

        pass.set_vertex_buffer(0, vertex_buffer_slice.buffer.slice(..));
        pass.set_vertex_buffer(1, instance_buffer.buffer.slice(..));

        let instance_count = instance_buffer.length as u32;
        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_buffer_slice) = mesh_allocator.mesh_index_slice(mesh_asset_id)
                else {
                    return RenderCommandResult::Skip;
                };
                pass.set_index_buffer(index_buffer_slice.buffer.slice(..), *index_format);
                pass.draw_indexed(
                    index_buffer_slice.range.start..(index_buffer_slice.range.start + count),
                    vertex_buffer_slice.range.start as i32,
                    0..instance_count,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_buffer_slice.range, 0..instance_count);
            }
        }
        RenderCommandResult::Success
    }
}

pub struct OrcInstancingPlugin;

impl Plugin for OrcInstancingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<InstanceMaterialData>::default())
            .add_systems(Update, write_orc_instances);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_render_command::<Transparent2d, DrawOrcInstancedCmd>()
            .init_resource::<SpecializedMeshPipelines<OrcInstancedPipeline>>()
            .add_systems(RenderStartup, init_orc_instanced_pipeline.after(init_mesh_2d_pipeline))
            .add_systems(
                Render,
                (
                    queue_orc_instances.in_set(RenderSystems::QueueMeshes),
                    prepare_orc_instance_buffers.in_set(RenderSystems::PrepareResources),
                ),
            );
    }
}

/// One instance per orc, written every frame. Cheap: 100k x 32 B pushes.
/// Serves both the sine-corridor (`Orc`) and flow-field (`FlowOrc`) hordes —
/// a run spawns exactly one kind, so the other query is simply empty.
/// View-space rect for instance culling (world units), computed from the
/// camera transform + ortho scale + actual window size, with an orc-size margin.
fn view_cull_rect(
    camera: &mut Query<(&GlobalTransform, &Projection), With<Camera>>,
    window: Option<&Window>,
) -> Option<Rect> {
    if std::env::var("MOONSHELL_CULL").map(|v| v == "0").unwrap_or(false) {
        return None;
    }
    let (ct, proj) = camera.single().ok()?;
    let win = window?;
    let scale = match proj {
        Projection::Orthographic(p) => p.scale.max(1e-6),
        _ => return None,
    };
    let half = Vec2::new(win.width() / (2.0 * scale), win.height() / (2.0 * scale))
        + Vec2::splat(ORC_SIZE);
    Some(Rect::from_center_size(ct.translation().truncate(), half * 2.0))
}

/// One instance per orc, written every frame. Cheap: 100k x 32 B pushes.
/// View culling skips off-screen orcs (biggest win when zoomed / large maps).
fn write_orc_instances(
    orcs: Query<(&Transform, &Orc)>,
    flow_orcs: Query<(&Transform, &FlowOrc)>,
    mut data: Query<&mut InstanceMaterialData>,
    mut camera: Query<(&GlobalTransform, &Projection), With<Camera>>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut tick: Local<u32>,
) {
    *tick += 1;
    let Ok(mut data) = data.single_mut() else {
        return;
    };
    let view = view_cull_rect(&mut camera, window.single().ok());
    data.0.clear();
    for (tf, orc) in &orcs {
        let p = tf.translation.truncate();
        if let Some(r) = view {
            if !r.contains(p) {
                continue;
            }
        }
        data.0.push(InstanceData {
            pos_size: [p.x, p.y, ORC_SIZE, ORC_SIZE],
            color: instance_color(orc.progress, orc.lane),
        });
    }
    for (tf, orc) in &flow_orcs {
        let p = tf.translation.truncate();
        if let Some(r) = view {
            if !r.contains(p) {
                continue;
            }
        }
        data.0.push(InstanceData {
            pos_size: [p.x, p.y, ORC_SIZE, ORC_SIZE],
            color: flow_instance_color(orc.seed as usize),
        });
    }
    if *tick % 120 == 0 && std::env::var("MOONSHELL_DEBUG").is_ok() {
        info!("instances written: {}", data.0.len());
    }
}

fn instance_color(progress: f32, lane: f32) -> [f32; 4] {
    // MOONSHELL_BRIGHT=1 renders every orc pure green — for screenshot-based
    // verification of the instanced draw (immune to dimming/thresholds).
    if std::env::var("MOONSHELL_BRIGHT").is_ok() {
        return [0.0, 1.0, 0.0, 1.0];
    }
    let g = 0.40 + 0.35 * (1.0 - progress);
    let r = 0.10 + 0.10 * ((lane * 0.5).abs());
    [r, g, 0.18, 1.0]
}

/// Flow-field orc instance color — mirrors `orcid_color` (the naive
/// `flow-sprites` palette) so both complex-map paths render identically.
fn flow_instance_color(i: usize) -> [f32; 4] {
    if std::env::var("MOONSHELL_BRIGHT").is_ok() {
        return [0.0, 1.0, 0.0, 1.0];
    }
    if i % 97 == 0 {
        [0.75, 0.78, 0.82, 1.0] // silver
    } else {
        let g = 0.55 + 0.25 * ((i % 5) as f32 / 5.0);
        [0.12, g, 0.22, 1.0]
    }
}

/// Unit quad (two triangles), center-anchored, POSITION only.
fn quad_mesh() -> Mesh {
    let positions: Vec<[f32; 3]> = vec![
        [-0.5, -0.5, 0.0],
        [0.5, -0.5, 0.0],
        [0.5, 0.5, 0.0],
        [-0.5, -0.5, 0.0],
        [0.5, 0.5, 0.0],
        [-0.5, 0.5, 0.0],
    ];
    Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::RENDER_WORLD)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
}

/// Spawn 100k orcs WITHOUT per-entity sprites (they render via the instance
/// proxy instead).
fn spawn_orcs_plain(mut commands: Commands, mut stats: ResMut<SpikeStats>, cfg: Res<SpikeConfig>) {
    stats.spawn_started = Some(Instant::now());
    let n = cfg.entities;
    let mut batch: Vec<(Transform, Orc)> = Vec::with_capacity(n);
    for i in 0..n {
        let progress = (i as f32 + 0.5) / n as f32;
        let speed = 0.020 + (i % 7) as f32 * 0.0012;
        let lane = (i % 11) as f32 - 5.0;
        let p = path_pos(progress, lane);
        batch.push((
            Transform::from_translation(p.extend(0.0)),
            Orc {
                progress,
                speed,
                lane,
                hp: 1.0 + (i % 3) as f32,
            },
        ));
    }
    commands.spawn_batch(batch);
    stats.spawn_ms = Some(stats.spawn_started.unwrap().elapsed().as_secs_f64() * 1000.0);
    info!(
        "queued spawn of {n} orcs (instanced) in {:.1} ms",
        stats.spawn_ms.unwrap()
    );
}

fn spawn_instance_proxy(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.spawn((
        Mesh2d(meshes.add(quad_mesh())),
        InstanceMaterialData(Vec::with_capacity(200_000)),
        Transform::default(),
        Visibility::default(),
        NoFrustumCulling,
    ));
}

// ---------------------------------------------------------------------------
// Measurement
// ---------------------------------------------------------------------------

fn collect_stats(
    time: Res<Time>,
    mut stats: ResMut<SpikeStats>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if stats.done {
        return;
    }
    let elapsed = stats.t0.elapsed().as_secs_f64();
    if stats.measure_started.is_none() && elapsed >= WARMUP_SECONDS {
        stats.measure_started = Some(Instant::now());
        stats.frame_ms.clear();
        info!(
            "warmup done — measuring for {:.0}s",
            stats.seconds
        );
    }
    let Some(ms) = stats.measure_started else {
        return;
    };
    stats.frame_ms.push(time.delta_secs_f64() as f32 * 1000.0);

    if stats.last_print.elapsed().as_secs_f64() >= 1.0 {
        stats.last_print = Instant::now();
        let n = stats.frame_ms.len().max(1);
        let avg = stats.frame_ms.iter().sum::<f32>() / n as f32;
        info!("  t={:5.1}s  frame avg {:6.2} ms  (~{:5.1} fps)", ms.elapsed().as_secs_f64(), avg, 1000.0 / avg);
    }

    if ms.elapsed().as_secs_f64() >= stats.seconds {
        stats.done = true;
        report(&stats);
        app_exit.write(AppExit::Success);
    }
}

fn report(stats: &SpikeStats) {
    let mut v = stats.frame_ms.clone();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    let pct = |p: f64| -> f32 {
        if n == 0 {
            return 0.0;
        }
        let idx = ((n as f64 - 1.0) * p).round() as usize;
        v[idx]
    };
    let avg = v.iter().sum::<f32>() / n.max(1) as f32;
    let fps = if avg > 0.0 { 1000.0 / avg } else { 0.0 };
    let spawn_ms = stats.spawn_ms.unwrap_or(0.0);
    info!("=== RESULTS ({}) ===", stats.mode.name());
    info!("entities        : {}", stats.entities);
    info!("spawn           : {:.1} ms", spawn_ms);
    info!("frames measured : {}", n);
    info!("avg frame       : {:.2} ms  (~{:.1} fps)", avg, fps);
    info!("p50             : {:.2} ms", pct(0.50));
    info!("p95             : {:.2} ms", pct(0.95));
    info!("p99             : {:.2} ms", pct(0.99));
    info!("max frame       : {:.2} ms", v.last().copied().unwrap_or(0.0));
    let extra = if stats.extra.is_empty() {
        String::new()
    } else {
        format!(" {}", stats.extra)
    };
    println!(
        "RESULT mode={} entities={} spawn_ms={:.1} frames={} avg_ms={:.3} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3} max_ms={:.3} fps={:.1} ticks={}{}",
        stats.mode.name(),
        stats.entities,
        spawn_ms,
        n,
        avg,
        pct(0.50),
        pct(0.95),
        pct(0.99),
        v.last().copied().unwrap_or(0.0),
        fps,
        stats.tick_rate,
        extra
    );
}

// ---------------------------------------------------------------------------
// App entry points
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = match args.get(1).map(|s| s.as_str()) {
        Some("sim") => Mode::Sim,
        Some("sprites") | None => Mode::Sprites,
        Some("flow") => Mode::Flow,
        Some("flow-sprites") => Mode::FlowSprites,
        Some("instanced") => Mode::Instanced,
        Some("flow-instanced") => Mode::FlowInstanced,
        Some(other) => {
            eprintln!(
                "unknown mode '{other}' — usage: moonshell-spike <sim|sprites|flow|flow-sprites|instanced|flow-instanced> [entities] [seconds] [zoom] [ticks]"
            );
            std::process::exit(2);
        }
    };
    let entities = args
        .get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_ENTITIES);
    let seconds = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_SECONDS);
    // Arg 4/5 depend on mode: render modes take zoom then ticks; sim modes take ticks.
    let (zoom, ticks) = match mode {
        Mode::Sim | Mode::Flow => {
            let ticks = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
            (1.0, ticks)
        }
        Mode::Sprites | Mode::FlowSprites | Mode::Instanced | Mode::FlowInstanced => {
            let zoom = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(1.0);
            let ticks = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
            (zoom, ticks)
        }
    };
    let cfg = SpikeConfig {
        entities,
        seconds,
        zoom,
        tick_rate: ticks,
    };
    info!(
        "Moonshell spike | mode={} entities={} seconds={} zoom={} ticks={} (warmup {WARMUP_SECONDS:.0}s)",
        mode.name(),
        entities,
        seconds,
        zoom,
        ticks
    );

    match mode {
        Mode::Sim => run_sim(cfg),
        Mode::Sprites => run_sprites(cfg),
        Mode::Flow => run_flow(cfg, false),
        Mode::FlowSprites => run_flow(cfg, true),
        Mode::Instanced => run_instanced(cfg),
        Mode::FlowInstanced => run_flow_instanced(cfg),
    }
}

/// Sim-loop pacing: ZERO = as fast as possible (variable dt); otherwise a
/// fixed tick interval (e.g. 60 Hz).
fn sim_loop_wait(tick_rate: u32) -> Duration {
    if tick_rate == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(1.0 / tick_rate as f64)
    }
}

fn run_sim(cfg: SpikeConfig) {
    let mut app = App::new();
    app.add_plugins((
        TaskPoolPlugin::default(),
        TimePlugin,
        LogPlugin::default(),
        ScheduleRunnerPlugin::run_loop(sim_loop_wait(cfg.tick_rate)),
    ))
    .insert_resource(SpikeStats::new(Mode::Sim, cfg.entities, cfg.seconds).with_ticks(cfg.tick_rate))
    .insert_resource(cfg)
    .add_systems(Startup, spawn_orcs)
    .add_systems(Update, (move_orcs, collect_stats));
    app.run();
}

fn run_sprites(cfg: SpikeConfig) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        LogPlugin::default(),
        TransformPlugin,
        InputPlugin,
        InputFocusPlugin,
        InputDispatchPlugin,
        WindowPlugin {
            primary_window: Some(Window {
                title: "Moonshell spike — 100k @ 60fps?".to_string(),
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
    ))
    .insert_resource(SpikeStats::new(Mode::Sprites, cfg.entities, cfg.seconds).with_ticks(cfg.tick_rate))
    .insert_resource(cfg)
    .add_systems(Startup, (spawn_camera, spawn_orcs))
    .add_systems(Update, (move_orcs, collect_stats));
    app.run();
}

fn run_flow(cfg: SpikeConfig, render: bool) {
    if render {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            LogPlugin::default(),
            TransformPlugin,
            InputPlugin,
            InputFocusPlugin,
            InputDispatchPlugin,
            WindowPlugin {
                primary_window: Some(Window {
                    title: "Moonshell spike — flow field @ 100k".to_string(),
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
        ))
        .insert_resource(SpikeStats::new(Mode::FlowSprites, cfg.entities, cfg.seconds).with_ticks(cfg.tick_rate))
        .insert_resource(cfg)
        .add_systems(Startup, (setup_flow, spawn_camera, spawn_flow_orcs))
        .add_systems(Update, (move_flow_orcs, collect_stats));
        app.run();
    } else {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            TimePlugin,
            LogPlugin::default(),
            ScheduleRunnerPlugin::run_loop(sim_loop_wait(cfg.tick_rate)),
        ))
        .insert_resource(SpikeStats::new(Mode::Flow, cfg.entities, cfg.seconds).with_ticks(cfg.tick_rate))
        .insert_resource(cfg)
        .add_systems(Startup, (setup_flow, spawn_flow_orcs))
        .add_systems(Update, (move_flow_orcs, collect_stats));
        app.run();
    }
}

fn run_instanced(cfg: SpikeConfig) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        LogPlugin::default(),
        TransformPlugin,
        InputPlugin,
        InputFocusPlugin,
        InputDispatchPlugin,
        WindowPlugin {
            primary_window: Some(Window {
                title: "Moonshell spike — instanced 100k".to_string(),
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
        OrcInstancingPlugin,
    ))
    .insert_resource(SpikeStats::new(Mode::Instanced, cfg.entities, cfg.seconds).with_ticks(cfg.tick_rate))
    .insert_resource(cfg)
    .add_systems(Startup, (spawn_camera_no_indirect, spawn_orcs_plain, spawn_instance_proxy))
    .add_systems(Update, (move_orcs, collect_stats));
    app.run();
}

fn run_flow_instanced(cfg: SpikeConfig) {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        LogPlugin::default(),
        TransformPlugin,
        InputPlugin,
        InputFocusPlugin,
        InputDispatchPlugin,
        WindowPlugin {
            primary_window: Some(Window {
                title: "Moonshell spike — flow field instanced 100k".to_string(),
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
        OrcInstancingPlugin,
    ))
    .insert_resource(SpikeStats::new(Mode::FlowInstanced, cfg.entities, cfg.seconds).with_ticks(cfg.tick_rate))
    .insert_resource(cfg)
    .add_systems(Startup, (setup_flow, spawn_camera_no_indirect, spawn_flow_orcs_plain, spawn_instance_proxy))
    .add_systems(Update, (move_flow_orcs, collect_stats));
    app.run();
}

fn spawn_camera_no_indirect(mut commands: Commands, cfg: Res<SpikeConfig>) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = cfg.zoom;
    commands.spawn((
        Camera2d,
        Camera::default(),
        Projection::Orthographic(proj),
        Transform::from_xyz(WORLD_W * 0.5, WORLD_H * 0.5, 100.0),
        NoIndirectDrawing,
    ));
}

fn spawn_camera(mut commands: Commands, cfg: Res<SpikeConfig>) {
    let mut proj = OrthographicProjection::default_2d();
    proj.scale = cfg.zoom;
    commands.spawn((
        Camera2d,
        Camera::default(),
        Projection::Orthographic(proj),
        Transform::from_xyz(WORLD_W * 0.5, WORLD_H * 0.5, 100.0),
    ));
}
