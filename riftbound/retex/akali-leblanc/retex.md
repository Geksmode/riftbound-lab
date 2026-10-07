# Retex Akali Heron (Gorica) vs LeBlanc — terrain contre simulations

Rédigé le 2026-10-02. Terrain : 16 articles riftbound.gg (20/08 → 01/10/2026), extraits dans `raw/`
(collectés par `riftbound/tools/retex_fetch.py`). Simulations : moteur fidèle, `engine/results_gorica.json`.

## 1. Ce que dit le terrain
| Fait | Source |
|---|---|
| La liste de Gorica **gagne le RQ Singapour** (06/09), en battant Kennen en finale ; l'innovation est Blitzcrank, Impassive | [Singapore RQ](https://riftbound.gg/singapore-regional-qualifier/) |
| Une liste « très proche » **gagne Shenyang** (13/09) | [Shenyang RQ](https://riftbound.gg/shenyang-regional-qualifier/) |
| **Top 8 au RQ Los Angeles** (11-2-1) « avec la même liste qu'à Singapour » | [LA RQ](https://riftbound.gg/los-angeles-regional-qualifier/) |
| Akali a pourtant un **winrate global d'environ 45 %** sur les 4 RQ et une conversion faible (2 sur 72 en top 64 à Shenyang) | Shenyang RQ |
| Akali est **Tier 1** après les bans | [Tier list 24/09](https://riftbound.gg/riftbound-meta-tier-list-best-decks-for-vendetta-post-ban-metagame-heading-to-los-angeles/) |
| LeBlanc passe de Tier 3 à **Tier 1** après les bans : elle gagne la CCS IQ #5 (la liste modélisée dans le moteur) et fait top 16 à LA (11-2) | Tier list 24/09, [CCS IQ #5](https://riftbound.gg/ccs-riftbound-qualifier-5/) |
| La meilleure LeBlanc est une liste proactive autour de **Baited Hook** et du Deathknell | Tier list 24/09 |
| Menaces signalées contre Akali : **Vilemaw's Lair** (battlefield ciblant Akali et Irelia), et Decree of Rage / Decree of Focus dans le miroir | Tier list 24/09, Shenyang RQ |
| Le guide présente Defy, Discipline, Zhonya et Long Sword comme les cartes qui protègent tes unités contre tout le field | [Guide Akali](https://riftbound.gg/akali-rogue-assassin-guide/) |

## 2. Confrontation avec les simulations
| Question | Simulations | Terrain | Verdict |
|---|---|---|---|
| Niveau de la liste Gorica | 44,7 % vs LeBlanc (640 parties neuves) | ~45 % de winrate global, contre tout le field | **Pas comparable** (field contre un seul matchup) ; le seul chiffre terrain du matchup (~35 %, non vérifié) suggère que le moteur est optimiste |
| La liste Gorica est-elle meilleure que Wuhan ? | 40 % contre 33 % sur les mêmes parties | Gorica gagne 2 RQ, Wuhan fait top 8 | **Cohérent** |
| Le gain de G2 (−2 Long Sword, −3 Defy, +5 unités) tient-il contre la LeBlanc la plus récente (LA, 12e) ? | **38,4 % → 48,8 %** (160 parties chacun) | pas de donnée par matchup accessible | **Confirmé** en simulation |
| Faut-il couper Defy et Long Sword partout ? | testé seulement contre LeBlanc | ces cartes sont citées comme clés contre le field | **Non : c'est un plan de side** |

## 3. Ce que ça change pour toi
1. **Garde la liste de Gorica en main deck.** Elle a fait ses preuves sur trois RQ contre tout le field.
   Le gain mesuré vient d'un seul matchup, et Defy reste utile contre les decks à sorts.
2. **Contre LeBlanc, side** : sors 3 Defy et 2 Long Sword, rentre 1 Akali, Silent, 2 Ferrous Forerunner,
   1 Lonely Poro et 1 Scuttle Crab. Si tu n'as pas 5 places au sideboard, fais au moins −2 Long Sword,
   +1 Akali, Silent, +1 Ferrous Forerunner (48 % contre 44 % en simulation).
3. **LeBlanc monte** (Tier 1 post-ban) : ce matchup sera plus fréquent qu'avant. Le plan de jeu reste :
   prendre l'initiative, poser des unités, ne pas garder 2 runes ouvertes pour un contre.
4. **À surveiller** : Vilemaw's Lair en face (tes unités ne peuvent plus revenir en base depuis ce
   battlefield) et les Decrees dans le miroir. Ces cartes ne sont pas encore modélisées.

## 4. Limites
- Les winrates par matchup et les decklists de riftbound.gg sont chargés depuis api.dotgg.gg, bloqué par
  le réseau du projet : seuls les articles sont lisibles. Le site est en « Premium Week » gratuite jusqu'au
  7 octobre (avec connexion), ce qui permettrait de lire le winrate réel Akali-LeBlanc à la main.
- Le sideboard réel de Gorica n'est pas publié dans les articles.
