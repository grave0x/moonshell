-- sample-mod/scripts/critter.lua
-- Behavior override: critters split into two smaller critters on death.
-- Proves races can behave DIFFERENTLY, not just different stats.

function on_death(ent, ctx)
  if ent.hp <= 1 then return end
  for i = 1, 2 do
    local child = api.spawn("critter", ent.pos())
    child.hp = math.floor(ent.hp / 2)
    child.speed = ent.speed * 1.1
  end
end
