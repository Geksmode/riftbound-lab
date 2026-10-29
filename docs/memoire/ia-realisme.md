---
name: ia-realisme
description: Règle de l'utilisateur (2026-10-29) : un réglage qui corrige des erreurs marquées par l'utilisateur est gardé s'il ne fait pas perdre (deux tests séquentiels de non-infériorité, 48,5 % contre 50 %), sans gain de victoires exigé
metadata:
  type: feedback
---
Le 2026-10-28, l'utilisateur : « il faut remettre en perspective cette règle des 2, il faut que le simulateur se comporte
comme un humain » ; puis, après la correction des unités [Ambush] mesurée ainsi, « oui, l'ajouter » (règle 10 de `CLAUDE.md`).

**Règle :** un réglage de comportement est gardé, même sans gain démontré, s'il corrige des erreurs que l'utilisateur a
**marquées** sur la table (bouton « Erreur de l'IA ») et s'il **ne fait pas perdre** : `mesure/sprt.py` avec
`RB_SPRT_H0=0.485 RB_SPRT_H1=0.50` dit « garde », puis un second test sur des graines neuves dit « garde » aussi.

**Pourquoi :** la règle 9 (+2 points) mesure la force de l'IA contre elle-même ; les erreurs que voit un joueur coûtent
peu entre deux IA (coût moyen d'une erreur du banc −0,8 ± 2,4 points, [[ia-banc-erreurs]]) mais rendent l'IA peu crédible
comme adversaire d'entraînement. Le juge automatique du banc (note après résolution) se trompe souvent sur ce qui fait
gagner (Irresistible Faefolk) : seules les erreurs marquées par l'utilisateur servent de preuve.

**Application :**
- vérifier d'abord sur les positions marquées (`banc/humain.py`, ou plusieurs tirages par position : la recherche a du
  hasard) que le réglage change bien ces coups ;
- puis les deux tests de non-infériorité (decks variés, `defaut`) ; noter les deux résultats et les graines ;
- un réglage gardé ainsi est allumé pour la table (`train.TABLE_CFG`), pas forcément par défaut dans les simulations ;
- la règle 9 reste celle des réglages de force (gain de victoires).
- Premier cas : unités [Ambush] et réaction sans préférence, 688 paires 50,6 % ± 0,8 puis 1 448 paires 49,9 % ± 0,6.

Related: [[ia-sprt]], [[ia-banc-erreurs]].
