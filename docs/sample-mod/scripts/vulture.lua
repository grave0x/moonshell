-- sample-mod/scripts/vulture.lua
-- Economy tower: salvage bonus on kills near the drone.
-- Proves scripted towers can bend the economy, not just the combat math.

function on_kill(ent, killer, ctx)
  if killer and killer.weapon == "vulture" then
    local bonus = api.round(ent.reward_diamonds * killer.bonus_diamonds_pct)
    api.grant_diamonds(bonus)
    api.floating_text(killer.pos(), "+" .. bonus, "#7CFC00")
  end
end
