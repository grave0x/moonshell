# P1 — Core Loop Prototype: Plan

**Status:** DRAFT (2026-08-24) · **Owner:** next engineering session · **Spec ref:** SPEC.md §3, §4, §6
**Inputs:** P0 spike verdict (docs/PERF_SPIKE.md) — architecture locked: ECS sim, flow-field
routing, instanced renderer, fixed 60 Hz tick.

## Goal

A playable vertical slice of the faithful clone loop, driven entirely by the sample mod
(YAML + Lua) — the v1-gate rehearsal:

> hub (upgrade tree, 3 currencies) → build phase (place towers, caps, drag-to-aim) →
> battle (continuous orc spawn from cave, HP 80, leak/kill/economy) → win/lose/surrender →
> layout persists → next map.

**Acceptance:** the sample mod's critter race + 3 towers (arc_weaver chain / lance
single-target / vulture economy) with their upgrade trees runs end-to-end as pure data —
zero engine code changes.

## Architecture to build on (from the spike, locked)

| Piece | Decision | Evidence |
|---|---|---|
| Sim | 100k `Orc` entities in ECS, fixed tick | 2.3 ms/frame; 60 Hz held cleanly |
| Routing | Flow field per map (Dijkstra), 1 lookup/orc/frame | build <1.3 ms; 100k @ 650 fps; A* infeasible |
| Render | Instanced draw call + view culling | 125–135 fps @ 100k; 2.2× culling win |
| Small counts | Per-entity sprites OK (< ~10k: projectiles, UI) | naive path 36–55 fps @ 100k — not for hordes |

## Proposed repo layout (proposal — confirm with spike owner before scaffolding)

```
crates/
  moonshell-core/     # ECS sim + content model (races/weapons/maps/upgrades structs)
  moonshell-render/   # instanced renderer (lifted from spike/src/main.rs), UI, sprites
  moonshell-app/      # binary: hub → build → battle, save, console REPL stub
mods/                 # sample-mod moved here (content root), loaded at runtime
spike/                # stays as the standalone perf harness (not part of the game)
```

Simpler alternative: one crate with modules (`sim/`, `render/`, `ui/`) until the split
hurts. Recommend the single-crate start for P1 velocity.

## Milestones

### M1 — Engine skeleton + content load (foundation) ✅ done (commit c752b8e)
- [x] Crate skeleton + app loop (winit window, instanced renderer from spike)
- [x] Content model: serde structs for race/weapon/map/upgrade; load sample mod YAML from `mods/`
- [x] Validate the whole sample mod loads + cross-references resolve (errors carry file paths)
- [x] Sample map `sector_1_1` (draft schema additions: `path` polyline, `starting_towers`)
- [x] Battle sim seed: spawner per wave schedule, flow-field follow, leaks drain HP, win/lose
- **Exit:** met — towers render as cyan instanced quads, orcs trace the serpentine route and leak, no panic.

### M2 — Battle sim (the heart)
- [ ] Map: routes (waypoint polyline for the flow-field sources), rounds, orc budgets (docs/format/maps.md)
- [ ] Orc spawner: continuous stream per round/budget; flow-field follow; leak at base → HP 80 drains
- [ ] Towers: place (cap per type enforced), drag-to-aim for manual weapons, targeting modes (first/last/strong)
- [ ] Projectiles: speed/splash/pierce/bounce/chain/burn per weapon schema
- [ ] Damage & death: **hp <= 0 kills**; `on_death` hooks receive pre-death HP (split-on-death works);
      kills → 💎 diamonds; silver/explosive meta nodes
- [ ] Economy: orbs/cubes from upgrade-tree node purchases (in-play source pending SPEC §9-1 confirmation)
- [ ] Win (all orcs dead → full-clear bonus) / lose (HP 0, loot kept, layout persists) / surrender (bank, exit)
- **Exit:** a full battle on a sample map runs to a win and a lose with correct currencies.

### M3 — Hub + build UI
- [ ] HUB screen: upgrade tree from YAML (unlock nodes = 🟠 orbs, upgrade nodes = 🟪 cubes), tooltips
- [ ] Build phase: placement grid, caps, sell/replace rules, drag-aim preview
- [ ] Battle HUD: top bar (3 currencies, HP 80, orc count, kills, leaks), pace controls (⏸ ▸ ⏩), SURRENDER
- [ ] Deadpan AI voice lines as text callouts (audio later)
- **Exit:** the full loop hub → build → battle → hub works with the sample mod.

### M4 — Persistence + polish pass
- [ ] Save: explicit slots + auto-save; atomic writes (temp + rename); layout persists across runs
- [ ] Settings stub (resolution, fps cap, vsync, keybinds) — minimal
- [ ] Logging + mod-error surfacing (console with file:line)
- **Exit:** kill the app mid-battle, relaunch, layout + currencies intact.

## Open decisions needed before/early P1 (user sign-off)

1. **Death threshold:** docs recommendation stands — hp <= 0, `on_death` gets pre-death HP
   (so split-on-death works). Unblocks M2 damage system.
2. **Orb/cube exact sources** (SPEC §9-1): observed diamonds = kills; orbs/cubes pending
   in-play confirmation of the original. Suggest until confirmed: orbs per map-clear +
   wave milestones, cubes per run score — flag as placeholder.
3. **Repo layout** (above) — confirm with current spike owner before scaffolding.

## Out of scope for P1 (P2+)

Hot-reload, console REPL, Lua scripting sandbox, mod manager, records, achievements,
settings breadth, Android/WASM. P1 proves the loop; P2 makes it a platform.

## Sizing (rough)

M1: 1 session · M2: 2–3 sessions · M3: 2 sessions · M4: 1 session.
