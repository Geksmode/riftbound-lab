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

Ajouter un deck : créer un `raw/<legende>_<joueur>_<event>.txt` au même format, le contrôler avec `python3 check_raw.py raw/<fichier>` (noms, 40 / 12 / 3, sideboard, cartes bannies ou non modélisées ; il appelle le moteur : `cd rust && cargo build --release` d'abord), puis `python3 parse_decks.py` et, depuis la racine, `python3 rust/tests/gen_decks.py > rust/tests/data/decks.txt`.

**Les decks par défaut actuels sont ceux de la dernière section (2026-11-24, ban list du 2026-11-23).** Les sections d'avant sont l'historique.

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

## Decks par défaut refaits pour la ban list du 2026-11-23 (2026-11-24)

Demande de l'utilisateur : « il faut que les deck lists de base respectent la ban list récente ; mets à jour les deck lists de
toutes les légendes par les derniers tournois ». Ban list : `docs/vault/Ban list.md` (8 cartes et 2 battlefields de plus, annoncés
le 2026-11-13, effectifs le 2026-11-23).

**Source** : riftatlas.com (données TopDeck.gg), lisible directement depuis le conteneur. 28 tournois du 2026-09-19 au
2026-11-22, classements complets, **2 647 listes** décodées (méthode et scripts : `riftatlas/README.md`). Chaque liste retenue
est comparée ligne à ligne à sa page (58 / 58 identiques), puis validée par `check_raw.py`.

**Ce qu'il faut savoir** : le ban est effectif depuis le 23/11 et **aucun tournoi joué avec la nouvelle ban list n'a encore de
listes publiées** (dernier tournoi lu : 22/11). Les listes retenues sont donc des listes d'avant le ban **qui ne jouaient
aucune des cartes bannies** : elles sont légales, mais ce ne sont pas des listes construites pour le nouveau format. À refaire
quand les premiers tournois d'après le 23/11 seront publiés (`riftatlas/README.md`, une heure environ).

**Règle de choix**, par légende : liste sans carte bannie et entièrement modélisée ; sideboard d'au moins 8 cartes ; tournoi
d'au moins 16 joueurs ; d'abord une liste de la première moitié d'un tournoi du format Radiance (depuis le 2026-10-23), sinon
de la première moitié d'un tournoi Vendetta (19/09 au 18/10), sinon le reste ; puis le meilleur rang relatif. Résultat :
**37 listes Radiance** (dont 12 des 21 et 22 novembre), **16 listes Vendetta**, **5 decks communautaires** (Akali, Darius,
Lee Sin, Leona, Yasuo : aucune liste de tournoi légale ; deck riftatlas modifié après l'annonce des bans, le plus copié,
**jamais classé en tournoi**).

Le moteur prend ces listes grâce à l'en-tête `# defaut: 2026-11-24` (`train::default_decks` : sans carte bannie, puis
`# defaut:`, puis sideboard, puis classement ; test `rust/tests/decks_defaut.rs` : chaque légende a un deck par défaut valide
et sans carte bannie). Contrôle : 348 parties entre ces 58 decks (116 avec l'IA de la table, 232 avec des agents au hasard,
chaque deck des deux côtés), 0 erreur, 0 partie sans fin.

| Légende | Joueur | Tournoi | Place | Score | Sideboard |
|---|---|---|---|---|---|
| Ahri, Nine-Tailed Fox | Motoriffic | Convergence IV (2026-10-31) | 159 / 227 | 0-3 | 10 |
| Akali, Rogue Assassin | deck communautaire « Turn 3 Heron into Falling Star(Post Ban) » | riftatlas, modifié le 2026-11-16 | sans tournoi (205 copies) | | 10 |
| Ambessa, Matriarch of War | Jamen | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 221 / 745 | 4-3 | 10 |
| Annie, Dark Child | Syn | Aserty's Creator Invitational (2026-10-10) | 50 / 58 | 1-2 | 10 |
| Azir, Emperor of the Sands | Zefran Castillo | Open the Vault - Riftbound $5k (2026-11-21) | 1 / 42 | 8-1-1 | 10 |
| Darius, Hand of Noxus | deck communautaire « Darius warcamp » | riftatlas, modifié le 2026-11-21 | sans tournoi (18 copies) | | 10 |
| Diana, Scorn of the Moon | David Penaloza | LDXP Sacramento - Riftbound Challenge (2026-10-25) | 3 / 38 | 4-1 | 10 |
| Draven, Glorious Executioner | Sean Barmapov | CCS Sunday $5,000 Riftbound Event #5 (2026-09-20) | 14 / 88 | 5-2 | 10 |
| Ekko, Boy Who Shattered Time | Eric Zhang | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 430 / 745 | 3-4 | 10 |
| Evelynn, Agony's Embrace | Haven Gilbert-Avila | MFG's Riftbound $2k (2026-11-22) | 2 / 28 | 5-2-1 | 10 |
| Ezreal, Prodigal Explorer | Berkun | Convergence IV (2026-10-31) | 6 / 227 | 6-1-1 | 10 |
| Fiora, Grand Duelist | Diego | Open the Vault - Riftbound $5k (2026-11-21) | 9 / 42 | 4-2-1 | 10 |
| Garen, Might of Demacia | Sayonara Ame | Aserty's Creator Invitational (2026-10-10) | 35 / 58 | 1-2 | 5 |
| Irelia, Blade Dancer | David Garza | Open the Vault - Riftbound $5k (2026-11-21) | 3 / 42 | 6-1-2 | 10 |
| Ivern, Green Father | GTPL Skonoms | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 217 / 745 | 4-3 | 10 |
| Jarvan IV, Exemplar of Demacia | Braden Van't Hof | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 77 / 745 | 6-4 | 10 |
| Jax, Grandmaster At Arms | Brandon Ho | CCS $25,000 Showdown Series Event - Riftbound Invitational Qualifie (2026-09-19) | 270 / 563 | 4-4 | 10 |
| Jayce, Defender of Tomorrow | Rakuula | Aserty's Creator Invitational (2026-10-10) | 11 / 58 | 2-1 | 10 |
| Jhin, Virtuoso | Eric Robles | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 332 / 745 | 3-3-1 | 10 |
| Jinx, Loose Cannon | Koko | Aserty's Creator Invitational (2026-10-10) | 5 / 58 | 3-0 | 9 |
| K'Sante, Pride of Nazumah | Saeed Gatson | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 483 / 745 | 2-4-1 | 10 |
| Kai'Sa, Daughter of the Void | Kamryn Luppino-Copeland | MFG's Riftbound $2k (2026-11-22) | 5 / 28 | 4-2 | 10 |
| Kennen, Heart of the Tempest | Vitor Grassato | Riftbound $10,000 Cash Tournament @ Pro-Play Summit: Orlando (2026-09-26) | 26 / 77 | 4-3 | 10 |
| Kha'Zix, Voidreaver | Mingrui Li | CCS $25,000 Showdown Series Event - Riftbound Invitational Qualifie (2026-09-19) | 195 / 563 | 5-4 | 10 |
| LeBlanc, Deceiver | Nick Reak | Open the Vault - Riftbound $5k (2026-11-21) | 19 / 42 | 3-3 | 10 |
| Lee Sin, Blind Monk | deck communautaire « Tempo Lee Sin » | riftatlas, modifié le 2026-11-17 | sans tournoi (21 copies) | | 10 |
| Leona, Radiant Dawn | deck communautaire « [ESP RAD] Leona la mala (En desarrollo 28/10/2026) » | riftatlas, modifié le 2026-11-24 | sans tournoi (16 copies) | | 10 |
| Lillia, Bashful Bloom | Michael Fladung | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 46 / 745 | 8-3 | 10 |
| Lucian, Purifier | Joshua Moises Mares | CCS $25,000 Showdown Series Event - Riftbound Invitational (2026-09-19) | 143 / 563 | 5-3-2 | 10 |
| Lux, Lady of Luminosity | Tingfeng Wan | LDXP Sacramento - Riftbound Challenge (2026-10-25) | 12 / 38 | 3-2 | 10 |
| Master Yi, Wuju Bladesman | Scout | Open the Vault - Riftbound $5k (2026-11-21) | 2 / 42 | 7-1-2 | 10 |
| Master Yi, Wuju Master | Serge DRAUT | Convergence #3 (2026-09-19) | 133 / 217 | 2-3 | 10 |
| Mel, Soul's Reflection | GLORP etani | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 121 / 745 | 6-5 | 10 |
| Miss Fortune, Bounty Hunter | Maddie - Judge_Azooka | Aserty's Creator Invitational (2026-10-10) | 32 / 58 | 1-2 | 8 |
| Mordekaiser, Iron Revenant | Christian Mukai | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 89 / 745 | 7-4 | 10 |
| Nasus, Curator of the Sands | Kenny Tran | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 34 / 745 | 8-3 | 10 |
| Orianna, Lady of Clockwork | Brandon Wolfgang | Convergence IV (2026-10-31) | 56 / 227 | 4-2 | 10 |
| Ornn, Fire Below the Mountain | Anthony brienzo | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 88 / 745 | 7-3 | 10 |
| Poppy, Keeper of the Hammer | Roman-Anthony Thomas Reece | CCS $25,000 Showdown Series Event - Riftbound Invit (2026-09-19) | 280 / 563 | 4-6 | 10 |
| Pyke, Bloodharbor Ripper | Nizmosity | LDXP Sacramento - Nexus Night Constructed (2026-10-24) | 19 / 58 | 2-1 | 10 |
| Rek'sai, Void Burrower | funnyanger | Convergence IV (2026-10-31) | 18 / 227 | 5-1 | 10 |
| Renata Glasc, Chem-Baroness | Frank Gibbons | CCS $25,000 Showdown Series Event - Riftbound Invitational Quali (2026-09-19) | 201 / 563 | 5-5 | 10 |
| Renekton, Butcher of the Sands | era | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 170 / 745 | 4-2-1 | 10 |
| Rengar, Pridestalker | Dwayne | Open the Vault - Riftbound $5k (2026-11-21) | 40 / 42 | 0-5 | 10 |
| Riven, The Exile | CJ Wisniach | MFG's Riftbound $2k (2026-11-22) | 13 / 28 | 3-2 | 10 |
| Rumble, Mechanized Menace | Evan M Nouchi | LDXP Sacramento - Nexus Night Constructed (2026-10-24) | 7 / 58 | 3-0 | 10 |
| Seraphine, Starry-Eyed Songstress | Bradykin | Convergence IV (2026-10-31) | 31 / 227 | 4-2 | 10 |
| Sett, The Boss | Dave Salinas | Open the Vault - Riftbound $5k (2026-11-21) | 6 / 42 | 5-3 | 10 |
| Shen, Eye of Twilight | Garrett Cheng | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 247 / 745 | 4-3 | 10 |
| Sivir, Battle Mistress | DMB0 | Convergence #3 (2026-09-19) | 99 / 217 | 3-2 | 10 |
| Teemo, Swift Scout | Sentried | Aserty's Creator Invitational (2026-10-10) | 54 / 58 | 0-3 | 0 |
| Vex, Gloomist | Michael Chapman | MFG's Riftbound $2k (2026-11-22) | 1 / 28 | 6-0-2 | 10 |
| Vi, Piltover Enforcer | Violetexe | Aserty's Creator Invitational (2026-10-10) | 6 / 58 | 3-0 | 10 |
| Viktor, Herald of the Arcane | Calvin Jin | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 5 / 745 | 8-2-1 | 10 |
| Volibear, Relentless Storm | Quang Nguyen | MFG's Riftbound $2k (2026-11-22) | 10 / 28 | 3-2 | 10 |
| Yasuo, Unforgiven | deck communautaire « Yasuo, Windrider / Yasuo, Unforgiven » | riftatlas, modifié le 2026-11-23 | sans tournoi (96 copies) | | 10 |
| Zed, Master of Shadows | Kevin Tang | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 233 / 745 | 4-3 | 10 |
| Ziggs, Hexplosives Expert | HccDespoiler | LDXP Sacramento - Riftbound Showdown 25k (2026-10-24) | 80 / 745 | 7-4 | 10 |

**Limites**
- Listes faibles faute de mieux (une seule liste légale, ou presque) : Rengar 40e / 42 (0-5), Ahri 159e / 227 (0-3),
  Teemo 54e / 58 (0-3, sans sideboard), Annie 50e / 58, Garen 35e / 58 (sideboard 5), K'Sante 483e / 745, Ekko 430e / 745.
- Légendes dont presque toutes les listes sont devenues illégales (listes légales / listes lues) : Akali 0 / 114, Leona 0 / 10,
  Lee Sin 0 / 2, Rengar 1 / 133, Riven 2 / 217, LeBlanc 3 / 135, Irelia 4 / 122, Kennen 4 / 218, Jayce 5 / 94, Kai'Sa 12 / 142,
  Master Yi Bladesman 9 / 97. Leur deck par défaut n'est pas l'archétype dominant d'avant le ban.
- Aserty's Creator Invitational (10/10, 3 rondes) : scores sur 3 parties seulement (Jinx 3-0, Vi 3-0, Jayce 2-1).
- Les anciennes listes restent dans `raw/` (tests de non-régression, études Akali contre LeBlanc) ; celles qui contiennent
  une carte bannie sont signalées dans `validation.txt` et par « ⚠ » dans l'éditeur.
- Les **plans par légende** (`rust/src/plans_legendes.rs`) ont été écrits pour les anciens decks : 35 plans sur 49 citent au
  moins une carte absente du nouveau deck, et 8 n'ont plus aucune carte « à garder » dans le deck (Fiora, Garen, Irelia,
  Jayce, Miss Fortune, Poppy, Rengar, Teemo : versions Dazzling Aurora ou Stellacorn Herder, bannies). À refaire.
- Les chiffres de `exp_search … defaut` d'avant le 2026-11-24 portent sur les anciens decks par défaut : ne pas les
  regrouper avec de nouveaux.
