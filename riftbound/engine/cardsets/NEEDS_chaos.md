# Batch chaos: cards left out of IMPL

121 of the 122 cards of `batches/chaos.txt` are registered in `cardsets/chaos.py` (tests in `test_chaos.py`). The 17
cards that needed engine hooks (Miss Fortune, Buccaneer, Ocean Drake, Sai Scout, Sneaky Deckhand, Irelia, Graceful,
Stargazer, Vex, Cheerless, Ezreal, Prodigy, Syndra, Transcendent, Kennen, Storm of Shuriken, Mask Mother, Scrapheap,
Treasure Trove, Maduli the Gatekeeper, Vex, Apathetic, Sivir, Mercenary, Nocturne, Horrifying) use the hooks
described in `GUIDE.md`.

## Still not registered

- **Baron Nashor** — "As you play me, add the Baron Pit battlefield token to the board if it's not there already. If
  you do, I enter there. (It has "Units can move here from anywhere.") I can't be chosen by enemy spells and
  abilities. Other friendly units have +2 might." The engine has exactly two battlefields (`g.bfs`, locations 0/1 in
  every rule, the AI, the replays and the interface); a third battlefield token (rule 187.9) needs a variable number
  of battlefields everywhere. Baron Pit (battlefield token) is blocked for the same reason.

## Rulings of the registered cards

- **Conscription / Possession** change the controller of the unit (`o.ctrl`); Equipment attached to it keeps its
  controller (rule 718.5.f: a control change of a unit doesn't change the control of its attached Equipment).
- **Open battlefield** (Miss Fortune, Buccaneer and the "play me to an open battlefield" units) = no units there and
  no controller (`actions.open_bf`, rule 355.2.b).
- **Ezreal, Prodigy** — each optional additional cost paid (Accelerate, each Repeat cost — Curtain Call's three are
  separate —, "you may pay ... as an additional cost") costs 1 energy or 1 rune of any type less (the player's
  choice: energy first when both can be paid); the Deflect of a repetition's target is not an optional cost.
- **Syndra, Transcendent** — the granted Repeat is a separate instance (rule 820.1.c.2): its repetition makes its own
  choices; "in a showdown" = at the battlefield of the current showdown or combat.
- **Kennen, Storm of Shuriken** — the granted [Flow] costs the spell's printed Energy and Power (of its domains).
- **Mask Mother**, **Scrapheap** — "When you discard me" triggers wherever the card went (`Impl.on_discard`);
  Scrapheap's "killed" is its own leave event from a kill (not a Deathknell: Karthus doesn't double it).
- **Treasure Trove** — "When this leaves the board" triggers for every way it leaves (`Impl.on_leave`).
- **Nocturne, Horrifying** — the play trigger (from `Game.look`: Predict, Vision, "look at the top N"...) resolves
  only if he is still in the Main Deck and was not drawn since; he is played for [A] instead of his cost
  (alternative cost) to a location where he could be played.
- A [Repeat] cost can be paid when a spell is played from the trash with [Flow] (the Repeat is an optional
  additional cost, the Flow cost replaces the base cost).
