---
tags: [riftbound, donnees, reference]
maj: 2026-10-02
---
# Sources et données

## Fichiers du projet (`/mnt/project-files/riftbound/`)
| Chemin | Contenu |
|---|---|
| `README.md` | index et lacunes |
| `rules/core-rules-resume.md`, `rules/tournament-rules-resume.md` | résumés FR (voir [[Règles du jeu]], [[Règles de tournoi]]) |
| `rules/source/core_rules_2026-07-16.txt` | Core Rules complètes, règle par règle |
| `cards/cards_unique.csv` | **938 cartes** (OGN 298, SFD 222, UNL 227, VEN 167, OGS 24), coûts, might, mots-clés, ban, texte avec errata |
| `cards/cards_all_printings.csv/.json` | 1 231 impressions avec URL d'image |
| `cards/build_cards.py` | reconstruction depuis le bulk RiftHunt |
| `decks/raw/*.txt`, `decks.json`, `card_frequency.csv` | 6 listes Akali + 6 LeBlanc (août-sept. 2026), parsées et validées 40/12/3 |
| `decks/matchups/` | analyses du matchup (deux d'entre elles portent un correctif) |
| `engine/` | [[Moteur de règles fidèle]] |
| `sim/` | ancien simulateur, périmé |
| `retex/` | [[Retex étude Akali-LeBlanc]] et extraits d'articles riftbound.gg |
| `tools/retex_fetch.py` | collecte d'articles riftbound.gg via l'API WordPress |

## Sources en ligne
- Core Rules : https://playriftbound.com/en-us/news/rules-and-releases/gameplay-guide-core-rules/
- Tournament Rules : https://playriftbound.com/en-us/news/organizedplay/riftbound-tournament-rules/
- Règles par numéro : https://app.riftjudge.com/rules/core et /rules/tournament
- Mots-clés, FAQ, errata : https://riftwatcher.com/rules/
- Ban list : https://riftbound.zone/en/riftbound-ban-list/
- Cartes (sans auth) : https://api.rifthunt.com/bulk/cards (snapshot du 2026-09-11) et
  https://api.rifthunt.com/cards/search?q=set:ven
- Articles : https://riftbound.gg/wp-json/wp/v2/posts?search=...

## Accès réseau (au 2026-10-02)
- **Accessibles** : api.rifthunt.com, riftbound.gg (texte des articles seulement), app.riftjudge.com.
- **Bloqués** : api.dotgg.gg (winrates et decklists de riftbound.gg), api.riftcodex.com, riftdecks.com,
  riftools.app, mobalytics.gg, piltoverarchive.com, riftmana.com, magicalmeta.ink, hextechanalytics.com.
- Contournement pour les decklists : recherche web nommant joueur + légende + « decklist » ; les quantités
  des résumés peuvent être fausses, d'où la validation 40/12/3. Trois listes restent à vérifier
  (`decks/validation.txt`).
- Les hôtes peuvent être autorisés dans les réglages réseau de l'environnement du projet.
