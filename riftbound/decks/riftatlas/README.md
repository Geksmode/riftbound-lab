# Collecte des listes de tournoi sur riftatlas.com

riftatlas.com est le seul site de decks lisible depuis le conteneur (2026-11-24) ; ses données de tournoi viennent de
TopDeck.gg. Les pages sont lues directement (pas de transcription par recherche web). `robots.txt` interdit `/api/` : on ne lit
que les pages. Tout se lance depuis ce dossier ; les téléchargements vont dans `ra/` (ignoré par git).

| Étape | Commande | Rôle |
|---|---|---|
| 1 | `mkdir -p ra && node classements.cjs <slug> <slug>…` | ouvre `riftatlas.com/events/<slug>` dans Chromium, déplie « All N standings », écrit `ra/ev_<slug>.json` (rang, joueur, légende, lien du deck). Les slugs sont dans la page `/events`. |
| 2 | `python3 candidats.py` | décode le code de chaque deck (`decode.py`, aucun téléchargement), contrôle 40 / 12 / 3, marque les cartes bannies (colonne `banned_standard` de `cards_unique.csv`), classe les listes par légende, écrit `cands.json` |
| 3 | `xargs -a urls.txt -P 6 -n 1 python3 page_communaute.py` puis `python3 communaute.py` | secours pour les légendes sans liste de tournoi légale : decks communautaires (URLs du `sitemap.xml`, filtrées sur `lastmod`), écrit `com.json` |
| 4 | `python3 choisir.py` | valide les candidats avec le moteur (`rust/target/release/check_decks`) et garde le premier valide par légende : `pick.json` |
| 5 | `python3 verifier_pages.py` | télécharge la page de chaque liste retenue et compare chaque ligne « N x Carte » au deck décodé |

Les fichiers `../raw/*.txt` sont ensuite écrits à partir de `pick.json` (en-tête `# defaut: <date>` pour la liste retenue),
puis `python3 ../parse_decks.py` et `python3 rust/tests/gen_decks.py > rust/tests/data/decks.txt` (depuis la racine).

**Code de deck** (`decode.py`) : base32, suite de varints. Un octet d'en-tête, puis pour chaque quantité de 12 à 1 :
nombre de groupes, et par groupe `nombre de cartes, extension, variante, numéros de collection…` (extensions : 0 OGN,
1 OGS, 3 SFD, 4 UNL, 5 VEN, 6 RAD). Légende, runes et battlefields sont dans le même bloc (on les sépare par le type de la
carte). Suit le sideboard (quantités 3 à 1), puis `1, extension, variante, numéro` du champion choisi (déjà compté dans le
bloc principal). Vérifié sur 58 pages ; 15 codes sur 2 865 ne se décodent pas (ignorés).

**Règle de choix** (`candidats.py`, fonction `key`) : sans carte bannie ; sideboard d'au moins 8 cartes ; tournoi d'au moins
16 joueurs ; liste de la première moitié d'un tournoi du format en cours (date ≥ `RAD`), sinon de la première moitié d'un
tournoi plus ancien, sinon le reste ; puis le meilleur rang relatif (rang / joueurs). Les decks communautaires ne servent
que s'il n'y a aucune liste de tournoi valide (plus copiés d'abord).
