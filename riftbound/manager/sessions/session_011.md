# Session 011 — 2026-10-05  (moteur 7daf3a6f86, plans.py 8b2147fcef)

**Version de l'IA.** Le hash du moteur change (44d6a1b695 → 7daf3a6f86), mais la seule différence avec le moteur
figé de la session 10 est un **nouveau fichier `train.py`** (fil Replays, 16:49 UTC), qu'aucun module du jeu
n'importe : tous les autres `.py` sont identiques octet pour octet (`cmp` sur `engine_session_010` et
`engine_session_011`), et `plans.py` est inchangé. Je traite donc la session 11 comme la **même version de jeu
que s004-s010** et j'autorise le cumul avec la session 10. Si `train.py` venait à être importé par le jeu, ce
raisonnement tomberait.

Graines neuves 204400-204799, liste stock, plan Gorica contre LeBlanc « Hook tempo Windswept ».

## Questions posées
1. **Fixée à l'avance** : avec le plan, présenter Sigil of the Storm plutôt que Void Gate (forcé), ça vaut quoi ?
2. **Rejeu sur bloc neuf** de la piste s010 : avec le plan, Forgotten Monument au lieu de Sigil coûtait 10,2 ± 3,4.

## Résultats
| Config | n | Winrate ± ET | Akali commence / LeBlanc commence | Écart apparié à la référence |
|---|---|---|---|---|
| **REF** plan Gorica, Sigil (choix du plan) | 400 | 45,5 % ± 2,5 | 53,8 / 37,3 | — |
| plan Gorica, Void Gate forcé | 400 | 36,8 % ± 2,4 | 45,7 / 27,9 | **−8,8 ± 3,3** |
| plan Gorica, Forgotten Monument forcé | 400 | 43,8 % ± 2,5 | 50,8 / 36,8 | −1,8 ± 3,3 |

Cumuls (blocs indépendants, même version de jeu) :
- Forgotten Monument − Sigil, avec le plan : −10,2 ± 3,4 (s010) et −1,8 ± 3,3 (s011) → **−5,9 ± 2,4** en cumul
  pondéré. Mais le chiffre de s010 a été promu *parce qu'il était grand* : le cumul est tiré vers le bas par
  la sélection. Seul le bloc neuf est propre, et il dit « on ne sait pas ».
- Niveau de la référence : 49,8 % (s010) puis 45,5 % (s011) → ~47,5 % ± 4 (dispersion entre blocs).

## Ce que ça change
- **Piste forte, question fixée à l'avance** : face à Windswept, présenter **Void Gate coûte ~9 pts** à Akali
  par rapport à Sigil (−8,8 ± 3,3, 2,7 écarts-types). L'effet est le même qu'Akali commence ou non (−8,1 et
  −9,4). C'est cohérent avec ce que fait déjà le plan Gorica (Sigil dès que LeBlanc est sur Windswept), avec
  Metafy et avec le classement de la session 2 (Sigil − Void Gate +4,0 ± 3,4, ancienne IA). Je la laisse en
  pistes jusqu'à un deuxième bloc, comme pour le plan (s7 → s8).
- **Piste Forgotten Monument non confirmée** : −1,8 ± 3,3 sur bloc neuf. Le −10,2 de s010 était au moins en
  partie de la chance. Ce qu'on peut dire : Forgotten Monument n'est pas meilleur que Sigil (cumul −5,9 ± 2,4,
  avec la réserve de sélection) ; l'ampleur est entre 0 et ~10 pts, **on ne sait pas**.
- Classement provisoire des battlefields d'Akali face à Windswept, avec le plan : **Sigil ≥ Forgotten Monument
  > Void Gate**.

## Autocritique
- En s010 j'avais annoncé « Forgotten Monument coûte ~10 pts » dans le message à l'utilisateur, même avec la
  réserve « piste ». Le bloc neuf le ramène à −1,8 : c'était surévalué. La règle « une piste reste une piste » a
  fonctionné, mais le chiffre du message était trop mis en avant.
- J'ai autorisé le cumul malgré un hash de moteur différent. Je l'ai justifié fichier par fichier ; si le fil
  Replays voulait que `train.py` change le jeu (poids appris chargés au démarrage, par exemple), il faudrait
  recompter.
- Le −8,8 de Void Gate est l'écart de la question fixée, pas un écart choisi parmi plusieurs : il n'est pas
  gonflé par la sélection. Il reste un seul bloc.
- Les deux battlefields forcés changent aussi les décisions du plan (il est écrit pour Sigil face à Windswept) :
  « Void Gate coûte 9 pts » veut dire « Void Gate joué avec ce plan », pas la valeur intrinsèque de la carte.
- Le niveau de la référence a baissé de 4 pts entre s010 et s011 : c'est dans la dispersion connue entre blocs.

## À signaler au fil Replays
- Le plan Gorica présente **Void Gate dès que LeBlanc n'est pas sur Windswept** (plans.py, `battlefield` ;
  dans le moteur le plan voit le battlefield de LeBlanc avant de choisir, ce n'est donc pas vraiment « à
  l'aveugle »). Face à Windswept, Void Gate coûte ~9 pts. La session 12 mesure si Void Gate coûte aussi des
  points face à Star Spring (plan « Hook tempo » quand Akali commence) ; si oui, la règle « Void Gate à
  l'aveugle » mériterait d'être revue (c'est au fil Replays de modifier le plan, pas au manager).

## Prochaine session (12)
1. **Fixé** : rejouer Sigil − Void Gate forcé face à Windswept sur bloc neuf (confirmation pour les acquis).
2. Troisième bloc pour Sigil − Forgotten Monument.
3. **Nouvelle question fixée** : contre LeBlanc « Hook tempo » (Windswept quand elle commence, Star Spring
   sinon), plan Gorica avec son choix de battlefield (Void Gate face à Star Spring) contre plan Gorica avec
   Sigil forcé. L'écart ne peut venir que des parties où Akali commence.
