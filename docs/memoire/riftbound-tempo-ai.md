---
name: riftbound-tempo-ai
description: "Win at all costs" AI change (2026-10-03): ai.TEMPO flag, RB_TEMPO=0 = old AI; measured effects and remaining Akali-at-7 passivity
metadata:
  type: project
---
2026-10-03 the user asked that the AIs "veulent gagner à tout prix" (LeBlanc took Sigil instead of attacking Akali's Mech holding Windswept at 7, seed 200433 turn 13) and to learn tempo principles (guides: riftbound.gg scoring guide via WP API; riftbound.zone/learnriftbound/ultimateguard blocked, WebSearch summaries only; YouTube needs the user's PC).

Change in engine/ai.py (flag `TEMPO`, env RB_TEMPO=0 restores old AI):
- terminal score ±10000 ∓ 25·turn (win early, lose late);
- +40 for controlling a battlefield at victory−1 points;
- SearchAgent.pick: when opponent ≥ victory−2 or all options lose, re-score with 6 samples (RecAgent uses pick too);
- PlanAgent drops negative priors in danger;
- PolicyAgent attacks the opponent's winning-hold battlefield first.
exp_plans labels get suffix " · IA tempo"; add_replays spec/pick key "tempo": false replays old AI.

Paired, seeds 90000+, 400 games (Akali win%): no plan vs Hook 42.0→47.0 (+5.0±2.4); engine plan vs Hook 51.8→52.2 (+0.5±2.3); vs Hook Windswept 51.5→48.5 (−3.0±2.3). Games 14.2→13.4 turns. Remaining flaw: Akali at 7 ends turns without contesting (seed 90003 new AI). Manager informed. Replays group « IA gagner à tout prix » in viewer v9.
Related: [[riftbound-engine]], [[riftbound-gameplans]], [[retex-method]].
