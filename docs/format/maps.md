# Maps (lunar sectors)

A map is a sector of the lunar defensive base: the path geometry, routes, round
structure, and orc budget.

## Fields (draft)

| Field | Type | Meaning |
|---|---|---|
| `id` | string | unique; `map.sector_1_1` |
| `name` | string | e.g. "Sector 1 — Crater Rim" |
| `path_svg` | string | SVG/BMP file: the route(s) orcs follow (vector data = moddable art) |
| `routes` | int | number of entrances/paths (original's late maps: multi-route) |
| `rounds` | int | rounds per level (1 = original parity; >1 = the multi-round demo mod) |
| `orc_budget` | int | total orcs to clear (original: 400 on map 1 → 8,000+ later) |
| `hp` | int | base HP (original: 80) |
| `spawn_rate` | object | orcs/sec, ramp curve |
| `wave_schedule` | list? | per-round composition: race ids + counts (continuous spawn, not discrete waves) |
| `starting_diamonds` | int | resources granted at battle start |
| `path` | list of [x, y] | **draft (P1-M1):** gameplay route as a polyline in world px — the flow field is built from this; `path_svg` stays the art reference |
| `starting_towers` | list of {weapon, x, y} | **draft (P1-M1):** pre-placed towers (temporary until the build UI lands); `weapon` references a `weapons/*.yaml` id |
| `par_time` | number? | seconds (records, not leaderboards) |

## Example

```yaml
id: sector_1_1
name: Sector 1 - Crater Rim
path_svg: maps/sector_1_1.svg
routes: 1
rounds: 3                 # the multi-round demo mod: 3 escalating rounds
orc_budget: 400
hp: 80
spawn_rate: { base: 12, ramp: 0.05 }   # orcs/sec, ramping
wave_schedule:
  - round: 1
    orcs: [{ race: critter, count: 150 }]
  - round: 2
    orcs: [{ race: critter, count: 150 }]
  - round: 3
    orcs: [{ race: critter, count: 100 }]
starting_diamonds: 50
path:                      # polyline the orcs follow (world px)
  - [80, 450]
  - [300, 450]
  - [420, 250]
  - [1520, 450]
starting_towers:           # pre-placed demo towers (until build UI)
  - { weapon: arc_weaver, x: 430, y: 380 }
  - { weapon: lance, x: 730, y: 700 }
```

## Notes

- The original's maps were chicanes and multi-route puzzles; v1 ships 3 sectors.
- `path` (draft): at least 2 points; engine validates (`content.rs`) and builds the flow field from it.
- `starting_towers` (draft): engine places them at battle start (`sim.rs`); to be replaced by the build UI.
- Reference sample: `mods/maps/sector_1_1.yaml`.
- Round count, routes, and budget are *data* — community maps scale the game
  forever, which is the point.
