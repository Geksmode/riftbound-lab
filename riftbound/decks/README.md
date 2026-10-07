# Decklists Riftbound

Collecte du 2026-10-02. **riftdecks.com, riftbound.gg, riftools.app, mobalytics.gg, piltoverarchive.com, riftmana.com, magicalmeta.ink, hextechanalytics.com sont bloqués par le réseau** (proxy 403). Les listes ont été reconstituées à partir des résumés du moteur de recherche (WebSearch), puis validées contre `../cards/cards_unique.csv`.

| Fichier | Contenu |
|---|---|
| `raw/*.txt` | 1 fichier par deck : en-tête `# clé: valeur` (legend, player, event, date, placement, url, note) puis sections `[legend] [champion] [main] [runes] [battlefields] [sideboard]` avec `qty nom` |
| `parse_decks.py` | parse `raw/` et joint à la base de cartes, produit les fichiers ci-dessous |
| `decks.json` | decks + cartes jointes (card_id, type, domaine, coûts) + flags de validation |
| `deck_cards.csv` | 1 ligne par (deck, section, carte) |
| `card_frequency.csv` | par légende : % de decks jouant la carte en main, copies moyennes, % en sideboard |
| `validation.txt` | contrôles 40 / 12 runes / 3 battlefields / sideboard ≤ 10 / cartes bannies depuis |
| `matchups/akali-vs-leblanc.md` | analyse du matchup |

Ajouter un deck : créer un `raw/<legende>_<joueur>_<event>.txt` au même format, puis `python3 parse_decks.py`.

Échantillon actuel : 6 Akali, Rogue Assassin + 6 LeBlanc, Deceiver (août-sept. 2026). 3 listes à vérifier (voir validation.txt).
| `matchups/akali-vs-leblanc-moteur-fidele.md` | **référence** : 4 480 parties sur le moteur fidèle (`../engine/`), deck et plan recommandés |
| `matchups/akali-vs-leblanc-simulations.md` | ancienne étude sur le moteur simplifié (périmée) |
