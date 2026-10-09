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

Ajouter un deck : créer un `raw/<legende>_<joueur>_<event>.txt` au même format, le contrôler avec `python3 check_raw.py raw/<fichier>` (noms, 40 / 12 / 3, sideboard, cartes bannies ou non modélisées), puis `python3 parse_decks.py`.

Échantillon actuel : 6 Akali, Rogue Assassin + 6 LeBlanc, Deceiver (août-sept. 2026). 3 listes à vérifier (voir validation.txt).
| `matchups/akali-vs-leblanc-moteur-fidele.md` | **référence** : 4 480 parties sur le moteur fidèle (`../engine/`), deck et plan recommandés |
| `matchups/akali-vs-leblanc-simulations.md` | ancienne étude sur le moteur simplifié (périmée) |

## Decks par défaut de l'IA (écran « Contre l'IA », 2026-10-09)

Collecte du 2026-10-09 par recherche web (transcriptions, pages des decks non lues directement). Règle (`train.default_decks`) : parmi
les listes jouables par le moteur, celle avec sideboard d'abord, puis le meilleur classement ; le même deck en BO1 et en BO3.
**28 légendes sur 49** ont un deck. Bilan des 56 parties de contrôle (chaque deck des deux côtés) : 0 erreur, 0 partie sans fin.

**Sans deck (21)** : la plupart des listes publiées datent d'avant les bans (Aspirant's Climb, The Arena's Greatest, Stealthy Pursuer le
2026-07-24 ; Stacked Deck, Ekko, Recurrent le 2026-09-18), et le quota de recherches web du tour a été épuisé. Volibear, Jinx, Darius,
Ahri, Yasuo, Leona, Teemo, Viktor, Miss Fortune, Rumble, Draven, Ezreal, Jhin, Vi, Poppy, Renekton, Zed, Mel, Annie, Lux, Garen.

**Transcriptions à vérifier** (une ligne `# note:` dans le fichier quand le doute est noté) :
- Ambessa : 22 unités visibles sur 23, la 23e (2e Ambessa, The Wolf) déduite du total ; sideboard d'une seule source.
- Master Yi Bladesman : dernière carte du sideboard coupée, retirée (sideboard 9).
- Jax : runes d'une seule source (6 Calm / 6 Body) ; sans sideboard. Sivir : runes et battlefields d'une copie communautaire ; sans
  sideboard. Nasus : sideboards trouvés contradictoires, laissé vide.
- Kha'Zix : runes, battlefields et sideboard d'une deuxième source. Kennen : 2 Ravenbloom Prefect au sideboard (lecture difficile).
- Places faibles faute de mieux : Shen 450e, Renata 311e, Ivern 277e, Ambessa 201e. Listes d'avant Vendetta : Lee Sin, Lillia, Vex, Sett.
