---
name: leblanc-real-play
description: LeBlanc realism rules (2026-10-03) from the user and two pasted video transcripts; flags RB_REFL / RB_VIDEO; no measurable win-rate effect
metadata:
  type: project
---
User remarks, 2026-10-03: real LeBlanc players never pull the Reflection or the copied unit back to base, and they hide a card even when only a Reflection holds the battlefield. The user pasted two YouTube transcripts: a Long Beach Deathknell/Hook player, and Riftlab's « LeBlanc Deceiver guide ». YouTube is blocked from the cloud and from the user's PC (subtitles 429), so pasted transcripts are the only route. Notes are in gameplans/notes_leblanc_videos.md.

Code, in engine/plans.py, LeBlancDeathknell (inherited by Hook plans):
- Reflections are identified by `u.name == "Reflection"`, because cname becomes the copied card.
- KEEP_REFLECTION (RB_REFL=0 restores the old behaviour):
  - −8 for a voluntary move to base of the Reflection or the unit it copies;
  - +2.5 for hiding a card, +1.5 more when the battlefield has only a Reflection.
- VIDEO, on by default (RB_VIDEO=0 turns it off):
  - copy Ruined Rex and Watcher first;
  - discard a duplicate Karthus when Glasc can bring it back;
  - Watcher closer bonus;
  - +2 for a ready Reflection ganking the other battlefield.
- exp_plans label suffixes: " · LeBlanc réel", " · vidéos". add_replays spec keys: refl / video.

Measured on 80 games: recalls 103 → 61; hides 14% → 38% of the opportunities.
Win-rate effect for Akali, engine plan, paired 400-game runs:
- Reflection rules: +1.8 ± 2.1 (Hook tempo) and +0.2 ± 1.9 (Windswept).
- Video rules: −3.0 ± 1.5 on seeds 91000+, not confirmed on fresh seeds 92000+ (+0.2 ± 1.3).
- Conclusion: no measurable effect; the play is just more faithful.

Viewer v11 group « LeBlanc comme les vrais joueurs ».
Related: [[riftbound-tempo-ai]], [[riftbound-gameplans]], [[retex-method]].
