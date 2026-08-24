<p align="center">
  <img src="branding/nanosproject-icon-48px.png" alt="Moonshell" width="112"/>
</p>

# Moonshell

**The open-source, horde-scale incremental tower defense where modding is editing data files.**

An AI defends its lunar defensive base from an overwhelming orc swarm. Races, weapons,
maps, upgrade trees — the whole game is data: YAML you can open in any editor, Lua you can
write in any editor. No SDK. No compilation. No loader hacks.

- **Modding = data:** drop a folder in `mods/`, watch it hot-reload live. Or drive the same
  content tree from the in-game console (`~`) with tab completion and `save()`.
- **Stack:** Rust + Bevy 0.19 · lossless round-trip YAML (yaml-edit) · sandboxed Lua 5.4 (mlua)
- **License:** GPL-3.0 · free on GitHub/itch, paid Steam + Play later (Mindustry model)
- **Goal:** out-perform the original — 100k entities @ 60fps ([spike passed](docs/PERF_SPIKE.md): 125–135 fps @ 100k on mid hardware)

## The docs exist before the engine (deliberately)

The modding contract is the product. The draft schemas are published now so the community
shapes them before a line of game code is written:

| Doc | Covers |
|---|---|
| [races](docs/format/races.md) | Enemy races: stats, behaviors, wave composition |
| [weapons](docs/format/weapons.md) | Towers: firing behavior, projectiles, caps, upgrades |
| [maps](docs/format/maps.md) | Lunar sectors: routes, rounds, orc budgets |
| [upgrades](docs/format/upgrades.md) | The tree: unlocks (orbs), upgrades (cubes), stats |
| [scripting](docs/format/scripting.md) | Lua dialect, API surface, permission manifest |

[Try the sample mod](docs/sample-mod/) — a working skeleton: a new race + 3 towers with
upgrade trees. Pure data, zero engine code.

[Perf spike](docs/PERF_SPIKE.md) — the P0 gate, measured: 100k entities move at ~400–650 fps
(ECS + flow-field routing) and render through one instanced draw call at 125–135 fps. The
horde renderer is instanced; per-entity sprites stay for small counts.

## Status & roadmap

**P0 ✅ done.** Mod-format docs published + repo live + 100k perf spike passed
(2026-08-24) → **P1** = core loop prototype → P2 = modding platform → P3 = v1 gate
(demo mods, settings, saves) → v1 free beta on itch, paid Steam/Play launch.

## Signal interest

This repo is an active interest check. If Moonshell sounds like your thing:

- ⭐ **Star the repo** — the single best signal
- 👍 **React to the pinned issue** — tell us what you'd want (player? modder? learner?)
- 🐛 **Open an issue** — the schemas are drafts; the docs are data too
- 📖 **Read [SPEC.md](SPEC.md)** — the full locked design (market research included)

## Links

- [Project specification](SPEC.md) · [Mod-format docs](docs/) · [Announcement](docs/ANNOUNCEMENT.md)
