---
tags: [riftbound, simulation, retex]
created: 2026-10-03
maj: 2026-10-03
---
# Agent manager

Boucle automatique d'apprentissage sur [[Akali Heron (Gorica)]] et ses matchups. Toutes les 6 heures, une
session relit ce qui a été appris, joue de nouvelles simulations sur des graines jamais utilisées, écrit un
retex autocritique et met à jour ce vault.

- Fichiers : `riftbound/manager/` (protocole, journal `learnings.md`, retex dans `sessions/`, résultats bruts).
- Méthode : celle de [[Méthode statistique]] (graines neuves, comparaisons appariées, « on ne sait pas » sous
  2 écarts-types).
- Ce que ça ne fait pas : modifier les règles du moteur ou ajouter des cartes ([[Moteur de règles fidèle]]).
  Ces besoins remontent dans [[Questions ouvertes]].

## Sessions
| Session | Date | Ce qu'on a appris |
|---|---|---|
| 1 | 2026-10-03 | Windswept Hillock coûte ~8 pts à Akali quand elle commence (−8,0 ± 3,3). Avec plans, G2 fait 59,5 % mais c'est flatté par le plan LeBlanc : ~52 % si LeBlanc présente toujours Windswept. G2 contre stock sous plans : +5,0 ± 3,3, on ne sait pas encore. |
| 2 | 2026-10-03 | Contre un LeBlanc qui présente toujours Windswept : ~47 % ± 4. L'avantage de G2 sur la liste stock n'y apparaît plus (−3,0 ± 3,3) : on ne sait pas si le side vaut le coup. Un bloc de 400 parties peut dévier de ~6 pts. Sigil ≥ Void Gate, sans preuve. |
| 3 | 2026-10-03 | **Nouvelle IA « tempo »** : niveaux non comparables aux sessions 1-2. G2 contre stock toujours « on ne sait pas » (+3,8 ± 3,3 face à Windswept). Piste : c'est la moitié Long Sword qui compte (GL − GD +8,3 ± 3,3, GL − stock +6,3), à confirmer en session 4. Windswept coûte ~4 pts. |
| 4 | 2026-10-04 | **LeBlanc « réel »** (garde Reflet et unité copiée) : niveaux non comparables. La piste GL ne se confirme pas (GL − stock −2,0 ± 3,4) : stock, G2 et GL sont indiscernables contre un LeBlanc qui suit un plan. Windswept +2 à +3 pts, dans le bruit. |
| 5 | 2026-10-04 | Même IA que s4, blocs cumulés : G2 − stock **−0,3 ± 2,3** sur 800 parties (au mieux un petit écart). GL − stock +1,6 ± 2,4, on ne sait pas. Piste revenue : couper les Long Sword vaut mieux que couper les Defy (GL − GD +7,5 ± 3,4), à confirmer en s6. |
| 6 | 2026-10-04 | Question fixée à l'avance : GL − GD **−6,8 ± 3,3**, la piste « couper Long Sword plutôt que Defy » est réfutée. **Acquis** : stock, G2, GL et GD se valent à ± 4 pts (G2 − stock +0,4 ± 1,9 sur 1 200 parties). On passe aux questions de jeu (plans, Windswept). |
| 7 | 2026-10-04 | **Plan de jeu** (apparié) : le plan Gorica vaut **+7,0 ± 3,1** pts contre aucun plan ; jouer agressif coûte **6,0 ± 3,0** pts face à Windswept. Windswept cumulé +2,5 ± 1,6 (on ne sait pas). À confirmer sur bloc neuf en s8. |
| 8 | 2026-10-05 | **Trois acquis** (questions fixées, cumuls) : plan Gorica **+7,7 ± 2,3** contre aucun plan ; jouer agressif **−6,1 ± 2,1** ; Windswept présenté par LeBlanc **−4,3 ± 1,3**. Bien jouer son plan compte plus que le side. |
| 9 | 2026-10-05 | **D'où vient le gain du plan** : présenter Sigil seul (sans plan) vaut +3,2 ± 2,6 (on ne sait pas) ; à Sigil égal, le plan vaut encore **+11,0 ± 3,3**. Le plan gagne surtout par ses décisions en partie. Plan cumulé sur trois blocs : +9,8 ± 1,9. |
| 10 | 2026-10-05 | **À battlefield égal, suivre le plan vaut ~11 pts** (cumul s9-s10 : +10,7 ± 2,3) : acquis. Piste : avec le plan, présenter Forgotten Monument au lieu de Sigil coûte **10,2 ± 3,4** pts — à confirmer en s11 en même temps que Void Gate. |
| 11 | 2026-10-05 | Question fixée : face à Windswept, **présenter Void Gate au lieu de Sigil coûte ~9 pts** (−8,8 ± 3,3, piste forte, à rejouer en s12). La piste Forgotten Monument de s10 retombe sur bloc neuf (−1,8 ± 3,3) : on ne sait pas. Garder Sigil. |
| 12 | 2026-10-06 | **Acquis : contre Windswept, présente Sigil.** Void Gate coûte ~8 pts (−7,9 ± 2,3) et Forgotten Monument ~6 pts (−5,7 ± 2,3) sur deux blocs neufs. Face à Star Spring, Void Gate et Sigil se valent (−1,0 ± 4,7). En s11 j'avais trop vite déclaré la piste Forgotten Monument surévaluée. |
| 13 | 2026-10-06 | Forgotten Monument confirmé à ~6 pts sur trois blocs (−5,6 ± 1,9). Face à Star Spring, garder Void Gate (Sigil −2,1 ± 3,3). Contre LeBlanc **Deathknell**, Akali fait 46,2 %, le même niveau que contre Hook tempo. La dispersion entre blocs est redevenue normale (niveau de référence ~48 %). |
| 14 | 2026-10-06 | Contre LeBlanc Deathknell : le plan de LeBlanc ne change rien (Hook tempo − Deathknell −0,5 ± 2,4), le side G2 non plus (+0,8 ± 3,3), jouer agressif −3,0 ± 3,2 (on ne sait pas). **Commencer vaut ~9 pts** (+9,0 ± 2,2 sur sept blocs). Rendement décroissant : le matchup est bien décrit par quelques facteurs. |
| 15 | 2026-10-06 | Contre Deathknell, le plan Gorica vaut +11,8 ± 3,3 : contre les deux plans LeBlanc, **le plan vaut ~10 pts** (+10,3 ± 1,6) et **jouer agressif coûte ~5 pts** (−5,1 ± 1,5). Dernière session avant le changement de règles du moteur (rune recyclée, énergie flottante) : la session 16 remesure tout. |
| 16 | 2026-10-07 | **Nouveau moteur** (règles de rune) : les acquis tiennent. Plan +12,5 ± 3,4, Void Gate −9,0 ± 3,2, agressif −5,5 ± 3,3, niveau 47 %. Windswept et commencer plus petits sur ce bloc, à confirmer. Conteneur redémarré en cours de session : reprise sur les mêmes graines. |
| 17 | 2026-10-07 | Deuxième bloc du nouveau moteur : **agressif −7,3 ± 2,2 et Void Gate −9,3 ± 2,2 acquis** avec les nouvelles règles. Windswept +2,9 ± 1,6 et commencer +4,0 ± 3,5 : on ne sait pas encore. Niveau 48 %. |
