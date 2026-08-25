//! Content model: typed view of the mod YAML tree.
//!
//! Draft schemas live in docs/format/*.md; the sample mod lives in mods/.
//! This module loads + validates a mod directory into typed structs. The
//! lossless yaml-edit round-trip (SPEC §6) is a P2 concern (console save);
//! M1 loads and validates.
//!
//! Many schema fields (names, descriptions, tints, permissions…) are read by
//! M2+ systems (UI tooltips, scripting, mod manager) — allow dead_code here.
#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::log::info;
use bevy::prelude::Resource;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// Schema types (draft v0.1 + P1-M1 additions: map.path, map.starting_towers)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Race {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub hp: f32,
    pub speed: f32,
    pub reward_diamonds: u32,
    pub damage_to_hp: f32,
    #[serde(default)]
    pub silver_chance: f32,
    #[serde(default)]
    pub explosive_chance: f32,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub scripting: Option<String>,
    #[serde(default)]
    pub tint: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Projectile {
    pub kind: String,
    #[serde(default)]
    pub speed: Option<f32>,
    #[serde(default)]
    pub splash_radius: Option<f32>,
    #[serde(default)]
    pub pierce: Option<u32>,
    #[serde(default)]
    pub bounce: Option<u32>,
    #[serde(default)]
    pub chains: Option<u32>,
    #[serde(default)]
    pub chain_falloff: Option<f32>,
    #[serde(default)]
    pub burn: Option<f32>,
    #[serde(default)]
    pub knockback: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Weapon {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sprite: Option<String>,
    pub cost_orbs: u32,
    pub cap: u32,
    pub damage: f32,
    pub fire_rate: f32,
    pub range: f32,
    pub targeting: String,
    #[serde(default)]
    pub projectile: Option<Projectile>,
    #[serde(default)]
    pub tick_damage: Option<f32>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub upgrades: Vec<String>,
}

#[derive(Debug, Clone, Default, Resource, Deserialize)]
pub struct Cost {
    #[serde(default)]
    pub orbs: u32,
    #[serde(default)]
    pub cubes: u32,
    #[serde(default)]
    pub diamonds: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Upgrade {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub weapon: Option<String>,
    #[serde(default)]
    pub cost: Cost,
    pub rank_max: u32,
    #[serde(default)]
    pub effect: Option<serde_yaml::Value>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default, Resource, Deserialize)]
pub struct SpawnRate {
    pub base: f32,
    pub ramp: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WaveGroup {
    pub race: String,
    pub count: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WaveRound {
    pub round: u32,
    #[serde(default)]
    pub orcs: Vec<WaveGroup>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartingTower {
    pub weapon: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Resource, Deserialize)]
pub struct MapDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub path_svg: Option<String>,
    /// Gameplay route polyline (world px). P1-M1 draft addition; the engine
    /// builds the flow field from this corridor.
    #[serde(default)]
    pub path: Vec<[f32; 2]>,
    pub routes: u32,
    pub rounds: u32,
    pub orc_budget: u32,
    pub hp: f32,
    #[serde(default)]
    pub spawn_rate: SpawnRate,
    #[serde(default)]
    pub wave_schedule: Vec<WaveRound>,
    #[serde(default)]
    pub starting_diamonds: u32,
    /// Pre-placed towers (data-driven demo; the build phase replaces this
    /// with the player's persisted layout).
    #[serde(default)]
    pub starting_towers: Vec<StartingTower>,
}

// ---------------------------------------------------------------------------
// Loaded content
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct Content {
    pub manifest: Option<Manifest>,
    pub races: HashMap<String, Race>,
    pub weapons: HashMap<String, Weapon>,
    pub upgrades: HashMap<String, Upgrade>,
    pub maps: HashMap<String, MapDef>,
}

impl Content {
    pub fn get_map(&self, id: &str) -> Option<&MapDef> {
        self.maps.get(id)
    }
}

/// Load + validate a mod root (`manifest.yaml`, `races/`, `weapons/`,
/// `upgrades/`, `maps/`). Every cross-reference is resolved and reported
/// with the offending file name.
pub fn load_mods(root: &Path) -> Result<Content, String> {
    let mut content = Content::default();

    let manifest_path = root.join("manifest.yaml");
    if manifest_path.exists() {
        let text = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("cannot read {}: {e}", manifest_path.display()))?;
        content.manifest = Some(
            serde_yaml::from_str(&text)
                .map_err(|e| format!("{}: YAML error: {e}", manifest_path.display()))?,
        );
        info!(
            "manifest: {} v{} ({})",
            content.manifest.as_ref().unwrap().id,
            content.manifest.as_ref().unwrap().version,
            manifest_path.display()
        );
    }

    content.races = load_typed_dir::<Race>(root, "races")?;
    content.weapons = load_typed_dir::<Weapon>(root, "weapons")?;
    content.upgrades = load_typed_dir::<Upgrade>(root, "upgrades")?;
    content.maps = load_typed_dir::<MapDef>(root, "maps")?;

    // --- validation: cross references ---
    for (id, weapon) in &content.weapons {
        for up in &weapon.upgrades {
            if !content.upgrades.contains_key(up) {
                return Err(format!(
                    "weapon `{id}` (mods/weapons/{id}.yaml): upgrade `{up}` does not exist in mods/upgrades/"
                ));
            }
        }
    }
    for (id, upgrade) in &content.upgrades {
        for req in &upgrade.requires {
            if !content.upgrades.contains_key(req) {
                return Err(format!(
                    "upgrade `{id}` (mods/upgrades/{id}.yaml): requires `{req}` which does not exist"
                ));
            }
        }
        if let Some(w) = &upgrade.weapon {
            if !content.weapons.contains_key(w) {
                return Err(format!(
                    "upgrade `{id}`: references weapon `{w}` which does not exist"
                ));
            }
        }
    }
    // requires chains must be acyclic (simple DFS from every node)
    for id in content.upgrades.keys() {
        let mut seen = std::collections::HashSet::new();
        let mut stack = vec![id.clone()];
        while let Some(node) = stack.pop() {
            if !seen.insert(node.clone()) {
                return Err(format!(
                    "upgrade tree is cyclic: `{node}` (mods/upgrades/{node}.yaml)"
                ));
            }
            if let Some(u) = content.upgrades.get(&node) {
                stack.extend(u.requires.iter().cloned());
            }
        }
    }
    for (id, map) in &content.maps {
        if map.path.len() < 2 {
            return Err(format!(
                "map `{id}` (mods/maps/{id}.yaml): `path` needs >= 2 points"
            ));
        }
        for round in &map.wave_schedule {
            for group in &round.orcs {
                if !content.races.contains_key(&group.race) {
                    return Err(format!(
                        "map `{id}` round {}: race `{}` does not exist in mods/races/",
                        round.round, group.race
                    ));
                }
            }
        }
        for tower in &map.starting_towers {
            if !content.weapons.contains_key(&tower.weapon) {
                return Err(format!(
                    "map `{id}`: starting tower weapon `{}` does not exist in mods/weapons/",
                    tower.weapon
                ));
            }
        }
    }

    info!(
        "content loaded: {} races, {} weapons, {} upgrades, {} maps",
        content.races.len(),
        content.weapons.len(),
        content.upgrades.len(),
        content.maps.len()
    );
    Ok(content)
}

/// Load `root/<sub>/**.yaml` keyed by each file's `id` field, reporting
/// missing ids and duplicates with the file path.
fn load_typed_dir<T: serde::de::DeserializeOwned + HasId>(root: &Path, sub: &str) -> Result<HashMap<String, T>, String> {
    let dir = root.join(sub);
    if !dir.exists() {
        return Ok(HashMap::new()); // some mods legitimately lack a folder
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("cannot read {}/: {e}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "yaml" || x == "yml"))
        .collect();
    entries.sort();
    let mut out = HashMap::new();
    for path in entries {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let value: T = serde_yaml::from_str(&text)
            .map_err(|e| format!("{}: YAML error: {e}", path.display()))?;
        let id = value.id();
        if out.insert(id.clone(), value).is_some() {
            return Err(format!("{}: duplicate id `{id}`", path.display()));
        }
        info!("loaded {sub}/{id} ({})", path.display());
    }
    Ok(out)
}

pub trait HasId {
    fn id(&self) -> String;
}

impl HasId for Race {
    fn id(&self) -> String {
        self.id.clone()
    }
}
impl HasId for Weapon {
    fn id(&self) -> String {
        self.id.clone()
    }
}
impl HasId for Upgrade {
    fn id(&self) -> String {
        self.id.clone()
    }
}
impl HasId for MapDef {
    fn id(&self) -> String {
        self.id.clone()
    }
}
