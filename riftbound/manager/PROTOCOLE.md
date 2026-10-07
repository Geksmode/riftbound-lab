# Protocole de l'agent manager (une session)

But : à chaque session, apprendre quelque chose de vrai sur Akali Heron (Gorica) et ses matchups, en partant
de ce que les sessions précédentes ont appris, puis écrire un retex autocritique. La routine relance une
session toutes les 6 heures (elle remplace la détection du renouvellement des tokens, que Claude ne voit pas).

## Étapes
1. **Relire** `learnings.md`, le dernier retex de `sessions/`, `agenda.json`, et la note du vault
   `vault/Questions ouvertes.md`. Regarder si d'autres fils ont changé le moteur (`ls -lt ../engine`),
   surtout `plans.py` : ne jamais modifier les fichiers du moteur ni de `plans.py` depuis le manager.
   Les plans (`plans.py`, `../gameplans/`) appartiennent au fil « Replays graphiques des simulations » :
   utiliser les plans actuels de `../gameplans/gameplans.md`, ne pas refaire les rejeux qu'il fait déjà, et
   noter dans le retex ce qu'il faudrait lui signaler. Le runner fige `plans.py` au début de la session.
2. **Agenda.** Si `agenda.json` porte `session_prevue` = session suivante (`state.json` + 1), le garder.
   Sinon l'écrire : 3 à 5 jobs, 400 parties chacun, tirés des questions ouvertes du §4 de `learnings.md`.
   Le premier job est la **référence** de la session ; les autres s'y comparent en apparié (mêmes graines).
   Une option qu'on veut faire passer en « Acquis » est rejouée avec `"fresh": true`.
2b. **Version de l'IA.** Le runner fige tout le moteur au début (`results/engine_session_NNN/`) et note
   `engine_md5` dans les résultats. Comparer à la session précédente : si la version a changé (le fil Replays
   améliore l'IA et les plans), **ne jamais agréger** les chiffres d'avant et d'après, le dire dans le retex, et
   prévoir dans l'agenda une référence qui rejoue un acquis important avec la nouvelle IA.
3. **Jouer** : `python3 run_session.py agenda.json 4800 > results/session_NNN.log 2>&1` (en arrière-plan, environ 13 min par job).
   Le runner réserve un bloc de graines neuf dans `state.json` : ne jamais l'éditer à la main.
   **Session interrompue** (conteneur redémarré : le .json n'a pas tous les jobs et le processus a disparu) :
   `RB_RESUME=NNN python3 run_session.py agenda.json 6000 >> results/session_NNN.log 2>&1` rejoue seulement les jobs
   manquants, sur le moteur figé et les graines de la session. Le dire dans le retex.
   **Moteur modifié depuis la session précédente** (un `.py` du jeu ou de `engine/cardsets/` diffère, hors chemin ROOT) :
   avant de cumuler, rejouer les deux premiers jobs de la session précédente sur ses 200 premières graines avec le moteur
   de la nouvelle session (copie dans le scratchpad, script type `eqtest.py` : importer run_session depuis une copie
   rb/manager + rb/engine, `run(job, seed0)`, comparer `per_seed`). Toutes identiques → même version de jeu, cumuler et
   le dire dans le retex ; sinon nouvelle version, ne rien cumuler. Le runner fige aussi `cardsets/` depuis s018.
4. **Retex** dans `sessions/session_NNN.md` (gabarit ci-dessous). Règles (mémoire `retex-method`) :
   - annoncer chaque écart avec son intervalle, et « on ne sait pas » sous 2 écarts-types ;
   - ne jamais regrouper des lignes jouées sur les mêmes graines comme si elles étaient indépendantes ;
   - une option choisie parmi plusieurs sur un lot est gonflée : elle reste en « Pistes » jusqu'à un lot neuf ;
   - dire ce que la session précédente avait surévalué, si c'est le cas.
5. **Mettre à jour** `learnings.md` : déplacer les lignes entre Acquis / Pistes / Réfuté, réécrire les
   questions ouvertes, ajouter la ligne d'historique. Puis préparer `agenda.json` pour la session suivante
   (`session_prevue` = N+1).
5b. **Replays** (demande de l'utilisateur, 2026-10-03) : `python3 pick_games.py NNN` écrit
   `results/session_NNN_picks.json` (2 à 4 parties marquantes, rejouables à l'identique). Remplacer les
   titres et notes génériques par une phrase sur ce qui se passe (`python3 ../engine/add_replays.py digest …`
   pour une partie sans battlefield forcé). Puis `cd ../engine && python3 add_replays.py picks
   ../manager/results/session_NNN_picks.json` (outil du fil Replays : il enregistre les parties et met à jour
   `replays/games/index.json`). Enfin republier l'artefact https://claude.ai/artifact/ThZartFw9uTqb6jcjvpgFV :
   lire d'abord le `games/index.json` publié, puis publier `../replays/viewer.html` avec `files` = le nouvel
   `games/index.json` et les nouveaux `games/sNNN-*.json`. Ne jamais modifier replay.py, add_replays.py,
   curate_replays.py ni le viewer.
   **Si le moteur vivant a changé depuis le début de la session** (comparer `results/engine_session_NNN/*.py` à
   `../engine/`), enregistrer les parties avec le moteur figé, sinon elles ne se rejouent pas à l'identique :
   copier `results/engine_session_NNN` dans un dossier de travail `rb/engine`, créer le lien `rb/replays` →
   `riftbound/replays`, puis lancer `python3 add_replays.py picks <chemin absolu des picks>` depuis `rb/engine`
   (aucun avertissement « ATTENTION » attendu).
6. **Vault** (`docs/vault/ (dans ce dépôt)`) : mettre à jour `Akali vs LeBlanc.md`, `Questions ouvertes.md`,
   et `Chiffres périmés.md` pour tout chiffre remplacé ; ajouter la session à `Agent manager.md`.
   Le vault réel de l'utilisateur est sur son PC (Obsidian « Cerveau ») : ne pas y écrire sans qu'il le demande.
7. **Message** : une seule réponse courte dans le fil, en français : ce qui a changé dans ce qu'on sait,
   le chiffre clé avec son intervalle, et la question de la prochaine session.

## Gabarit du retex
```
# Session NNN — AAAA-MM-JJ  (moteur engine_md5, plans.py plans_md5)
## Questions posées
## Résultats (tableau : config, n, winrate ± ET, écart apparié à la référence ± ET)
## Ce que ça change (acquis / pistes / réfuté)
## Autocritique (ce que j'ai pu surévaluer, biais, limites du moteur)
## Prochaine session (questions et jobs)
```

## Garde-fous
- Pas plus d'une session à la fois : si `results/session_NNN.log` le plus récent ne contient pas
  « TERMINÉE » et date de moins de 2 h, ne rien lancer et finir le tour.
- Si une carte de l'agenda n'est pas modélisée, le runner refuse avant de jouer : changer l'agenda.
- Pas de nouvelle carte ni de changement des règles du moteur dans une session automatique : le noter en
  question ouverte pour l'utilisateur.
