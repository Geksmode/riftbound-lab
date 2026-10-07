---
name: riftbound-manager
description: Agent manager loop: routine every 6h runs matchup sim sessions from riftbound/manager/PROTOCOLE.md and writes retex
metadata:
  type: project
---
2026-10-03: the user asked for a "manager agent" that writes a retex at the end of each session and, once tokens renew, runs a new matchup session building on what it learned. Claude can't see token renewal, so a routine (trig_01PWy4DH7kxfmh32NvyUbZrU, cron every 6h, fires into the thread "Agent manager de sessions matchups") stands in for it.

Files in /mnt/project-files/riftbound/manager/: PROTOCOLE.md (steps each session follows), learnings.md (current knowledge: acquis/pistes/réfuté/questions + session history, rewritten each session), agenda.json (next jobs), run_session.py (JSON jobs, PlanAgent from engine/plans.py, one shared fresh seed block per session for paired comparisons, "fresh": true for confirmations), state.json (next_seed, starts 200000; never reuse lower seeds), sessions/ retex, results/. Vault note: vault/Agent manager.md.

**How to apply:** never edit engine files or plans.py from the manager; follow [[retex-method]]; to pause, disable the routine. Related: [[akali-gorica-heron]], [[riftbound-vault]].

2026-10-03: user wants each sim session's notable games in the replay viewer. Contract proposed to the Replays thread: manager writes results/session_NNN_picks.json via pick_games.py (id sNNN-k, seed, title, note, full job spec, plans snapshot); the Replays thread owns the recording tool (expected engine/add_replays.py) and the viewer/index — manager never edits them.
From s003 the runner freezes the whole engine (results/engine_session_NNN, engine_md5 in results; parents[1] paths rewritten to riftbound/) because the Replays thread is changing ai.py/plans.py (user asked for tempo/resource-aware AI, 2026-10-03). Never aggregate figures across engine versions.

2026-10-06: engine rules changed (Replays thread, user request) — s004-s015 = old version, s016+ = new; never pool across. When the live engine differs from a session's frozen engine, record its replays by running add_replays.py from a scratch copy of results/engine_session_NNN with a rb/replays symlink (PROTOCOLE 5b); otherwise games replay differently (ATTENTION warnings). User was asked (decision card, 2026-10-06 12h) whether to keep the 6h cadence; no answer yet — keep 6h until they say.

2026-10-07: engine refactored (generic keywords + engine/cardsets/ modules imported by cards.py). run_session.py now freezes cardsets/ too. Before pooling across an engine change, run manager/eqtest.py-style check (replay previous session's first 2 jobs on 200 seeds with the new frozen engine); s018 matched s017 400/400 twice → pooled. A routine fire that lands while a session is still wrapping up is absorbed (no extra session).