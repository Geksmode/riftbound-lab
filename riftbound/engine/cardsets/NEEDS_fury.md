# Batch fury: cards left out of IMPL

All 120 cards of `batches/fury.txt` are registered (plus Minotaur Reckoner, which was missing from this list). The
33 cards that waited for engine hooks now use them (see GUIDE.md, "Hooks"):
Bonus Damage (`dmg_bonus`, g.effects `dmg_bonus`, `next_spell`), damage doubling (`double_damage`), death
replacement (`death_rep`), "enter ready" for other units (`units_enter_ready`, g.effects `enter_ready`), play
restrictions and locations (`forbid_play`, `no_play`, `only_locs`, `extra_locs`, `card_kw`), alternative costs from
the trash (`alt_costs`), triggers of cards outside the board (`on_discard`, `zone_event`), kill attribution
(`die` info `by`/`by_kind`), statics bound to a permanent (g.effects `src=(uid, oid)`: `skip_draw`,
`play_from_trash`, `trash_to_banish`), per-turn history (`Game.hist`), cost modifiers consumed by the next spell
(`cost_mod` once + `next_spell`), granted activated abilities (`Obj.abs`), excess damage (`Showdown.excess`),
`ignore_tank`, `scoring_extra`, `fx_swap_scoring`, `reveal_rep`, `aura_cant_move`.

## Rules decisions taken for registered cards
- **Blood Rush**: neither the database text nor the card image gives a duration ("Give a unit [Assault 2]."): by
  rule 801.3.a.3 the keyword lasts while the unit stays on the board.
- **Death from Below**: "you may play this from your trash for 1 rune of any type" is offered once, right after the
  spell resolves and reaches the trash (it is not a lasting permission). **Dancing Grenade** works the same way (the
  damaged unit's controller may play it again from the trash for [A], with +1 Bonus Damage per time this card dealt
  damage this turn).
- **Disintegrate**: "If this kills it, draw 1": the draw happens when the unit dies in the cleanup right after the
  spell (rule 428.5.c).
- **Sun Disc**: "The next unit you play this turn enters ready" is a replacement (rule 369.3) used by the next unit
  that enters the board played by that player, a unit token included.
- **Spinning Axe**: its [Temporary] only applies while it is unattached, as its reminder text says.
- **Blind Fury** (1v1): the single revealed card is banished and played for free; if it can't be played (no legal
  choice) it stays banished (rule 419.3.c). **Void Rush**: the card is played for its cost minus 2 energy; optional
  additional costs (Accelerate, Repeat) are offered (rule 356.1.b.3).
- Tokens "played" without a location (Shadow Clone from Death Mark / Zed): the player picks base or a battlefield
  they control where it may be played.
- **Smite**: a unit banished instead of dying was not killed (no Deathknell, no "when you kill" trigger such as
  Immortal Phoenix's).
- **Unlicensed Armory**: "recall it exhausted" doesn't heal it: a unit saved from lethal damage dies again at the next
  cleanup (the replacement was used).
- **Immortal Phoenix**: "kill a unit with a spell" = the unit died with your spell as the killer (rule 428.5: the
  resolving spell, or the spell whose damage was lethal); it also triggers when the Phoenix itself is the unit killed.
- **Raging Firebrand**: the 5-energy discount is used by the next spell played, even one played by an effect without
  paying its cost.
- **Endless Riches**: its three static abilities apply while it is on the board, for its controller; the burned cards
  go to the trash (they come from the Main Deck).
- **Dominus**: "double a unit's Might" gives +X this turn, X = its Might at resolution (rule 432.1.a).
- **Red Brambleback**: "Your conquer effects for conquering here trigger an additional time" repeats every triggered
  ability of that player queued by the conquer of that battlefield (its own Buff included).
