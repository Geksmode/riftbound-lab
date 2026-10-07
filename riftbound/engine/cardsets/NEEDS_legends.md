# Batch legends: cards not modelled

All 47 legends of `batches/legends.txt` are registered in `legends.py` (tests: `test_legends.py`), plus the Brush
battlefield token created by Ivern, Green Father. The 12 legends that waited for engine hooks use: the player who
empowers (`empowered` event `by`), battlefield replacement (`Game.replace_battlefield` / `swap_back`, rule 438),
energy paid for a spell (`item.data["paid_e"]`), `units_enter_ready`, the `activated` event (printed Energy cost),
`gold_extra`, [Add] with a resource cost and restricted energy (`add` with `conv="p2e"`, `only=`), `death_rep` on a
legend, the `rune_recycle` and `banish` events, and `hide_cost`.

## Rules decisions
- **Ivern, Green Father / Brush**: the Brush token keeps every status of the battlefield it replaces (same
  `Battlefield`, rule 438.1); the replaced battlefield waits in Banishment as "replaced" (kept in `b.replaced`,
  438.5.a) and comes back by "When you score here, you may replace this with the battlefield it replaced"
  (conquer or hold there, 438.7). Brush can't be put in a deck (a token).
- **Jhin, Virtuoso**: "spent 4 energy or more" is the Energy actually paid for that spell (discounts applied); the
  spells "banished with me" are those Jhin banished that are still in Banishment.
- **Nasus, Curator of the Sands**: "Energy cost 7 or more" is the printed Energy cost (rule 206); an activated
  ability counts as it resolves (rule 377.2.a), like the other "when you play an activated ability" triggers.
- **Renata Glasc, Chem-Baroness / Vex / LeBlanc**: "you or an ally" is "you" in 1v1.
- **Renekton, Butcher of the Sands**: its [Add] is used while paying for a unit or a unit's activated ability, when
  the payment needs it (like every [Add] in this engine); the unused energy floats, still restricted.
- **Sett, The Boss**: "recall it exhausted" doesn't heal (rule 455): a unit saved from lethal damage dies again at
  the next cleanup unless something else saves it.
- **Sivir, Battle Mistress**: the units killed by one action ready her once (like Leona, Radiant Dawn).
- **Teemo, Swift Scout**: the Teemo unit is chosen as the ability is played and must still be there (same object)
  when it resolves.
