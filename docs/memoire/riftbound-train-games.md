---
name: riftbound-train-games
description: Training table v10 records every user game to artifact db "games"; train_games.py replays them to learn the user's play vs LeBlanc
metadata:
  type: project
---
User request 2026-10-06: « garde surtout les journaux et apprend sur ce que je fais contre leblanc, tu affineras l'IA de LeBlanc avec ce que j'ai fait pour gagner ».

- Page v10 (artifact HTmfbgtzcPP7jN7UsCfcHC, capability db) saves each game to db collection "games", doc id = `<time36>-<seed>`. Fields: seed, bf, first, level, moves (["act",i,label,turn] / ["ans",x,label,turn] / ["hint",5] / ["undo"]), log (journal), result, pts, turn, ver.
- Fetch: ArtifactData list collection "games" with out_dir /mnt/project-files/riftbound/train/games.
- Replay: `cd engine && python3 train_games.py <json...> [--coach]` writes riftbound/train/analyses/<id>.md and choices.jsonl. It checks fidelity (the result matches; the log matches when there was no undo).
- Determinism fix: engine iterated sets of tuples whose order differs on 32-bit Pyodide. Fixed with sorted() in actions.py (groups) and cards.py (Bellows Breath). This changes option order for old sim seeds (statistically neutral).
- First real user game (seed 453115, unfinished, 0-2 at turn 4) replayed faithfully.
- Plan: once there are several games, wins especially, model the user's choices and use them as LeBlanc's opponent model (opp_plan) in place of the Gorica plan. Measure on fresh seeds per [[retex-method]].
Related: [[riftbound-training-table]], [[riftbound-tempo-ai]].
