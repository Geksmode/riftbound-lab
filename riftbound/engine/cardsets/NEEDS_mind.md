# Batch mind: cards not modelled

All 120 cards of `batches/mind.txt` are registered in `mind.py` (tested in `test_mind.py`). The 20 that waited for
engine hooks use: `cost_aura` (with `min_e`, rule 356.4.e), play permissions (`g.effects` kind `play_perm`),
granted Repeat (`grant_repeat` with `next`), `alt_costs`, `Game.hist` (`spell_e`, `died`, draw event `n`),
`hide_free`, `no_combat_damage`, `point_rep`, `scoring_extra`, `token_rep`, the `activated` event, item
`played_by` / `gain_control_item` / `remake_choices`, `item.data['after'] = 'recycle'`, `no_counter`,
`minus_extra`, `grant_abilities` / `grant_add`, [Add] converters (`conv='p2e'` with `var`, `conv='kill'`) and
`equip_tags`.

## Rules questions decided in mind.py

- **Tokens played by effects** (`_play_token`): they enter the board, get their keyword play triggers ([Vision])
  and emit `played` with `n=0` (tokens can be played, rule 350.2, but are not cards: nothing is added to
  `g.played`). So Pit Crew sees a Gold token, Forecaster gives [Vision] to a Mech token. Tokens of other modules
  (`make_token`) don't do this yet (NEEDS.md, "Tokens played by effects").
- **Unit tokens without a printed location** ("Play a 3 might Mech unit token", Sprite Burst, Gutter Palace's
  Bird): the player chooses base or a battlefield they control (rule 355.2); from Hidden, at that battlefield
  (rule 811.1.d.3).
- **Ava Achiever**: a spell played "here" chooses its targets as if played from Hidden at that battlefield
  (rule 811.1.d.2); a gear goes to base; base costs are ignored, additional costs (Deflect) are still paid.
- **Teemo, Strategist**: "Deal 1 to an enemy unit here for each card with [Hidden]" = one chosen enemy unit takes
  1 per revealed Hidden card (chosen when the trigger is finalized; no enemy unit here: no trigger, rule 355.8).
- **Sumpworks Map**: "scores" = a scoring Conquer/Hold (rule 469) or a point scored by a card effect; a point
  gained from the opponent's Burn Out is not a score.
- **"Score 1 point"** (Bottled Constellation, Renata Glasc, Swain): plain point gain, the Final Point rule (471.1.b)
  only concerns Conquer.
- **"When you play a spell"** (Lux, Ravenbloom Student, Chemtech Cask, Viktor): the engine's `played` event of a
  spell is emitted as it resolves (as in rule 359.3.e.10's example); a countered spell doesn't trigger them.
- **Heimerdinger, Inventor**: "all exhaust abilities" are the activated abilities with [E] in their cost and the
  [Add] abilities with [E] (Gold's "Kill this, [E]: [Add] [A]" when a friendly Gold is on the board) of the
  friendly legend, units and unattached gear, granted ones included; "me"/"this" in them is Heimerdinger (rule 376).
- **Jayce, Man of Progress**: the permission is used by the first gear played from hand with it (the player
  chooses to use it); its Power cost and additional costs are still paid.
- **Temporal Portal**: "[Repeat] equal to its cost" = the spell's printed Energy and Power cost.
- **Kai'Sa, Evolutionary**: the spell's Power cost and optional additional costs are paid; it is recycled wherever
  it leaves the chain (resolved or countered).
- **Otterpus**: "during their first or second turn" = during that player's own turn, while it is one of their
  first two turns.
- **Blue Sentinel**: the delayed [Add] happens at the start of the controller's next Main Phase (the rune stays in
  the pool until the pool empties).
- **Rebuttal**: the 1 rune is paid on resolution; if it isn't, the spell is countered.

