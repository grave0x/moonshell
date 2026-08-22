# THREAT ASSESSMENT: DOCUMENTATION RELEASED

**Moonshell — Mod Format Documentation (Draft v0.1)**

Good news, modders: the barrier to entry has been measured, and it is zero.

Moonshell is an open-source, horde-scale incremental tower defense in which an AI
defends its lunar defensive base from an overwhelming orc swarm. The entire game —
races, weapons, maps, upgrade trees, behaviors — is data. Editable data. YAML you
can open in any text editor, and Lua you can write in any text editor.

This documentation is the contract: **if you can write a YAML file, you can ship
content for Moonshell.** No SDK. No compilation. No loader hacks. No permission
from us. The game re-reads your files while it runs, and the in-game console
(`~` / backtick) lets you inspect and edit the live content tree with tab
completion.

**What's inside (draft schemas, pre-implementation):**

- `format/races.md` — enemy races: stats, behavior hooks, wave composition
- `format/weapons.md` — towers: firing behavior, projectiles, caps, upgrade hooks
- `format/maps.md` — lunar sectors: SVG paths, routes, round counts, orc budgets
- `format/upgrades.md` — the tree: weapon unlocks (orbs), upgrades (cubes), stats
- `format/scripting.md` — Lua dialect, API surface, permission manifest
- `sample-mod/` — a working skeleton: a new race + 3 towers with upgrade trees

**Status: DRAFT.** Schemas will harden as the engine lands (P0 → P3). If you see a
field that looks wrong, file an issue — the docs are data too, and data is
moddable.

— Nanos Project, deadpanly yours
