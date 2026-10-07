---
name: akali-vs-yi-coaching
description: 2026-10-04 user lost 0-2 BO3 vs Yi Bladesman and Azir, wants to improve; Yi fiche, game log, decision puzzles artifact
metadata:
  type: project
  modified: 2026-10-04T14:09:02.426Z
---
2026-10-04: user went 0-2 in BO3 vs Master Yi, Wuju Bladesman and Azir with Gorica's Akali Heron ([[akali-gorica-heron]]), lost confidence, asked "aide moi à devenir meilleur" and "fais-moi des puzzles personnalisés".

Their diagnosis vs Yi: legend (+2 to a lone defender) makes taking battlefields too hard, so the Heron plan stalls; Heron in base gets hit by Charm/Rampage.

Built:
- Yi list: decks/raw/yi-bladesman_ekpilot_rq-singapore_3rd.txt (WebSearch summary, validated 40/12/3).
- vault/Akali vs Yi Bladesman.md: card-text sheet with trick timings (Charm/Rampage/Onslaught = his turn only; Punch First 2 runes = Defy can't counter; sideboard Decree of Focus +4 vs Fury). Rule 355.2: units can be played onto a battlefield you control, so Heron is placed on a held battlefield, not walked in.
- vault/Carnet de parties.md: game-log template.
- Puzzles artifact https://claude.ai/artifact/Ds4TSVoR6tcNRbQKBKYpcP: v1 was a QCM; user said they want a real BOARD where they move units and play cards themselves (2026-10-04), so v2 = interactive board, 6 Yi puzzles. Source riftbound/puzzles/ (engine.js mini-engine + minimax Yi AI blind to user's hand, puzzles.js, test.js solver, build.py). Every puzzle verified by solver. v3 (user: 'aucune restriction'): engine allows every legal action (keep priority, any target per card text, Hidden, Accelerate, Flow, Heron discount, Akali legend Empower/retreat, player-chosen damage order, undo). Several solutions accepted; only the goal counts. v4 (user: "forme comme tcg arena et riftatlas"): table.html + build_table.py, playmat look with real card images, drag and drop, hover preview.
Nothing measured: Yi and Azir not modelled in engine. Azir debrief still pending from user.
**How to apply:** new puzzles should come from the user's real situations; keep puzzles to what card text and rules settle.
