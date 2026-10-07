---
name: riftbound-deckbuilder
description: Training table v11 deck builder (all 938 cards, only IMPL cards playable) and the 2026-10-07 project to model every Riftbound card
metadata:
  type: project
---
2026-10-07: the user asked for a menu to build any decklist and play it against the AI, then « modélise toutes les cartes de riftbound ». Decision card: cards not yet modelled are greyed and blocked from play (option « Bloquer »), never played as stats-only.

- Page v11 (artifact HTmfbgtzcPP7jN7UsCfcHC):
  - "Decks" button opens a full-screen builder (#deckView):
    - filters: search, type, set, domain, "domaines de ma légende", "jouables seulement";
    - deck panel with live rule check (train.validate, rule 103);
    - import/export as text lines « 3 Name »;
    - decks saved in db collection "decks" plus localStorage "rbt-decks".
  - The setup form picks your deck and the AI's deck (references, tournament lists, your own; mirrors allowed).
  - Images: 938 thumbnails in 16 atlas sheets (atlas/aN.jpg, 10x6 cells of 200x279; atlas/index.json maps name to [sheet, cell]; helper art(n)). Battlefields are rotated to portrait. Built by riftbound/train/get_thumbs.py.
- train.py:
  - `new(seed, bf, first, level, mine, opp)`: a deck is a PRESETS key, a decks.json key, or JSON {legend, champion, main, runes, battlefields}.
  - `catalog()`: m=1 marks a card in cards.IMPL.
  - `check()` / `validate()`: lines starting with ⚠ are warnings only (e.g. a banned card).
  - Plans: Akali uses Gorica, LeBlanc uses Hook tempo, any other legend uses the generic Plan().
  - Player names come from the legends (replay.NAMES is overwritten).
  - REC stores mine/opp so that train_games can replay.
- Modelling everything (2026-10-07):
  - Foundation done: keywords (Vision/Predict/Hunt/Level/Weaponmaster/Burn/Add/Legion/Equip bonus=…), testkit.py, test_all.py, fuzz_cards.py, 42 keyword-only cards auto-registered (IMPL 133). Env RB_CARDSETS=a,b loads only those modules.
  - 8 batch workers (fury, calm, mind, body, chaos, order incl. colorless, legends, battlefields; lists in cardsets/batches/*.txt). Each writes cardsets/<b>.py, test_<b>.py, NEEDS_<b>.md; never registers an approximated card. Results: 768/927 playable (35/47 legends, 35/55 BF), 812 tests; page v12 published 2026-10-07 (engine snapshot train/engine_versions/v12). Worker "integration" then adds the NEEDS hooks (~159 cards), an ASK_TEXT registry for French ask titles, and full option lists for the human. Test harnesses: train/t_rand.py (random legal decks through train.py), train/dk2.py (browser; CSP blocks wait_for_function, so poll instead).
  - First, worker "mecaniques" adds generic keywords and the engine/cardsets/ package (one module per batch with its tests, explicit list in cardsets/__init__.py, GUIDE.md, NEEDS.md, test_all.py).
  - Then batches run in parallel workers.
  - build.sh copies the cardsets modules listed in __init__.py into the page.
Related: [[riftbound-training-table]], [[riftbound-engine]], [[riftbound-train-games]].
