# Scripting (Lua dialect + permissions)

Behaviors the YAML can't express live in scripts. v1 dialect: **Lua 5.4**
(via mlua). Rhai is planned as an optional backend in v1.x — mods declare their
dialect (`scripting: lua | rhai`).

## Environment (containment, honestly labeled)

- Whitelisted globals: math, string, table, plus the **game API** (see below).
- No `io`, `os`, `debug`, `package`, `loadfile` — file/network only via the API
  and only if the permission manifest grants it.
- Enforced memory limit (`set_memory_limit`) and a runaway-loop instruction guard.
- This is containment, not isolation: a determined attacker can escape a Lua
  sandbox. Install mods you trust. Warn-on-install is the product, not a bug.

## Script hooks (the behavior contract)

A race/weapon script overrides behavior through hook functions. Hooks are
optional; missing hooks mean default behavior (walk path, shoot, die).

```lua
function on_death(ent, ctx) end      -- race: death behavior (split, explode, ...)
function on_kill(ent, killer, ctx) end -- weapon: kill credit (economy towers)
-- future: on_hit, on_spawn, on_update
```

`ctx` carries battle context (round, position, battle state). `killer` is the
attacking entity: `killer.weapon` (weapon id), `killer.pos()`, plus any weapon
upgrade effects merged onto it (`killer.bonus_diamonds_pct`, ...).

## Game API surface (draft)

```lua
-- entity
ent.hp, ent.speed, ent.reward_diamonds, ent.pos(), ent.set_pos(x, y)
-- damage
api.damage(ent, amount, {kind="fire"|"shock"|"kinetic"})
api.spawn(race_id, at)
api.slow(ent, factor, seconds)
api.chain(from_ent, targets, {chains=n, falloff=0.8})
api.explode(at, radius, damage)
-- economy
api.grant_diamonds(n)
api.round(n)          -- round half away from zero (math.round semantics)
-- floating feedback
api.floating_text(pos, text, color)  -- world-space combat text
-- wave control
api.round(), api.remaining(), api.spawn_next_round()
-- meta
api.log(msg)   -- surfaces in the console with file:line
```

## Permission manifest

```yaml
# manifest.yaml at the mod root
id: my-first-mod
name: My First Mod
version: 0.1.0
permissions: []            # or: [files.read, network]
```

| Permission | Grants | Plain-language note shown in the mod manager |
|---|---|---|
| (none) | logic only | "Runs sandboxed: game logic only, no file or network access" |
| `files.read` | read inside the mod's own folder | "Can read files inside its own mod folder; cannot touch saves or other mods" |
| `network` | outbound network (future) | "Can make network requests — only install from authors you trust" |

## Console parity

Everything scriptable is also console-able and YAML-able. Three surfaces, one
content tree, zero compilation. That is the whole pitch.
