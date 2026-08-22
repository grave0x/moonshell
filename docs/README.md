# Moonshell Mod Format — Documentation

*Draft v0.1 · pre-implementation · schemas harden with the engine*

The whole game is data. Here's the shape of that data.

## Index

| Doc | Covers |
|---|---|
| [format/races.md](format/races.md) | Enemy races: identity, stats, behaviors, waves |
| [format/weapons.md](format/weapons.md) | Towers: behavior, projectiles, caps, upgrade hooks |
| [format/maps.md](format/maps.md) | Sectors: SVG paths, routes, rounds, orc budget |
| [format/upgrades.md](format/upgrades.md) | The tree: orbs (unlocks), cubes (upgrades), stats |
| [format/scripting.md](format/scripting.md) | Lua dialect, API surface, permission manifest |

## Quick start (the 60-second version)

```bash
# every mod is a folder
mods/my-first-mod/
  manifest.yaml      # id, name, permissions
  races/goblin.yaml  # content files — hot-reloaded
  weapons/...
  maps/...
```

1. Drop the folder in `mods/`.
2. Watch the game hot-reload it (no restart).
3. Or drive the same objects live in the console: `create race.goblin` → `race.goblin.` + Tab → `race.goblin.new(Speed, 5)` → `save()`.

## Example (skeleton, from sample-mod/)

```yaml
# sample-mod/races/critter.yaml
id: critter
name: Lunar Critter
description: Small, fast, and plentiful. The swarm's worker unit.
hp: 1
speed: 2.2
reward_diamonds: 1
script: critter.lua        # optional behavior override
scripting: lua             # dialect (rhai planned v1.x)
```

## Permission manifest

```yaml
permissions: []            # what the mod may touch; none = sandboxed logic only
# e.g. files.read — read files inside the mod's own folder
#      network     — outbound network access (future)
```

Each permission gets a plain-language explanation in the in-game mod manager.
Mods run in a contained Lua environment: whitelisted globals, memory limit,
runaway-loop guard. This is containment, not isolation — install mods you trust.

## Feedback

Schemas are drafts until the engine (Bevy 0.19, P0+) locks them. Open issues;
the docs are data, and data is moddable.
