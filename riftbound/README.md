# Riftbound LAB — données de référence

Collecte du 2026-10-02. Base cartes reconstruite le 2026-10-02 depuis https://api.rifthunt.com/bulk/cards (snapshot du 2026-09-11, source riftcodex.com).

## Contenu
| Fichier | Contenu |
|---|---|
| `rules/core-rules-resume.md` | Résumé FR des Core Rules (2026-07-16) + tableau des mots-clés |
| `rules/tournament-rules-resume.md` | Résumé FR des Tournament Rules (2026-07-16), sanctions juge, ban list |
| `cards/cards_unique.csv` | 938 cartes uniques (1 ligne par nom, version non-variante du set d'origine de préférence) : OGN 298, SFD 222, UNL 227, VEN 167, OGS 24 |
| `cards/cards_all_printings.csv` / `.json` | 1231 impressions (alt art, overnumbered, signature, showcase incluses, avec URL d'image) : OGN 352, SFD 306, UNL 306, VEN 243, OGS 24 |
| `cards/build_cards.py` | Script de reconstruction depuis le bulk RiftHunt |
| `decks/` | Decklists de tournoi parsées et jointes à la base + analyses de matchup (voir `decks/README.md`) |
| `rules/source/core_rules_2026-07-16.txt` | Core Rules complètes, règle par règle (téléchargées depuis app.riftjudge.com) |
| `engine/` | **Moteur de règles fidèle (1v1)** : 85 cartes du matchup Akali/LeBlanc modélisées une par une, 91 tests, runner Monte Carlo (voir `engine/README.md`) |
| `sim/` | Ancien moteur simplifié (périmé, gardé pour l'historique) |

## Colonnes cartes
`id, set, set_code, collector_number, name, type, rarity, domain (sép. "|"), energy_cost, power_cost, might, tags (champion/région/tribu), keywords (extraits du texte entre crochets), is_variant, banned_standard (date), supertype (Champion / Signature / Basic / Token), text, image_url`

- `power_cost` : nombre de runes de domaine à payer. 0 pour une Unit/Spell/Gear sans coût en power, vide pour Legend/Rune/Battlefield.
- `might` : vide pour les sorts (l'ancienne base y mettait une valeur erronée).
- `text` : icônes converties en texte (« 1 energy », « 1 fury rune », « 1 rune of any type », « exhaust », « might »). Les textes viennent de la source à jour et intègrent les errata.
- Noms : suffixes de variante retirés (« (Alternate Art) », « (Overnumbered) », « (Signature) »…). Les Legends portent le nom complet (« Kai'Sa, Daughter of the Void »).

## Sources
- Core Rules : https://playriftbound.com/en-us/news/rules-and-releases/gameplay-guide-core-rules/
- Tournament Rules : https://playriftbound.com/en-us/news/organizedplay/riftbound-tournament-rules/
- Règles navigables : https://app.riftjudge.com/rules/core et /rules/tournament
- Ban list : https://riftbound.zone/en/riftbound-ban-list/ (dernière vague : 2026-09-18)
- Cartes (source actuelle) : https://api.rifthunt.com/bulk/cards (bulk JSON, 1432 impressions, 9 sets). Recherche : https://api.rifthunt.com/cards/search?q=set:ven

## Lacunes connues
1. Promos (OPP, PR, JDG, SGN) exclues ; réimpressions sans incidence gameplay (présentes dans le bulk si besoin).
2. Snapshot bulk du 2026-09-11 : un erratum postérieur ne serait pas reflété. Relancer `build_cards.py` pour rafraîchir.
3. PDF des Core Rules non archivé (>30 Mo) ; utiliser les liens ci-dessus.
4. api.riftcodex.com et api.dotgg.gg restent bloqués par le réseau (403 proxy) ; api.rifthunt.com suffit.
5. Sites de decklists (riftdecks.com, riftbound.gg, riftools.app, mobalytics.gg…) bloqués au 2026-10-02 : seules les recherches web passent.
