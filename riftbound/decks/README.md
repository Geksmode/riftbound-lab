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

Collecte du 2026-10-09 par recherche web (transcriptions des résultats, pages des decks non lues directement), chaque liste
contrôlée par `check_raw.py`. Règle (`train.default_decks`) : parmi les listes jouables par le moteur, sans carte bannie d'abord,
puis avec sideboard, puis le meilleur classement ; le même deck en BO1 et en BO3. **Les 58 légendes ont un deck** (49 jusqu'au
2026-11-06, puis les 9 de Radiance, section suivante).
Contrôle : 106 parties (chaque deck des deux côtés), 0 erreur, 0 partie sans fin.

- **28 listes récentes** (Vendetta, sans carte bannie) : RQ Los Angeles, Singapore, Barcelona, Wuhan Open…
- **18 listes « Best-Of » d'Unleashed** (RQ Sydney, Vancouver, Utrecht, Hartford, mai-juin 2026), demandées par l'utilisateur pour les
  légendes sans liste récente valide. La plupart contiennent des cartes **bannies depuis** (marquées ⚠ dans la table) : Volibear,
  Jinx, Darius, Ahri, Yasuo, Leona, Teemo, Viktor, Miss Fortune, Rumble, Draven, Ezreal, Jhin, Vi, Poppy, Annie, Lux, Garen.
  Lille et Atlanta (avril 2026) sont écartés : format Spiritforged.
- **3 légendes Vendetta** (absentes d'Unleashed) : Best-Of Vendetta. Mel (Barcelona 19e, Stacked Deck), Zed (Barcelona 164e,
  Stacked Deck), Renekton (Singapore 384e).
- Listes écartées pour une carte alors non modélisée (Baron Nashor, modélisé depuis le 2026-10-19 : elles peuvent être
  ajoutées) : Teemo Sydney 6e, Yasuo Utrecht 112e, Miss Fortune Hartford 13e, Draven Sydney 10e.

**Transcriptions à vérifier** (une ligne `# note:` dans le fichier) :
- Ligne incertaine retirée plutôt que devinée : Yi Bladesman (sideboard 9), Renekton (sideboard 4), Miss Fortune (sideboard 7),
  Garen et Nasus (sans sideboard).
- Quantité choisie parce que c'est la seule qui donne 40 cartes : Ambessa (2e Ambessa, The Wolf), Teemo (Teemo, Strategist),
  Darius (2 Seal of Unity), Rumble (Rumble, Hotheaded, Deadly Flourish), Renekton (2 Sabotage, donnés par une source).
- Runes d'une seule source : Jax, Sivir, Garen, Zed, Renekton. Sideboard d'une seule source : Ambessa, Annie (en partie), Vex, Rengar.
- Places faibles faute de mieux : Shen 450e, Renekton 384e, Renata 311e, Garen 288e, Ivern 277e, Rumble 250e, Ambessa 201e.

## Decks par défaut des 9 légendes de Radiance (2026-11-06)

Demande de l'utilisateur : « trouve des decks par défaut pour les légendes qui n'en ont pas ». Radiance est sorti le 2026-10-23 :
les listes viennent des premiers grands tournois du format, Showdown 25k LDXP Sacramento (2026-10-24, 755 joueurs) et Showdown
CoolStuffCon Orlando (2026-11-01, 556 joueurs). Même méthode qu'en octobre : sites de decks bloqués (proxy), listes reconstituées
à partir des résumés WebSearch (pages riftdecks, riftmana, riftatlas, runesandrift jamais lues directement), validées par les totaux
par catégorie affichés par la page (unités / équipements / sorts), puis par `check_raw.py`. Chaque doute est dans `# note:`.
Contrôle : 54 parties (chaque deck 6 fois, des deux côtés, contre un deck par défaut tiré au hasard), 0 erreur, 0 partie sans fin.

| Légende | Liste | Place | Fiabilité |
|---|---|---|---|
| Riven, The Exile | Bo Su, Sacramento | 3e / 755 | noms recoupés, quantités d'un seul résumé ; le 1er (Nam Dang) avait un sort manquant ; sans sideboard |
| Evelynn, Agony's Embrace | John Riftbound, Sacramento | 21e / 755 | main recoupé par deux résumés ; sideboard 5 / 10 |
| Orianna, Lady of Clockwork | Burrsir, Orlando | 59e / 556 | la plus sûre : deux sources concordantes, sideboard complet |
| Ekko, Boy Who Shattered Time | Tristan Baltazar, Sacramento | 60e / 755 | sorts, runes, battlefields d'un seul résumé ; sans sideboard |
| Jarvan IV, Exemplar of Demacia | Braden Van't Hof, Sacramento | 77e / 755 | quantités d'unités et battlefields d'un seul résumé ; Sett, Kingpin ×2 = seule quantité qui donne 40 ; sans sideboard |
| Ziggs, Hexplosives Expert | Kevin Kim, Sacramento | 85e / 755 | une seule source ; Hard Bargain et Star-Crossed ×1 = seules quantités qui donnent 40 ; sideboard 1 / 10 |
| Mordekaiser, Iron Revenant | Christian Mukai, Sacramento | 89e / 755 | sorts, équipement, runes d'un seul résumé ; la page dit 33 unités dans un encart (29 dans la liste) |
| Seraphine, Starry-Eyed Songstress | cheeserdude, Orlando | 190e / 556 | Raucous Anthem ×1 = seule quantité qui donne 40 ; sideboard 8 / 10 |
| K'Sante, Pride of Nazumah | Sigils, Nexus Night Sacramento | 52e / 64 | **place faible, petit tournoi** : seule liste complète trouvée ; sans sideboard |

Les listes mieux classées non retenues faute d'être complètes : Riven Nam Dang (1er Sacramento), Seraphine Cameron, Orianna
Kevin Yin, Mordekaiser Calvin / AA Cartaneko / TSS KnightinGale, Ekko Moses Addai / AV Aadion, Ziggs Kevin A (Best-Of Orlando),
K'Sante des Showdowns. À remplacer dès qu'une page de deck est lisible (réseau) ou que l'utilisateur colle une liste.
Sans sideboard complet, l'IA joue le BO3 avec le sideboard partiel (elle ne sideboarde pas encore de toute façon).
Ces 9 légendes n'ont pas de plan par légende (`plans_legendes.SANS_PLAN`, plan générique).
