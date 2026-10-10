---
name: riftbound-replays
description: Graphical replay viewer of simulated games (artifact ThZartFw9uTqb6jcjvpgFV), recorder engine/replay.py, curated 8 Gorica G2 vs LeBlanc games
metadata:
  type: project
  modified: 2026-10-03T08:52:38.932Z
---
2026-10-03: the user wanted to watch the simulated games graphically to learn. Built:
- engine/replay.py: additive recorder (RecGame/RecAgent subclass Game/SearchAgent, same RNG draws, resets Obj/Item id counters so a seed replays identically). Frames = French log line + full state + AI option scores + Akali-side evaluation. 91 tests still pass.
- engine/batch_replays.py (lot + summaries), engine/curate_replays.py (PICKS = curated list with notes).
- riftbound/replays/: viewer.html, games/*.json, img/ (card thumbnails from cards_all_printings image_url). Published artifact: https://claude.ai/artifact/ThZartFw9uTqb6jcjvpgFV (republish from a scratch copy of the folder with `files`).

Batch of 240 fresh-seed games (60000-60239) G2 vs IQ#5 LeBlanc: Akali 47.9%, 56% going first vs 41% second (consistent with [[akali-gorica-heron]] initiative effect, single batch, not a new headline figure).
2026-10-03 v4: viewer groups games by simulation (index fields group, plans); engine/replay.py record_plan replays exp_plans games exactly; group « Simulation des plans de jeu » = seed 80002 under three Akali plans + seed 80000 typical engine-plan loss.
2026-10-05 v19: phone layout (viewer.html @media max-width 820px or short landscape): board recomposed in one column, small cards, playback bar fixed at bottom, card text as bottom sheet, swipe on board = step. Desktop (≥1000px fit-to-screen) unchanged. Test with Playwright: pip install playwright, executable_path /opt/pw-browsers/chromium-1194/chrome-linux/chrome; local viewer.html now has a viewport meta.
Known AI quirk visible in replays: a side that is far behind often just ends its turn for several turns.
Related: [[riftbound-engine]], [[retex-method]].
