# Weapons (towers)

A weapon is a tower: firing behavior, projectile, stats, placement cap, and hooks
for its upgrade tree.

## Fields (draft)

| Field | Type | Meaning |
|---|---|---|
| `id` | string | unique; console path `weapon.vulcan` |
| `name` | string | display name |
| `description` | string | tooltip |
| `sprite` | string | SVG reference (vector art is data) |
| `cost_orbs` | number | orange orbs to unlock on the tree (red/weapon nodes) |
| `cap` | number | max placements per map (observed original: 5/6, 1/2) |
| `damage` | number | damage per hit |
| `fire_rate` | number | shots per second |
| `range` | number | pixels |
| `targeting` | string | `first` / `last` / `strong` / `manual` (drag-to-aim) |
| `projectile` | object | `kind` discriminator + fields. Kinds (draft): `bolt` (speed), `hitscan` (pierce, knockback), `chain` (chains, chain_falloff), `splash` (radius), `bounce` (bounces), `burn` (dps) |
| `tick_damage` | number? | DoT (flamethrower burn) |
| `script` | string? | Lua override for novel behavior |
| `upgrades` | list | ids of nodes in the upgrade tree (pink cubes = per-weapon upgrades) |

## Example (the demo mod's tower #1 — a beam that chains)

```yaml
id: arc_weaver
name: Arc Weaver
description: Chains between bio-signatures. Voltage is not a suggestion.
sprite: towers/arc_weaver.svg
cost_orbs: 1
cap: 4
damage: 12
fire_rate: 2.0
range: 220
targeting: strong
projectile: { kind: chain, chains: 5, chain_falloff: 0.8 }
script: arc_weaver.lua
upgrades: [arc_weaver_i, arc_weaver_ii, arc_weaver_iii]
```

## Design intent

The original's towers: machine gunner, wall-bounce laser, mortar, cannon,
flamethrower, Tesla coil, grenade launcher, manual turrets. Moonshell v1 ships 5;
parity arrives through mods — the community's towers are content, not engine
work. The demo mod's 3 towers must have *different design philosophies*
(single-target nuke, crowd control, economy/utility) to prove the schema's range.
