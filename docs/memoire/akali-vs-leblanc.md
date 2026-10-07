---
name: akali-vs-leblanc
description: User plays Akali (Rogue Assassin); Akali vs LeBlanc matchup data, simulation results and recommended anti-LeBlanc deck
metadata:
  type: project
  modified: 2026-10-02T16:34:25.823Z
---
User (thanh huy) plays Akali, Rogue Assassin (Fury/Calm). Akali vs LeBlanc, Deceiver (Mind/Order Deathknell/Baited Hook) is ~35/65 in real data: Riftools 83-153 (236 matches); Yomi's Place Barcelona RQ LeBlanc ~80% (10 matches). Figures came from search summaries.

2026-10-02 (faithful engine, /mnt/project-files/riftbound/engine, 4480 games): stock Wuhan list = 31.6%; lists that cut Defy and Long Sword for units = 39.4% (+8 pts, the only clear deck-building gain; C3 and C5 are equivalent). Two other significant effects: going first 46.4% vs 32.1% second (14 pts), and LeBlanc's battlefield - Windswept Hillock 30.6% vs Star Spring 48.8% (her best choice vs Akali is Hillock, Ganking lets her retake battlefields). Everything else is inside the noise (+-4 pts at 160 games/config): which battlefield Akali presents, mulligan policy, Not So Fast, equipment removal vs Baited Hook. vs sided LeBlanc: stock 28.1%, C5 35.0%, C3 38.8%.

This CORRECTS the old simplified sim: Targon's Peak is NOT better than Void Gate, the aggressive mulligan is not a lever, and playing around Vi does not cost 9 points (it is neutral to slightly positive).

Recommended list = Wuhan minus 3 Defy and 2 Long Sword, plus 2 Mournful Witness, 1 Akali Silent, 1 Ferrous Forerunner, 1 Lonely Poro (also the sideboard plan for games 2-3).

Docs: decks/matchups/akali-vs-leblanc-moteur-fidele.md (reference), akali-vs-leblanc.md (card-text analysis), akali-vs-leblanc-simulations.md (obsolete). Next step: user logs real games (who started, battlefields presented, opening hand, score) to recalibrate and settle what is still inside the noise. Related: [[riftbound-engine]], [[riftbound-decklists]], [[riftbound-lab-goal]].
