# Retex : autocritique de l'étude Akali vs LeBlanc

Rédigé le 2026-10-02. Objet : relire ce que j'ai affirmé pendant l'étude, séparer ce qui tient de ce qui
était surévalué, et nommer les biais. Chiffres : `engine/results.json` et `engine/results_gorica.json`.
Écart-type d'une ligne de 160 parties : environ ±4 points ; d'une différence entre deux lignes : ±5,5 points.

## 1. Ce qui tient (vérifié)
| Affirmation | Preuve | Solidité |
|---|---|---|
| Couper Long Sword et Defy pour des unités aide contre LeBlanc | Gorica : **+6,6 ± 2,8 pts** sur 640 parties neuves par liste (44,7 % → 51,2 %) ; contre la LeBlanc de LA : +10,4 pts (160) ; Wuhan à battlefield fixé : C3 37,5 % contre stock 30,0 % | Réel, même sens dans 3 contextes. Ampleur : **5 à 10 points**, pas plus |
| La liste de Gorica est meilleure que celle de Wuhan dans le moteur | Gorica stock 44,7 % (640 parties neuves) contre Wuhan stock 31,6 % (320) | Réel |
| Commencer la partie aide | Akali gagne plus en commençant dans les 47 configurations où les deux cas existent | Réel. Ampleur incertaine (voir 2.3) |
| Le moteur applique les règles | 91 tests de scénario, aucune erreur sur ~9 000 parties | Solide pour les 85 cartes du pool |

## 2. Ce que j'ai surévalué
1. **Le gain de G2.** J'ai annoncé 57,5 %, puis « +8,7 points, un écart réel ». J'avais choisi G2 parmi
   5 listes sur le même lot de parties : le meilleur de 5 tirages est gonflé par le hasard. Sur des parties
   neuves seulement, le gain est de **+6,6 points**.
2. **Garder Block et Not So Fast « vaut 10 points ».** C'est faux. La comparaison (47,5 % contre 57,5 %)
   utilisait le lot où G2 avait été gonflé. GB fait le même score que GL et GD (47,5 à 48,1 %) : **on ne sait
   pas** si Block et Not So Fast comptent.
3. **L'initiative « vaut 14 points ».** Ce chiffre additionnait 13 configurations jouées sur les mêmes
   160 donnes, comme si c'étaient 2 080 parties indépendantes. La seule paire contrôlée donne 45,0 % contre
   37,5 % : **7,5 points**, à ±5,5. L'effet existe, son ampleur est plutôt de 5 à 10 points.
4. **« 4 480 parties » et toutes les moyennes groupées.** Tous les jobs de `exp.py` utilisent les mêmes
   graines 0-159. Les regroupements de lignes n'étaient donc pas indépendants, et leur précision était
   surestimée.
5. **Le classement des battlefields d'Akali pour G2** (Sigil 55,6 %, Void Gate 52,5 %) vient du lot gonflé.
   Il ne vaut rien de plus que « indiscernables ».
6. **Windswept Hillock pire pour Akali (30,6 % contre 48,8 %)** : un seul lot. 18 points, c'est 3 écarts-types,
   donc probablement réel, mais jamais rejoué.
7. **La « LeBlanc sidée » est une invention.** Je l'ai construite moi-même (+LeBlanc, Everywhere At Once,
   +Thousand-Tailed Watcher). Aucun sideboard LeBlanc réel n'a été lu. Les chiffres « vs LeBlanc sidée »
   testent une hypothèse, pas le vrai side adverse.

## 3. Biais que j'ai eus
- **Annoncer le premier lot comme un résultat.** C'est arrivé trois fois : avec l'ancien simulateur
  (Targon's Peak, le mulligan agressif, la pénalité Vi), avec C3 à 45 % (39 % ensuite), puis avec G2 à 57,5 %.
  Chaque fois, la confirmation a fait baisser le chiffre.
- **Calibrer vers le chiffre attendu.** J'ai changé l'IA quand elle donnait 52 %, parce que le terrain
  disait ~35 %. Le changement se justifiait (des déroulements qui ne font que passer sont irréalistes),
  mais je l'ai accepté parce qu'il rapprochait le chiffre de la cible.
- **Une référence terrain non vérifiée.** Le « 35,2 % sur 236 matchs » vient d'un résumé de moteur de
  recherche (Riftools), jamais lu sur la page. Toute la calibration repose dessus.
- **Comparer ce qui n'est pas comparable.** Dans le retex riftbound.gg, j'ai jugé « cohérent » le winrate
  global d'Akali (45 %, contre tout le field) avec la simulation contre LeBlanc seule (44 %). Ce ne sont pas
  les mêmes quantités.
- **Le niveau absolu est probablement optimiste pour Akali.** Le moteur donne 44,7 % à Gorica contre
  LeBlanc, alors que le seul chiffre terrain par matchup est ~35 %. Explication possible, non testée : l'IA
  joue mal les lignes complexes de LeBlanc (Hidden, Baited Hook).
- **Interpréter au lieu de demander.** J'ai lu « le retex du website » comme « les articles du site » et
  construit un outil de collecte qui n'était pas demandé.

## 4. Ce qui n'est pas modélisé ou pas vérifié
- Les cartes hors pool : Vilemaw's Lair, Decree of Rage, Decree of Focus.
- Les bonus des équipements (Long Sword +2, Sterak +3, Pendulum +1), lus sur les cartes et absents de la
  base.
- Les decklists venues de résumés de recherche : les quantités ont été validées 40/12/3, pas carte à carte.
- L'IA est à un coup, identique des deux côtés, et elle ne bluffe pas.

## 5. Recommandations corrigées
- **Contre LeBlanc** : sors 2 Long Sword et 3 Defy, rentre 5 unités. Gain attendu : 5 à 10 points.
- **Contre le field** : garde la liste de Gorica. Rien dans l'étude ne mesure le reste du field.
- **Commence** quand tu as le choix.
- **Battlefields, mulligan, Not So Fast, Block, jouer autour de Vi** : pas de recommandation, les écarts
  sont dans le bruit.

## 6. Ce que je change dans ma méthode
1. Je ne choisis plus une option et ne la chiffre plus sur le même lot. Toute option retenue est rejouée
   sur des graines neuves (`run_job(..., offset)`), et seul ce chiffre est annoncé.
2. Je ne regroupe plus des lignes jouées sur les mêmes graines comme si elles étaient indépendantes.
3. J'annonce chaque gain avec son intervalle, et je dis « on ne sait pas » sous 2 écarts-types.
4. Une source terrain n'est citée comme chiffre que si je l'ai lue moi-même.
5. Si une demande est ambiguë, je pose une question fermée avant de construire.
