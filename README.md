# Moonshell

Open-source, horde-scale incremental tower defense. An AI defends its lunar
defensive base from an overwhelming orc swarm — and modding is editing data files.

- **Modding:** races, towers, maps, upgrade trees = YAML + Lua. No SDK, no compilation.
- **Stack:** Rust + Bevy · yaml-edit (lossless round-trip YAML) · Lua 5.4 (contained env + permissions)
- **License:** GPL-3.0 · Free on GitHub/itch, paid Steam + Play later.

- [Project Specification](SPEC.md) — the full design
- [Mod-format documentation](docs/) — draft schemas (published before the engine, deliberately)
- [Sample mod](docs/sample-mod/) — a working content skeleton

## Status

Pre-P0. The modding contract is the product; the docs exist before the engine does.
