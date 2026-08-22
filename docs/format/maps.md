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
| `par_time` | number? | seconds (records, not leaderboards) |

## Example

```yaml
id: sector_1_1
name: Sector 1 - Crater Rim
path_svg: maps/sector_1_1.svg
routes: 1
rounds: 3                 # the multi-round demo mod: 3 escalating rounds
orc_budget: 1200
hp: 80
spawn_rate: { base: 12, ramp: 0.05 }   # orcs/sec, ramping
wave_schedule:
  - round: 1
    orcs: [{ race: critter, count: 400 }]
  - round: 2
    orcs: [{ race: critter, count: 400 }, { race: brute, count: 50 }]
```

## Notes

- The original's maps were chicanes and multi-route puzzles; v1 ships 3 sectors.
- Round count, routes, and budget are *data* — community maps scale the game
  forever, which is the point.
