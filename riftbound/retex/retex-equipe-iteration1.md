# Retex : première itération de l'équipe d'agents (2026-10-07 au 08)

Revue autocritique, par le chef. Source : rapports de `cartes` et `ia`, `riftbound/train/captures-menu/RAPPORT.md` (design), et mes propres relances.

## Ce qui tient (vérifié par moi après fusion)
- `test_all.py` = **1034/1034** (968 avant) et `t_rand.py 30` = 0 erreur, sur la branche fusionnée ; wiki lint à 0 problème.
- `cartes` : 7 écarts à la règle 355.13 corrigés (Singularity, Fox-Fire, Bellows Breath, Tricksy Tentacles, Disposal Order, Volibear, Fae Dragon), chacun avec un test écrit d'abord. Shuriken Flip, l'exemple de l'utilisateur, marchait déjà.
- `design` : menu d'accueil lu sur captures à 360 et 1400 px (lisible, 5 entrées, grille 1 à 3 colonnes). Je n'ai pas rejoué ses parcours.
- `ia` : il a détecté tout seul que son premier test était invalide (parties miroir identiques) et l'a marqué comme tel.

## Ce qui était surévalué ou ne doit pas être cru
- **IA générale** : 51,9 % ± 3,0 sur 79 paires = « on ne sait pas » (0,6 écart-type). Deux séries identiques ont donné 58,8 % et 47,5 % : la résolution du banc est de ±3-4 points à 40-80 paires. Aucun gain n'existe.
- **Ressources de l'IA** : WebFetch bloqué par le proxy, 10 sources repérées par extraits, aucune lue en entier. Les « principes » sont des hypothèses de confiance basse, pas des connaissances.
- **« Testé » pour le design** : le robot pilote Chromium par DevTools, les coups sont injectés (pas de clic sur les cartes), la base claude.ai est simulée en mémoire, rien sur GitHub Pages. Au sens de la règle 6, le menu n'est **pas testé de bout en bout**.
- **Règle 4 violée par `cartes`** : Gentle Gemdragon et Hwei restent avec une approximation assumée (« ready up to 2 runes » prépare toujours le maximum). La règle dit : jamais d'effet approximé, une carte bloquée vaut mieux. À trancher (modéliser, ou bloquer).
- **Chiffres de simulation** : le moteur a changé (listes de choix, Fae Dragon). Aucun chiffre d'avant n'est comparable ; effet sur le niveau de l'IA non mesuré.

## Biais et erreurs du chef
- Missions trop larges pour une itération : 32 cartes sur ~900 auditées ; un agent ne peut pas tenir « toutes les cartes ».
- J'ai écrit « pas de push » et demandé des rapports dans `$RB_EQUIPE`, mais le sandbox de `design` interdisait cette écriture : rapports dispersés (dépôt vs boîte aux lettres).
- Un redémarrage du conteneur a tué tmux et le processus de `design` avant son rapport final ; sans sauvegarde `agent/*` poussée, son travail était perdu. La sauvegarde a sauvé l'itération.
- Mode `claude -p` : aucun suivi en direct (journal vide jusqu'à la fin). J'ai décrit « le script est testé » alors que seule la mécanique avec `echo` et une tâche triviale l'était.

## Ce qu'on change
1. Chaque mission a un **critère de fin chiffré** et le rapport est **écrit dans le dépôt** (`docs/rapports/<agent>-<date>.md`), pas seulement dans `$RB_EQUIPE`.
2. Sauvegarde automatique des branches `agent/*` (script versionné) dès le lancement, et `--output-format stream-json` pour suivre l'avancement.
3. Itérations plus petites : un paquet de cartes ou une seule hypothèse d'IA à la fois, avec ≥ 200 paires et confirmation sur graines neuves.
4. Transmettre à `cartes` le bug trouvé par `ia` : `TypeError ... multiple values for keyword argument 'tg2'` dans `game.py::repeat_resolution` (1 partie sur 79).
5. `design` : `build.sh` doit copier `replays/viewer.html` (le lien Replays reste masqué sinon) ; bulle de fin de partie qui chevauche la barre de commande à 360 px ; boutons ± de l'éditeur trop petits au toucher.
6. Régénérer `train/engine_versions/` avant toute republication de la table.
