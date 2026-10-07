# Reprise du projet (état au 7 octobre 2026)

Ce document dit où en est le travail, ce qui restait lié à l'ancien compte Claude, et comment tout recréer.

## 1. État du travail
- **Moteur** : 917 cartes jouables sur 919 (hors runes et jetons). Bloquées : Baron Nashor et Baron Pit (il faudrait
  un troisième battlefield ; le moteur suppose une paire partout). Suite : `python3 test_all.py` = 968/968.
  Détails et décisions de règles par paquet : `riftbound/engine/cardsets/NEEDS_*.md`, guide `cardsets/GUIDE.md`.
- **Table d'entraînement** : dernière version publiée = v12 (768 cartes jouables à ce moment). Le moteur du dépôt
  (917 cartes) **n'a pas encore été publié** dans la page : refaire `bash build.sh` puis publier (§3).
- **IA** : plans propres pour Akali (Gorica) et LeBlanc (Hook tempo) dans `engine/plans.py` ; toute autre légende
  joue avec l'IA générique (plus faible). Piste : écrire des plans pour les légendes que l'utilisateur joue.
- **Parties de l'utilisateur** : 17 parties enregistrées par la table (`riftbound/train/games/games/*.json`) et ses
  decks (`riftbound/train/games/decks/`). Demande en attente : « apprends sur ce que je fais contre LeBlanc, et
  affine l'IA de LeBlanc avec ce que j'ai fait pour gagner ». Outil prêt : `cd riftbound/engine && python3
  train_games.py ../train/games/games/*.json --coach` (rejoue chaque partie, vérifie la fidélité, écrit
  `train/analyses/<id>.md` et `choices.jsonl`). Une partie enregistrée avant le 7/10 au soir a été jouée avec un
  moteur plus ancien : si la vérification de fidélité échoue, le dire, ne pas forcer.
  Mécanisme proposé : un modèle des choix de l'utilisateur sert d'`opp_plan` à LeBlanc au lieu du plan Gorica, mesuré
  sur des graines neuves (règles de `CLAUDE.md`).
- **Agent manager** : sessions de simulation automatiques (voir §4). Ses acquis : `riftbound/manager/learnings.md`.

## 2. Ce qui restait sur l'ancien compte
Ces pages étaient des artefacts claude.ai privés de l'ancien compte : un nouveau compte ne peut pas les ouvrir ni
les modifier. Il faut les republier (nouvelle URL) depuis les sources du dépôt.

| Page | Ancienne URL | Sources |
|---|---|---|
| Riftbound Entraînement (table + éditeur de deck) | https://claude.ai/artifact/HTmfbgtzcPP7jN7UsCfcHC | `riftbound/train/` |
| Riftbound Replays | https://claude.ai/artifact/ThZartFw9uTqb6jcjvpgFV | `riftbound/replays/` |
| Puzzles Akali vs Yi | https://claude.ai/artifact/Ds4TSVoR6tcNRbQKBKYpcP | `riftbound/puzzles/` |

La table enregistrait les parties et les decks dans la base de l'artefact (capacité `db`, collections `games` et
`decks`) : elles sont exportées dans `riftbound/train/games/`. Sur un nouvel artefact, la base repart vide.

## 3. Republier la table d'entraînement
```bash
cd riftbound/train
bash fetch_pyodide.sh      # une fois (npm pack pyodide@0.26.4 ; le CDN jsdelivr est souvent bloqué)
bash build.sh              # -> build/train.html + build/py, build/pyodide, build/atlas, build/img, build/data
python3 serve_csp.py &     # test local avec la politique de sécurité de claude.ai
python3 dk2.py 1400 900    # robot Playwright : partie complète avec deux decks au hasard (adapter le port/chemins)
```
Publication (outil Artifact de Claude) : `file_path` = `build/train.html`, `root` = `build`, `files` = tous les
fichiers de `build/` sauf train.html (les `.py` avec `contentType: text/plain`), `capabilities: {"db": {}}`.
Pièges connus :
- la page claude.ai interdit les URL `blob:` et le `fetch` d'un blob : la bibliothèque standard Python est publiée en
  base64 (`python_stdlib.b64.txt`) et servie par une interception de `fetch` (déjà dans `src.html`) ;
- `.zip` n'est pas servi par les artefacts, d'où le base64 ;
- 255 fichiers au plus par publication ; la version complète en compte environ 110 ;
- dans un test Playwright, la politique de sécurité bloque `wait_for_function` : sonder avec `evaluate` en boucle.
`src.html` est la copie maîtresse de la page (CSS dans `base.css`, injecté par build.sh). Les vieux robots
`dnd*.py`, `tr.py`, `askt.py` testaient les versions v3 à v10.

## 4. Routine de l'agent manager (à recréer)
Ancienne routine : « Agent manager Riftbound (sessions matchups) », cron `2 */6 * * *` (toutes les 6 h), qui
réveillait le fil « Agent manager de sessions matchups » du projet. Prompt stocké :

> Nouvelle session de l'agent manager Riftbound. Suis riftbound/manager/PROTOCOLE.md de bout en bout : relis
> learnings.md et le dernier retex, garde ou écris agenda.json, lance run_session.py en arrière-plan, puis quand il a
> fini écris le retex de la session, mets à jour learnings.md, prépare l'agenda suivant, mets à jour le vault
> (docs/vault/) et poste une seule réponse courte en français dans ce fil. Si une session tourne encore (garde-fou
> du protocole), ne lance rien et termine avec no_reply_needed.

Après chaque session : committer `riftbound/manager/` et `docs/vault/` dans ce dépôt. L'utilisateur n'a pas tranché
s'il garde la cadence de 6 h (question posée le 6/10) : la garder tant qu'il ne dit rien.
Le runner fige tout le moteur au début de chaque session (`manager/results/engine_session_NNN/`) ; ne jamais
agréger des chiffres de deux versions du moteur sans le test d'équivalence (`manager/eqtest.py`).

## 5. Mémoire
`docs/memoire/` contient la mémoire des agents (un fait par fichier, index `MEMORY.md`). Certains fichiers parlent de
chemins `/mnt/project-files/...` : c'était la racine du dossier partagé du projet, l'équivalent de la racine de ce
dépôt (`/mnt/project-files/riftbound/...` = `riftbound/...`, `/mnt/project-files/vault/` = `docs/vault/`).
Un nouvel agent peut importer ces fichiers dans sa propre mémoire, ou simplement les lire.

## 6. Réseau (environnement cloud Claude Code de l'ancien compte)
api.rifthunt.com et riftbound.gg (API WordPress) passaient ; api.riftcodex.com, api.dotgg.gg et les sites de
decklists (riftdecks.com, mobalytics…) étaient bloqués ; YouTube aussi (les transcriptions de vidéos ont été collées
par l'utilisateur). Le registre npm passait, jsdelivr non.
