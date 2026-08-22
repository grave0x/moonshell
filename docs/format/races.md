# Races (enemy definitions)

A race is one enemy identity + how it behaves + how waves compose it.

## Fields (draft)

| Field | Type | Meaning |
|---|---|---|
| `id` | string | unique, lowercase, used by maps and console paths (`race.critter`) |
| `name` | string | display name |
| `description` | string | shown in bestiary/tooltips |
| `hp` | number | health |
| `speed` | number | movement speed along the path |
| `reward_diamonds` | number | blue diamonds dropped per kill |
| `damage_to_hp` | number | HP drained from the base if it leaks |
| `silver_chance` | number (0-1) | chance to spawn as silver (+1 diamond, from the original's meta) |
| `explosive_chance` | number (0-1) | chance to explode on death (meta node) |
| `script` | string? | Lua behavior override (movement/attack/wave logic) |
| `scripting` | string | `lua` (default) · `rhai` planned v1.x |
| `tint` | string | SVG/color reference for the sprite (vector art is data) |

## Example

```yaml
id: critter
name: Lunar Critter
description: Small, fast, and plentiful. The swarm's worker unit.
hp: 1
speed: 2.2
reward_diamonds: 1
damage_to_hp: 1
silver_chance: 0.01
tint: "#7CFC00"
```

## Behaviors (scripting)

A race without a script uses defaults: walk the path, leak at the end, die to
damage. A script can override movement, add splitting/regeneration/immunities —
anything the game API exposes. See scripting.md.

## The demo-mod contract

v1's acceptance test is a race that **behaves differently**, not just stat-block
different: e.g., one that splits on death, or regenerates, or flies over terrain.
If that ships as pure YAML + Lua, the modding promise is proven.
