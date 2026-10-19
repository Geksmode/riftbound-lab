# Battlefields not modelled (batch battlefields)

All 55 battlefields of `batches/battlefields.txt` are registered in `battlefields.py` (the Brush token
included: it is created by Ivern, Green Father). The 19 that waited for engine hooks use: `cost_aura` (Repeat parts
`on='rep'`, abilities `kind='ability'`, `flex`, `rm`, Power increase `p`), per-turn history (`Game.hist`
`gear_played`, `gear_abs`), `cost_mod` effects, `deflect_reqs`, `loc_veto`, `aura_locs` with an additional cost
(`loc_reqs`), `aura_cant_move`, `grant_abilities`, `death_rep`, the `returned` event, `item.data['paid_e']`,
`grant_repeat` (`next`), `hide_slots`, and `Game.replace_battlefield` / `swap_back` (rule 438).

## Baron Pit (registered 2026-10-19)

- Battlefield token (rule 187.9): "(You can't start the game with a token battlefield.) Units can move here from
  anywhere." Added by Baron Nashor (`Game.add_battlefield`). Battlefields are now a list of any size: locations are
  `"base"` or an index of `g.bfs` (`is_bf(loc)`, `g.bf_locs`, `g.locs()`). "From anywhere": the Standard Move
  (`actions.move_options`) lets a unit at another battlefield move here without [Ganking]. The deck editor refuses
  token battlefields (Baron Pit, Brush) as starting battlefields (`train.validate`, `cards.TOKEN_NAMES`).
- "A different battlefield" (Imposing Challenger) asks which one when there are two others.

## Rules decisions
- **Mystic Vortex**: "cards with [Reaction]" = cards with the keyword (printed or granted), [Hidden] cards played
  from facedown and [Quick-Draw] gear (rule 813: Quick-Draw gives Reaction).
- **Ornn's Forge / Piltovan Forge**: "the first ... each turn" counts every friendly non-token gear played (by any
  means) / every activated ability of a friendly gear played this turn ([Add] abilities used while paying included).
- **Rockfall Path**: unit tokens can't be played here either (rule 358.3.a: the play instruction is skipped).
- **Dragon Roost**: playing a Dragon there is offered as a location with its additional cost ([A][A]), even when
  the player doesn't control the battlefield.
- **Altar of Blood**: "during combat" = while the combat at this battlefield is in progress (the deaths of its
  combat damage included).
- **The Academy**: "its base cost" = the spell's printed Energy and Power cost.
