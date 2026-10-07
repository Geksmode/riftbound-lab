# Batch calm: cards left out of IMPL

All 114 cards of `batches/calm.txt` are registered in `calm.py` (tests in `test_calm.py`). The 22 cards that needed
engine hooks (Alpha Wildclaw, Twilight Shroud, Counter Strike, Esteemed Hierophant, Rabadon's Deathcrown, Highlander,
Guardian Angel, Vilemaw, Helm of Suppression, Lilting Lullaby, Mageseeker Warden, Jax, Unmatched, Lee Sin, Ascetic,
Stand United, Lillia, Protector of Dreams, Sandstone Chimera, Tianna Crownguard, Needlessly Large Yordle, Shadow
Watcher, Solari Shrine, Svellsongur, Mystic Reversal) use the hooks described in `GUIDE.md`.

# Notes on registered cards (rulings)

- **Nami, Headstrong** — "the next time you play a unit this turn": tokens are played too (rule 185.2.a, `played`
  with `token=True`), so a token played by an effect can be that next unit.
- **Simian Ancestor** — "When you buff me": the `buff` event names the player who buffs (`by`); only its
  controller's buffs ready it.
- **Ol' Poro**, **Legion Quartermaster** — effects that play a unit (`play_unit_free`) check that it can be played
  (`Impl.as_played` returning no choice: Ol' Poro's first three turns, no friendly gear to return) and pay the chosen
  additional costs.
- **Highlander** — "recall it exhausted instead" does not heal: a unit that was dying of lethal damage dies again at
  the next cleanup (its damage stays until the end of the turn).
- **Guardian Angel** (Might Bonus +1), **Rabadon's Deathcrown** (+3), **Svellsongur** (+0) — Might Bonus and Effect
  Text read on the card images.
- **Mageseeker Warden** — "spells and abilities can't ready enemy units and gear": every ready made by a spell or an
  ability (triggered abilities included) is stopped; the Awaken readying is a game action and is not.
- **Svellsongur** — the copied text is the text of the unit when it is attached (its name, `copy_name`); it is
  appended to the unit (`Game.copied_texts`): triggered abilities, Deathknells, Might modifiers, keywords,
  activated abilities and hooks on other objects count twice.
- **Mystic Reversal** — new choices (rules 750-755) are offered among the spell's legal choices for its new
  controller (`cards.remake_choices`), for the spell and for each repetition; costs of new choices are ignored.
