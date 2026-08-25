# Sample Mod — Contract Audit

*Audited 2026-08-24 against `docs/format/*.md` (draft v0.1 schemas). Read-only review + small fixes.*

## Verdict

**Structurally sound.** All 14 YAML files parse; every weapon→upgrade reference
resolves; every upgrade tree chain (`_i → _ii → _iii`) is valid; all weapon and
upgrade fields match the documented schema. The 3 towers cover 3 philosophies
(crowd control / single-target nuke / economy), and the race overrides behavior
(split-on-death), satisfying the demo-mod contract from `races.md`.

## Issues found & fixed

| # | Severity | Issue | Fix |
|---|---|---|---|
| 1 | **Bug (high)** | `critter.lua` split-on-death never triggered: guard is `if ent.hp <= 1 then return`, and the critter's `hp: 1` — the flagship "behaves differently" demo was dead code for its own stats | `critter.yaml`: `hp: 1 → 2` (splits into two 1-hp children) |
| 2 | **Doc gap** | `scripting.md` documented no script **hook contract** (`on_death` / `on_kill` signatures) even though the sample mod's whole point is hooks | Added "Script hooks" section |
| 3 | **Doc gap** | Sample scripts use API not in `scripting.md`: `api.round`, `api.floating_text`, `ent.reward_diamonds`, `killer.weapon`, `killer.bonus_diamonds_pct` | Added to API surface + documented `killer.*` semantics |
| 4 | **Doc gap** | `projectile.kind` enum (`chain`/`hitscan`/`bolt`/...) appears in the example but is never defined | Documented kinds in `weapons.md` |
| 5 | Minor | `critter.yaml` omitted `scripting: lua` that `docs/README.md` quick-start shows | Added |

## Open notes (no change made — design decisions)

- **Vulture economy ramp:** `bonus_diamonds_pct: 20` per rank, applied to
  critter kills worth 1 diamond → rank 1 pays `round(1×0.2)=0`. The economy
  tower visibly pays nothing until rank 3 (60% → 1). Consider `ceil` semantics,
  a base bonus, or higher-reward races in the demo map — or keep it as a
  deliberate "invest to see returns" curve. **Decision (2026-08-24): kept as-is and
  documented** — the demo intends an invest-to-see-returns arc; revisit only if playtest
  feedback flags rank-1 dead-zone as confusing.
- **Sample map: RESOLVED (2026-08-24).** `mods/maps/sector_1_1.yaml` exists —
  references the 3 sample weapons (arc_weaver, lance, vulture), 3 rounds,
  `path` polyline + `starting_towers` (draft schema), validated by the engine
  loader (`content.rs`). Completes the "every mod is a folder" picture.
- **`api.round` rounding mode** should be pinned in the engine spec (round-half-
  away-from-zero assumed here).

## Audit method

- YAML parsed with PyYAML (safe_load) — all files parse as mappings.
- Weapon `upgrades:` lists resolved against `upgrades/*.yaml` ids — 0 missing.
- Upgrade `requires:` chains resolved within the tree — 0 broken, acyclic.
- Field-by-field comparison against `races.md`, `weapons.md`, `upgrades.md`,
  `maps.md`, `scripting.md` tables.
- Lua scripts checked against the documented API surface + hook contract.
