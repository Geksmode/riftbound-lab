---
name: riftbound-gameplans
description: Player gameplans (Gorica Metafy Akali, LeBlanc Hook tempo) coded in engine/plans.py; results vs LeBlanc with intervals
metadata:
  type: project
  modified: 2026-10-03T10:55:36.669Z
---
2026-10-03: the user said the sim's play patterns don't match real players. Gameplans were taken from YouTube transcripts (fetched via Remote Control on the user's PC), Gorica's Metafy guide (the user put it in their PC vault) and written LeBlanc guides (riftbound.gg via its WP API; Hextech, RiftStorm and Cardsrealm only through WebSearch summaries because they are blocked). Summary: riftbound/gameplans/gameplans.md plus notes_*_videos.md.

The code is engine/plans.py (Plan: mulligan, battlefield, prior, shape, choose; PlanAgent = SearchAgent + plan) and engine/exp_plans.py (results in results_plans.json; per-seed results allow paired comparisons; env ONLY=indices).

Plans:
- Akali "Gorica" = engine plan.
- Akali "Gorica agressif" = Metafy vs-LeBlanc update.
- LeBlanc plans: "Deathknell" (slow, don't race); "Hook tempo" (the IQ#5 list is the western Baited Hook tempo build); "Hook tempo Windswept" (always presents Windswept).

Results, G2 vs IQ#5, 400 games per config:
- Hook tempo is LeBlanc's best plan: −6 ± 3 for Akali vs Deathknell.
- Akali engine plan vs Hook tempo: 51.5% (seeds 70000+) and 54.2% (80000+). That is +8 to +10 points over Akali with no plan (paired).
- CORRECTION: absolute levels from one 400-game block are only ±5, not ±2.5. Block-to-block spread is above binomial (manager session_002: engine vs Hook tempo Windswept 53.3/40.0/47.3%, mean 46.8% over 1200; cause unknown). Only trust paired differences within a block.
- Aggro plan: 47-51%. It is never better than the engine plan, and is clearly worse when LeBlanc always presents Windswept (45.2%; engine minus aggro = +8.0 ± 3.3).
- An always-Windswept LeBlanc costs the engine plan nothing (−1 ± 2.2), because the engine plan answers with Sigil of the Storm.
- The sim is still above real data (~30-35%; Gorica says 45/55).

Session "Agent manager" (riftbound/manager/) also runs experiments on plans.py. This thread owns plans.py.
Related: [[akali-gorica-heron]], [[riftbound-engine]], [[retex-method]].
