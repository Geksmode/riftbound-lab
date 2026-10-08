# Ressources pour l'IA générale Riftbound (consultation du 2026-10-07)

AVERTISSEMENT IMPORTANT : tous les appels WebFetch ont échoué (« proxy refused the connection », y compris sur arxiv.org,
riftbound.gg, ultimateguard.com, tacter.com, eprints.whiterose.ac.uk). Aucune page n'a donc été lue en entier.
Les entrées ci-dessous ne reposent que sur les **extraits et résumés renvoyés par WebSearch** (titres, URLs, 1 à 3 phrases).
Elles sont à relire à la main avant d'être utilisées. Tout ce qui suit est une **hypothèse**, pas une vérité.

## A. Sujet 1 : stratégie Riftbound (vues via résultats de recherche seulement)

### 1. Ultimate Guard, « Riftbound : three concepts to master to become a better player »
- URL : https://ultimateguard.com/en/blog/riftbound-three-concepts-to-master-to-become-a-better-player
- Consultée le 2026-10-07 (extrait de recherche, page non lue).
- Idées (reformulées) : le tempo = garder l'initiative pour que l'adversaire réagisse à nous ; la présence de plateau
  (plus d'unités ou de plus fortes) est une composante du tempo ; dépenser runes et énergie chaque tour, mais parfois en garder
  pour menacer une réaction pendant le tour adverse ; une réaction bien placée (ex. renvoyer en main l'unité ciblée par un buff)
  peut renverser un combat.
- Confiance : moyenne-basse (éditeur de accessoires, vulgarisation ; contenu seulement via l'extrait).

### 2. Tacter, « All You Need to Know About How to Play Riftbound »
- URL : https://www.tacter.com/riftbound/guides/all-you-need-to-know-about-how-to-play-riftbound-9e84e60c
- Consultée le 2026-10-07 (extrait de recherche, page non lue).
- Idées : on marque en conquérant un battlefield et en le tenant au début de son tour ; premier à 8 points ; conquérir sans gagner
  fait piocher ; règle du dernier point (le 8e point par conquête exige d'avoir marqué sur tous les battlefields ce tour-ci, sinon
  il faut tenir) ; chaque joueur choisit secrètement son battlefield ; mulligan : jusqu'à 2 cartes remises sous le deck, une fois.
- Confiance : moyenne (règles recoupant la règle officielle, mais je dois vérifier avec nos Core Rules du 2026-07-16 qui font foi).

### 3. Riftbound.gg / Riot, « OGN How to Play » (PDF officiel)
- URL : https://riftbound.gg/wp-content/uploads/sites/67/2025/06/OGN-HowtoPlay-Website.pdf
- Consultée le 2026-10-07 : repérée dans les résultats, PDF non lu.
- Intérêt : règles de base officielles ; pas de stratégie attendue. Doublon de nos règles locales (`riftbound/rules/`).
- Confiance : haute pour les règles (source officielle), mais non lue ici.

### 4. Metafy, « Conquer the Rift : the strategy guide » (payant)
- URL : https://metafy.gg/guides/view/conquer-the-rift-the-strategy-guide-158qZEq9usC
- Consultée le 2026-10-07 : seulement la fiche vue dans la recherche ; contenu payant non accessible.
- Thèmes annoncés dans la fiche : course au score, présence de plateau, tempo de ressources, calcul de Might lors d'un showdown,
  séquençage, mulligan, construction de deck. Utile comme liste de sujets à mesurer, pas comme source de principes.
- Confiance : basse (aucun contenu lu, une seule note).

### 5. Metafy, « Teemo : best aggro deck » (payant)
- URL : https://metafy.gg/guides/view/teempo-riftbounds-best-aggro-deck-best-tee-dQGHoEX4oNd
- Fiche vue seulement ; les guides par deck vieillissent avec les bans et les sets. Confiance : basse.

## B. Sujet 2 : méthode IA (vues via résultats de recherche seulement)

### 6. Cowling, Powley, Whitehouse, « Information Set Monte Carlo Tree Search » (IEEE TCIAIG, 2012)
- URL de dépôt (notice) : https://eprints.whiterose.ac.uk/75048 (notice seulement, PDF non lu)
- Idées : au lieu de chercher dans des arbres d'états déterminisés, ISMCTS construit un arbre d'ensembles d'information, ce qui réduit
  la « fusion de stratégies » (le déterminisation suppose qu'on connaît les cartes cachées) ; plusieurs variantes selon la source d'incertitude.
- Confiance : moyenne-haute sur le principe (article de référence, résumé cohérent avec ma connaissance), non vérifiée en détail ici.

### 7. Cowling, Ward, Powley, « Ensemble Determinization in MCTS for Magic: The Gathering » (IEEE TCIAIG, 2012)
- URL : https://eprints.whiterose.ac.uk/75050/1/EnsDetMagic.pdf (PDF non lu ; notice : https://eprints.whiterose.ac.uk/75050)
- Idées : M:TG a de l'information cachée (main adverse) et de l'aléa (pioche) ; on échantillonne plusieurs déterminisations
  (mains adverses plausibles), on lance un MCTS par échantillon et on agrège les votes ; l'article compare plusieurs variantes.
  (Les auteurs sont Cowling, Ward, Powley, sans Whitehouse.)
- Confiance : moyenne (notice vue, résultats chiffrés non lus).

### 8. Kowalski et Miernik, « Summarizing Strategy Card Game AI Competition » (arXiv 2305.11814, 2023)
- URL : https://arxiv.org/pdf/2305.11814 (non lu ; résumé vu dans la recherche)
- Idées : bilan de cinq ans de concours sur Legends of Code and Magic ; les premiers gagnants combinaient recherche (Monte-Carlo plat,
  MCTS, minimax) et heuristiques écrites à la main ; la dernière édition a été dominée par des réseaux de neurones.
- Confiance : moyenne (source académique, contenu seulement via résumé indirect).

### 9. Powley, Cowling, Whitehouse, « Information capture and reuse strategies in MCTS » (AIJ, 2014)
- URL : https://repository.falmouth.ac.uk/2264/ (notice vue seulement)
- Idées : réutiliser les statistiques entre noeuds/déterminisations pour mieux exploiter un budget de simulations limité. Confiance : moyenne-basse.

### 10. « Learning to Beat ByteRL: Exploitability of Collectible Card Game Agents » (arXiv 2404.16689, 2024)
- URL : https://arxiv.org/pdf/2404.16689 (non lu)
- Idée : des agents forts en auto-jeu peuvent être exploitables par un adversaire ciblé ; donc ne pas se fier qu'à l'auto-jeu contre soi-même. Confiance : basse-moyenne.

### Non retrouvé via recherche (méthodes citées dans la consigne)
SPSA, CMA-ES, hill-climbing apparié, Hearthstone AI : aucune page n'a été trouvée ni lue pendant cette session. Pas de source citée.

## C. Hypothèses à mesurer (toutes à tester par auto-jeu, graines communes, intervalle ± écart-type)

1. **Poids de la tenue de battlefield** : dans l'évaluation, multiplier la valeur d'un battlefield tenu en début de tour (point certain au tour suivant)
   par k dans {0,5 ; 1 ; 1,5 ; 2} ; mesurer le taux de victoire apparié (mêmes graines et mêmes decks pour chaque k).
2. **Mulligan : garder les unités de coût ≤3 énergie** : comparer « garder les cartes jouables en 3 énergie ou moins, renvoyer le reste (max 2) »
   à « ne jamais mulligan » et à « mulligan des cartes ≥5 ».
3. **Dernier point** : une IA qui, à 7 points, planifie explicitement la conquête de tous les battlefields du tour (règle du dernier point) gagne-t-elle plus
   qu'une IA qui maximise les points immédiats ?
4. **Garder de l'énergie pour une réaction** : comparer « dépenser toute l'énergie » à « garder N énergie quand on tient une réaction en main » ;
   mesurer par matchup (agressif vs contrôle), sans agréger.
5. **Attaquer ou défendre selon l'écart de score** : seuil d'attaque dépendant de (nos points - leurs points) ; tester un seuil de +2 / 0 / -2.
6. **Choix de battlefield** : l'IA choisit-elle mieux en fonction du matchup (corrélation avec le taux de victoire mesuré) qu'au hasard ?
7. **Valeur de la pioche sur conquête** : ajouter un bonus d'évaluation à une conquête qui ne gagne pas la partie ; tester 0 / 0,5 / 1 carte-équivalent.
8. **Déterminisation** : N déterminisations de la main adverse (1, 5, 20) à budget total de simulations fixe ; N optimal ? On ne sait pas avant mesure.
9. **Réglage des poids** : hill-climbing apparié (chaque candidat joué sur les mêmes graines que la référence) puis validation de l'option retenue sur graines neuves ;
   comparer à une recherche aléatoire de même budget pour savoir si le réglage apporte quelque chose.
10. **Exploitabilité** : l'IA réglée en auto-jeu contre elle-même perd-elle contre des adversaires de style différent (agro pur, passif) ? Mesurer sur un panel.

## D. Sources bloquées / non trouvées

- Tous les WebFetch ont échoué (proxy refuse la connexion) : aucune des pages ci-dessus n'a été lue en entier. Bash a aussi été refusé pour diagnostiquer le proxy.
- Non lues et même non repérées : riftbound.gg (articles de stratégie et matchups), tcgplayer, mobalytics, reddit (r/riftboundtcg).
- Non trouvées : papiers sur Hearthstone AI, SPSA, CMA-ES, hill-climbing apparié (aucune recherche abouti dans cette session).
- Le nombre de sources réellement lues est de 0 ; 10 sont seulement repérées. L'objectif de 8 à 14 sources lues n'est donc pas atteint.
- Suite proposée : relancer quand le proxy autorise WebFetch, ou fournir les PDF localement.
