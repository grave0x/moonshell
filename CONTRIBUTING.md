# Contributing

Moonshell is open source (GPL-3.0) and pre-P0: the mod-format documentation is the product
right now, and the engine hasn't been written yet. That shapes how you can help.

## Right now (pre-P0) — shape the contract

The highest-leverage contribution is **feedback on the mod format**:

1. Read the [mod-format docs](docs/) — races, weapons, maps, upgrades, scripting.
2. Try to build something with the [sample mod](docs/sample-mod/) as a skeleton.
3. Open an issue: what's missing, ambiguous, or wrong? Every schema field is negotiable
   until the engine locks it.

Ideas that are especially useful:
- A second sample mod in a different genre (the format shouldn't be orc-TD-shaped only).
- Corner cases: multi-round maps, tower caps, upgrade-tree topologies, permission needs.
- Naming/API bikesheds — better to fight now than after the engine lands.

## Later (P1+) — the engine

- **Performance spike (P0):** Bevy 0.19 ECS, 100k entities @ 60fps. Measurements welcome.
- **Core loop (P1):** hub → build → battle, 3 currencies, HP 80, continuous spawn.
- **Modding platform (P2):** yaml-edit object graph, file watcher, console REPL, Lua sandbox.

## Process

- Issues > PRs for format changes (the contract is the product).
- PRs need a clear description; docs PRs must keep the lossless round-trip guarantees in mind.
- By opening a PR you agree your contribution is licensed GPL-3.0 like the rest of the repo.
- Be kind. Deadpan AI narration is encouraged; gatekeeping is not.

## Questions

Open an issue with the `question` label, or comment on the pinned interest-check issue.
