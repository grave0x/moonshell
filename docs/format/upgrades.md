# Upgrades (the tree)

The hub's upgrade tree: weapon unlocks (orange orbs), weapon upgrades (pink
cubes), player stats, and meta nodes.

## Fields (draft)

| Field | Type | Meaning |
|---|---|---|
| `id` | string | unique; node paths like `upgrade.cannon_ii` |
| `type` | string | `weapon_unlock` (orbs) · `weapon_upgrade` (cubes) · `stat` · `meta` |
| `weapon` | string? | weapon id this node belongs to |
| `cost` | object | `{ orbs: n }` or `{ cubes: n }` or `{ diamonds: n }` |
| `rank_max` | int | max ranks (observed: fractions like 1/10, 2/12) |
| `effect` | object | what each rank grants (damage %, fire rate %, range, new bullet type…) |
| `requires` | list? | prerequisite node ids (the tree's edges) |
| `icon` | string | SVG reference |
| `color` | string | node color (red=weapon unlocks, pink=upgrades — kept for familiarity, shape-coded too) |

## Example

```yaml
id: cannon_ii
type: weapon_upgrade
weapon: cannon
cost: { cubes: 2 }
rank_max: 10
effect: { damage_pct: 10 }      # +10% damage per rank
requires: [cannon_i]
```

## Meta nodes (from the original's tree, as data)

- money per kill (+diamonds)
- +3 end-of-battle bonus
- 1% explosive orcs
- 1% silver orcs
- player stats: health, missile

## Console access

Nodes are live objects: `upgrade.cannon_ii.` + Tab shows fields;
`upgrade.cannon_ii.new(rank, 3)` mutates; `save()` persists. The tree is data,
and data is moddable.
