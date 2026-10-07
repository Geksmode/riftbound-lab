# Batch order: cards left out of IMPL

All 101 cards of `batches/order.txt` are registered in `order.py` (tests in `test_order.py`). The 15 cards that
needed engine hooks (Galio, Indefatigable; Sacred Protector; Tactical Retreat; Soraka, Wanderer; Symbol of the
Solari; Renata Glasc, Industrialist; Rally the Troops; Reluctant Leader; Fallen Feline; Mageseeker Investigator;
Hungry Wolf; Ivern, Friend to All; Shady Spectacles; Undertitan; Vanguard Helm) use the hooks described in
`GUIDE.md` ("Engine hooks for effects of other objects").

## Notes on registered cards (rulings and limits)

- **Dragon Form** — "Its base Might becomes 5 this turn" is a `base_might` effect read by `Game.might` (it replaces
  the printed or copied Might; increases and decreases still apply, rule 477.3).
- **Kayle, Justified** — the number of times she is Empowered is kept in `g.effects` and counts only while she has
  the Empowered status; a Disempower removes the status, so the count goes back to 0 (rule 442.1).
- **Sacred Shears** — its Effect Text "[Deathknell] — Draw 1." (card image) is given to the equipped unit
  (`Impl.fx_deathknell`, read by `Game.kill` from the look-back attachments): it triggers even when the gear dies
  in the same kill action; Karthus, Eternal doubles it.
- **Eye of the Herald** — Might Bonus +0 with Effect Text "When I move, play a 1 might Recruit unit token here."
  (card image). B.F. Sword +3, Blade of the Ruined King +4, Shepherd's Heirloom +2, Sacred Shears +1 (card images).
- **Blood Money** — the Gold tokens depend on the controller of the chosen unit when the spell resolves (they are
  played even if the kill was replaced).
- **King's Edict** — 1v1: the only "other player" is the opponent, who chooses (not targeted, rule 355.10.e).
- **Commander Ledros** — offered additional costs: kill nothing, the 1-4 cheapest units, or any single unit (the AI
  option list stays bounded).
- **Fallen Feline** — "name a spell": any spell name of the card database; the AI names first the spells it has
  seen in the opponent's trash or banishment (most frequent first).
- **Renata Glasc, Industrialist** — "Your tokens enter ready" also applies to gear tokens played "exhausted" (Gold):
  it is a replacement of the way they enter (rule 369.3).
- **Soraka, Wanderer** — her replacement saves, in one sequence, every other qualifying unit dying at the same time,
  even when she dies with them (rules 370.4, 373.2); "here" is read when it applies.
- **Shady Spectacles** — the copy is read through the gear (`Obj.cname`/`copy_src`): it ends as soon as the gear is
  detached, leaves the board, or is attached elsewhere (then a new unit is chosen).
- **Hungry Wolf** — "chosen an enemy unit this turn" is `Game.hist['chose_enemy']` (spells, abilities and triggers
  that chose an enemy unit, whatever left the board since).
