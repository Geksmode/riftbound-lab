---
name: ia-tests-decks-varies
description: Règle de l'utilisateur (2026-10-10) : les tests de l'IA se font sur des decks variés (exp_search mode defaut), nombre de paires raisonnable, plus d'Akali contre LeBlanc par défaut
metadata:
  type: feedback
---
Le 2026-10-10, l'utilisateur a dit : « pour les prochains tests, tu peux retirer l'idée de faire que du akali vs leblanc, maintenant c'est juste sur des decks variés avec un nombre de paires raisonnable ».

**Pourquoi :** l'IA doit bien jouer n'importe quel deck ; le seul matchup Akali contre LeBlanc coûte cher et ne dit rien des autres decks.

**Application :**
- un réglage de l'IA (`ai.py`, poids `ai.EV`, recherche) se mesure avec `engine/exp_search.py <paires> <graine0> <A> <B> defaut`
  (deux decks par défaut tirés au hasard parmi les 49 légendes, places échangées) ;
- nombre de paires raisonnable : environ 80 paires pour un criblage (≈ 15 min sur 4 processus), puis une confirmation sur des
  graines neuves de taille comparable pour une option retenue (règle 2 de `CLAUDE.md` inchangée : ± écart-type, « on ne sait pas » sous 2 écarts-types) ;
- ne lancer Akali contre LeBlanc (mode `akali`) que si l'utilisateur le demande, ou pour un plan propre à ces decks (`plans.py`).
Related: [[riftbound-search-ai]], [[retex-method]].
