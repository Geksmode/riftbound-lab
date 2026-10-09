---
name: riftbound-engine
description: Faithful Riftbound rules engine in /mnt/project-files/riftbound/engine (all pool cards modelled, 91 tests) and why the old sim/ is superseded
metadata:
  type: project
  modified: 2026-10-02T17:22:55.642Z
---
2026-10-02: the user rejected the simplified simulator ("je ne veux pas des simplification, il faut savoir modeliser toutes les cartes"), so a faithful engine was written in /mnt/project-files/riftbound/engine/ from the Core Rules dated 2026-07-16 (rules/source/core_rules_2026-07-16.txt).

Structure: game.py (state, cost payment, chain with priority/focus, cleanups, showdowns, combat, scoring, layers), actions.py (legal actions + process of play), cards.py (one Impl per card name — 85 pool cards + Mech/Reflection/Gold tokens; extensible to the other 938 cards by adding entries), ai.py (SearchAgent = 1-ply search on game clones with a 2-turn PolicyAgent rollout, hidden info determinized), decks.py, test_cards.py (91 scenario tests, every pool card covered), run.py + exp_*.py (multiprocess Monte Carlo, ~1.2 s/game on 4 cores). See engine/README.md for modelled rules and limits.

Calibration: stock Wuhan Akali vs IQ#5 LeBlanc = 37.5% +-5 (real data ~35%); the old simplified sim/ gave 31% and is superseded — keep it only for history. Equipment Might bonuses are not in the card DB (Long Sword +2, Sterak's Gage +3, Pendulum Blade +1, hardcoded in game.EQUIP_BONUS); Zhonya's Hourglass uses the errata text.

Related: [[akali-vs-leblanc]], [[riftbound-sources]], [[riftbound-lab-goal]].

## Leçons (retex 2026-10-09, validées par l'utilisateur)
- **Quand** je modélise un mot-clé, **le lire dans le texte imprimé (en tête du texte) ou sur l'image, pas dans la colonne `keywords` du CSV**, qui range aussi les simples mentions. *Pourquoi :* Ember Monk, Ava Achiever et Pack of Wonders se cachaient à tort (811.1). *(PR #9)*
- **Quand** un effet ne s'applique pas (condition de tour, aucune cible), **l'écrire au journal**, et ne jamais demander « utiliser l'effet ? » sans cible légale (402.4, `trig_target.options` dans `flush_triggers`). *Pourquoi :* retour de l'utilisateur sur Evelynn, Entrancing. *(2026-10-09)*
- **Dans un test qui passe par `train.act` / `train.answer`**, relire les objets par `g.obj(uid)` après chaque appel : l'état est restauré par copie profonde, les anciennes références sont périmées. *Pourquoi :* fausse alerte de bug sur Evelynn. *(2026-10-09)*
- **Les choix d'un humain suivent le texte** (« a unit » = toute unité, amie ou déjà étourdie ; deux unités de même Might sont légales ; zéro cible pour « up to ») ; l'IA garde sa liste courte via `full_choices`. *Pourquoi :* Back Off, Switcheroo, et avant Singularity, Fox-Fire. *(PR #9)*

