---
name: replays-after-every-sim
description: User rule (2026-10-03) - after EVERY simulation, add its most important games to the Riftbound replay viewer and republish
metadata:
  type: feedback
---
The user asked on 2026-10-03: « à chaque fois que tu fais une simulation tu mets les games les plus importantes dedans » (in the replay viewer, artifact ThZartFw9uTqb6jcjvpgFV).

**Why:** they learn the matchup by watching the games behind each finding, not just the percentages.

**How to apply:** after any sim run, including the manager sessions:
1. Run engine/add_replays.py. Use `flips` to find seeds where config A wins and B loses, i.e. the same deal where only the plan changes; these are the most instructive.
2. Use `digest` to read the games.
3. Write a spec with one `group` per simulation and French title and note, then run `add`.
4. Republish from a scratch copy of riftbound/replays/, sending only the changed files plus games/index.json, with `url`. Read the published index.json first if this conversation has not read it.

Pick 2-5 games per simulation:
- a same-deal pair or triple showing the headline effect;
- the typical loss of the recommended option.
Notes must describe what actually happens in the log.

Manager sessions: `add_replays.py picks manager/results/session_NNN_picks.json` (manager writes picks; replays sessions 1-2 added in v5, outcomes verified). The manager may lack the Artifact tool, so the Replays thread republishes on its message.
curate_replays.py now merges into index.json instead of overwriting it. Related: [[riftbound-replays]], [[riftbound-gameplans]], [[riftbound-manager]].
