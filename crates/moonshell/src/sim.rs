//! Battle simulation (P1-M1): orcs follow a flow field built from the map's
//! route corridor; the spawner honors the wave schedule + budget; leaks drain
//! base HP. Towers are placed + rendered but do not fire yet (M2).

use bevy::prelude::*;

use crate::content::{Content, MapDef, Race};

pub const CELL: f32 = 16.0; // flow-field grid cell, world px

#[derive(Component)]
pub struct Orc {
    pub pos: Vec2,
    pub speed_px: f32, // px/s along the field
    pub seed: u32,
    #[allow(dead_code)] // M2: damage/death
    pub hp: f32,
    #[allow(dead_code)] // M2: kill rewards
    pub reward_diamonds: u32,
    pub damage_to_hp: f32,
}

#[derive(Component)]
pub struct Tower {
    #[allow(dead_code)] // M2: firing
    pub weapon: String,
    #[allow(dead_code)] // M2: targeting
    pub pos: Vec2,
}

/// Grid of flow directions toward the base.
#[derive(Resource)]
pub struct FlowGrid {
    pub w: usize,
    pub h: usize,
    pub flow: Vec<Vec2>,
    #[allow(dead_code)] // diagnostics
    pub build_ms: f64,
}

impl FlowGrid {
    fn idx(&self, x: f32, y: f32) -> usize {
        let cx = (x / CELL).clamp(0.0, (self.w - 1) as f32) as usize;
        let cy = (y / CELL).clamp(0.0, (self.h - 1) as f32) as usize;
        cy * self.w + cx
    }

    pub fn direction_at(&self, pos: Vec2) -> Vec2 {
        let d = self.flow[self.idx(pos.x, pos.y)];
        if d.length_squared() > 0.0 {
            d
        } else {
            Vec2::new(1.0, 0.0)
        }
    }
}

/// Build a flow field from the map's route corridor: cells within
/// CORRIDOR_R of the polyline are passable; multi-source Dijkstra from the
/// path's end; each cell's flow points toward the lowest-cost neighbor.
pub fn build_flow_grid(map: &MapDef) -> FlowGrid {
    let t0 = std::time::Instant::now();
    let w = 100;
    let h = 57;
    let corridor = 22.0_f32; // px
    let end = Vec2::new(map.path[map.path.len() - 1][0], map.path[map.path.len() - 1][1]);

    let passable = |cx: i32, cy: i32| -> bool {
        if cx < 0 || cy < 0 || cx >= w as i32 || cy >= h as i32 {
            return false;
        }
        let p = Vec2::new((cx as f32 + 0.5) * CELL, (cy as f32 + 0.5) * CELL);
        dist_to_polyline(p, &map.path) <= corridor
    };

    let n = w * h;
    let mut dist = vec![f32::INFINITY; n];
    let mut heap: std::collections::BinaryHeap<std::cmp::Reverse<(i32, usize)>> =
        std::collections::BinaryHeap::new();
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let p = Vec2::new((x as f32 + 0.5) * CELL, (y as f32 + 0.5) * CELL);
            if p.distance(end) < corridor {
                let idx = y as usize * w + x as usize;
                dist[idx] = 0.0;
                heap.push(std::cmp::Reverse((0, idx)));
            }
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
            if !passable(nx, ny) {
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
    let mut flow = vec![Vec2::ZERO; n];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let idx = (y as usize) * w + (x as usize);
            if !passable(x, y) {
                continue;
            }
            let mut best = dist[idx];
            let mut dir = Vec2::new(1.0, 0.0);
            for k in 0..4 {
                let nx = x + DX[k];
                let ny = y + DY[k];
                if !passable(nx, ny) {
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
    info!("flow field built in {build_ms:.2} ms");
    FlowGrid { w, h, flow, build_ms }
}

fn dist_to_polyline(p: Vec2, path: &[[f32; 2]]) -> f32 {
    let mut best = f32::INFINITY;
    for w in path.windows(2) {
        let a = Vec2::new(w[0][0], w[0][1]);
        let b = Vec2::new(w[1][0], w[1][1]);
        best = best.min(dist_to_segment(p, a, b));
    }
    best
}

fn dist_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-9 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Battle state: currencies, HP, spawner progress, outcome.
#[derive(Resource)]
pub struct Battle {
    #[allow(dead_code)] // M2: round reporting
    pub map_id: String,
    pub base_hp: f32,
    pub base_hp_max: f32,
    pub diamonds: u32,
    pub orcs_alive: u32,
    pub orcs_spawned: u32,
    pub orc_budget: u32,
    pub kills: u32,
    pub leaks: u32,
    pub elapsed: f32,
    pub spawn_acc: f32,
    pub spawn_rate: f32,
    #[allow(dead_code)] // M2: multi-round demo
    pub rounds_complete: u32,
    pub outcome: Option<BattleOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BattleOutcome {
    Won,
    Lost,
}

impl Battle {
    pub fn new(map: &MapDef) -> Self {
        Self {
            map_id: map.id.clone(),
            base_hp: map.hp,
            base_hp_max: map.hp,
            diamonds: map.starting_diamonds,
            orcs_alive: 0,
            orcs_spawned: 0,
            orc_budget: map.orc_budget,
            kills: 0,
            leaks: 0,
            elapsed: 0.0,
            spawn_acc: 0.0,
            spawn_rate: map.spawn_rate.base,
            rounds_complete: 0,
            outcome: None,
        }
    }
}

/// Spawn orcs from the map's wave schedule at the given rate.
fn spawn_orcs(
    time: Res<Time>,
    mut battle: ResMut<Battle>,
    map: Res<MapDef>,
    races: Res<Races>,
    mut commands: Commands,
) {
    if battle.outcome.is_some() {
        return;
    }
    let dt = time.delta_secs_f64() as f32;
    battle.elapsed += dt;
    battle.spawn_rate = map.spawn_rate.base + map.spawn_rate.ramp * battle.elapsed;

    battle.spawn_acc += dt * battle.spawn_rate;
    let start = Vec2::new(map.path[0][0], map.path[0][1]);
    while battle.spawn_acc >= 1.0 && battle.orcs_spawned < battle.orc_budget {
        battle.spawn_acc -= 1.0;
        battle.orcs_spawned += 1;
        // Which race? Walk the schedule to find the group covering this orc.
        let mut idx = battle.orcs_spawned; // 1-based
        let mut race_id = None;
        'outer: for round in &map.wave_schedule {
            for group in &round.orcs {
                if idx <= group.count {
                    race_id = Some(group.race.clone());
                    break 'outer;
                }
                idx -= group.count;
            }
        }
        let Some(race_id) = race_id else {
            continue;
        };
        let Some(race) = races.0.get(&race_id) else {
            continue;
        };
        let seed = battle.orcs_spawned;
        let lane = ((seed % 9) as f32 - 4.0) * 6.0;
        let pos = start + Vec2::new(0.0, lane);
        commands.spawn(Orc {
            pos,
            speed_px: race.speed * 35.0,
            seed,
            hp: race.hp,
            reward_diamonds: race.reward_diamonds,
            damage_to_hp: race.damage_to_hp,
        });
        battle.orcs_alive += 1;
    }
}

/// Move orcs along the flow field; leak at the base.
fn move_orcs(
    time: Res<Time>,
    grid: Res<FlowGrid>,
    map: Res<MapDef>,
    mut battle: ResMut<Battle>,
    mut q: Query<(Entity, &mut Orc)>,
    mut commands: Commands,
) {
    let dt = time.delta_secs_f64() as f32;
    let end = Vec2::new(map.path[map.path.len() - 1][0], map.path[map.path.len() - 1][1]);
    let mut to_kill = Vec::new();
    for (entity, mut orc) in &mut q {
        let d = grid.direction_at(orc.pos);
        let speed = orc.speed_px;
        orc.pos += d * speed * dt;
        if orc.pos.distance(end) < 18.0 {
            battle.base_hp -= orc.damage_to_hp;
            battle.leaks += 1;
            battle.orcs_alive -= 1;
            to_kill.push(entity);
            if battle.base_hp <= 0.0 && battle.outcome.is_none() {
                battle.base_hp = 0.0;
                battle.outcome = Some(BattleOutcome::Lost);
            }
        }
    }
    for e in to_kill {
        commands.entity(e).despawn();
    }
}

/// End-of-round / win check.
fn check_outcome(mut battle: ResMut<Battle>) {
    if battle.outcome.is_some() {
        return;
    }
    if battle.orcs_spawned >= battle.orc_budget && battle.orcs_alive == 0 {
        battle.outcome = Some(BattleOutcome::Won);
    }
}

#[derive(Resource)]
pub struct Races(pub std::collections::HashMap<String, Race>);

#[allow(dead_code)]
pub const TICK_RATE: u32 = 60;

#[allow(dead_code)]
/// Fixed 60 Hz sub-stepping (M2 uses this; M1 runs variable dt).
pub fn fixed_step<F, Q>(frame_dt: f32, rate: u32, q: &mut Q, mut step: F)
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

pub struct BattlePlugin {
    pub content: Content,
}

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        let map = self.content.get_map("sector_1_1").cloned().expect("sector_1_1");
        let races = Races(self.content.races.clone());
        let grid = build_flow_grid(&map);
        let battle = Battle::new(&map);
        app.insert_resource(map)
            .insert_resource(races)
            .insert_resource(grid)
            .insert_resource(battle)
            .add_systems(Startup, spawn_towers)
            .add_systems(
                Update,
                (
                    spawn_orcs,
                    move_orcs,
                    check_outcome,
                    log_outcome,
                )
                    .chain(),
            );
    }
}

fn spawn_towers(mut commands: Commands, map: Res<MapDef>) {
    for t in &map.starting_towers {
        commands.spawn((
            Tower {
                weapon: t.weapon.clone(),
                pos: Vec2::new(t.x, t.y),
            },
            Transform::from_translation(Vec3::new(t.x, t.y, 0.0)),
        ));
    }
    info!("placed {} starting towers", map.starting_towers.len());
}

fn log_outcome(battle: Res<Battle>, mut last: Local<Option<BattleOutcome>>) {
    if let Some(outcome) = battle.outcome {
        if *last != Some(outcome) {
            match outcome {
                BattleOutcome::Won => {
                    info!(
                        "BATTLE WON — full-clear bonus! kills={} leaks={} diamonds={}",
                        battle.kills, battle.leaks, battle.diamonds
                    )
                }
                BattleOutcome::Lost => {
                    info!(
                        "BATTLE LOST — base HP 0. loot kept: {} diamonds",
                        battle.diamonds
                    )
                }
            }
            *last = Some(outcome);
        }
    }
}

/// Convenience: the map's starting flow (used by the renderer's culling).
pub fn world_bounds(map: &MapDef) -> (f32, f32, f32, f32) {
    let mut minx = f32::MAX;
    let mut miny = f32::MAX;
    let mut maxx = f32::MIN;
    let mut maxy = f32::MIN;
    for p in &map.path {
        minx = minx.min(p[0]);
        miny = miny.min(p[1]);
        maxx = maxx.max(p[0]);
        maxy = maxy.max(p[1]);
    }
    (minx, miny, maxx, maxy)
}

/// HUD-ish status line printed once per second.
pub fn hud_status(battle: Res<Battle>, time: Res<Time>, mut last: Local<f64>) {
    let now = time.elapsed_secs_f64();
    if now - *last >= 1.0 {
        *last = now;
        info!(
            "HP {:.0}/{} | orcs {} alive / {} spawned / {} budget | kills {} | leaks {} | 💎 {} | outcome {:?}",
            battle.base_hp, battle.base_hp_max, battle.orcs_alive, battle.orcs_spawned,
            battle.orc_budget, battle.kills, battle.leaks, battle.diamonds, battle.outcome
        );
    }
}
