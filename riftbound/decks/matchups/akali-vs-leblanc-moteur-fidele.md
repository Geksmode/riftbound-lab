# Akali vs LeBlanc : résultats du moteur fidèle

> **Correctif du 2026-10-02 (retex)** : tous les jobs utilisent les mêmes graines 0-159, donc les
> agrégats (« 4 480 parties », initiative « 14 points ») surestiment la précision. L'effet de l'initiative
> est de l'ordre de 5 à 10 points. La « LeBlanc sidée » est une hypothèse construite. Voir
> `../../retex/retex-etude-akali-leblanc.md`.

Rédigé le 2026-10-02. Moteur : `riftbound/engine/` (voir `engine/README.md`). Résultats bruts :
`engine/results.json`. **Ce fichier remplace `akali-vs-leblanc-simulations.md`**, qui venait du moteur
simplifié rejeté.

## En bref
- 4 480 parties simulées, 28 configurations, sur un moteur qui applique les règles officielles du 16/07/2026
  et modélise les 85 cartes du matchup une par une (91 tests de scénario au vert).
- La liste Akali « Wuhan top 8 » telle quelle gagne **31,6 %** (320 parties). Les listes modifiées
  (−Defy, +unités) gagnent **39,4 %** (1 920 parties) : **+8 points, l'écart le plus net de toute l'étude**.
- Deux autres écarts sont significatifs : **commencer la partie** (46,4 % contre 32,1 % en second, sur
  2 080 parties chacun) et **le battlefield que LeBlanc présente** (Star Spring 48,8 % pour toi, Windswept
  Hillock 30,6 %).
- Tout le reste (quel battlefield tu présentes, le mulligan, Not So Fast, le retrait d'équipement) tient dans
  le bruit à ±4 points : ce sont des choix libres, pas des leviers.

## 1. Deck building (160 parties par ligne, mêmes graines pour toutes les listes)

| Liste | Winrate Akali |
|---|---|
| Stock (Wuhan, 东东卡牌妙妙屋 5e) | 33,1 % |
| C1 : −2 Defy, +2 Mournful Witness | 31,9 % |
| **C3 : C1 −2 Long Sword, +1 Akali, Silent, +1 Ferrous Forerunner** | **45,0 %** |
| C5 : C3 −1 Defy, +1 Lonely Poro | 40,0 % |
| C5 +1 Not So Fast (−1 En Garde) | 40,6 % |
| C5 +1 Brittle Steel (−1 Adaptatron) | 36,2 % |
| C5 +2e Akali, Silent (−1 Astral Heron) | 36,2 % |

Agrégats (plus fiables que chaque ligne) :
- **stock : 31,6 %** sur 320 parties ;
- **C3 : 39,4 %** sur 800 parties ; **C5 : 39,5 %** sur 1 120 parties.

Ce qu'on peut en conclure :
1. **Sortir Defy et les Long Sword pour des unités est le vrai gain.** Defy ne contre que 6-7 sorts de
   LeBlanc, et garder 2 runes ouvertes coûte un tour de développement. Les Long Sword ne répondent à rien.
2. **C3 et C5 sont équivalents** : le 3e Defy enlevé pour un Lonely Poro ne change rien de mesurable.
   Prends celle dont tu as les cartes.
3. **Le retrait d'équipement contre Baited Hook n'aide pas** (−4 pts, non significatif mais jamais positif,
   comme dans l'ancienne étude) : la carte est morte quand Hook n'est pas en jeu, et LeBlanc en a 3.
4. **Not So Fast est neutre** (+0,6 pt). L'ancienne étude le disait négatif ; avec la chaîne correctement
   implémentée il n'est ni bon ni mauvais. C'est une carte de confort, pas une solution.
5. Un **2e Akali, Silent** à la place d'Astral Heron est moins bon : il te manque un finisher.

## 2. Battlefields

| Battlefield présenté par Akali (liste C5) | Winrate |
|---|---|
| Forgotten Monument | 46,9 % |
| Void Gate | 41,2 % |
| Targon's Peak | 38,8 % |
| Sigil of the Storm | 38,1 % |

Mais en recontrôlant avec la liste C3 : Forgotten Monument 37,5 % et Void Gate 36,2 %. **Les quatre options
sont indiscernables** une fois le bruit pris en compte : choisis celui que tu préfères, l'ancienne
recommandation « Targon's Peak plutôt que Void Gate » n'est pas confirmée.

| Battlefield présenté par LeBlanc | Winrate Akali |
|---|---|
| Star Spring | 48,8 % |
| Dusk Rose Lab | 44,4 % |
| **Windswept Hillock** | **30,6 %** |

**Windswept Hillock est son meilleur choix contre toi** (18 points d'écart avec Star Spring, bien au-delà du
bruit) : le Ganking lui permet de déplacer ses grosses unités d'un battlefield à l'autre et de reprendre
celui que tu tiens sans repasser par la base. En BO3, si elle l'a déjà présenté dans une partie gagnée, elle
ne pourra pas le rejouer : c'est ta meilleure partie.

## 3. Plan de jeu

| Choix | Winrate (liste C5) |
|---|---|
| Plan de référence | 40,0 % |
| Mulligan agressif (garder unités ≤ 4 énergie + Shuriken Flip) | 42,5 % |
| Ne jamais mulligan | 43,8 % |
| Éviter d'attaquer là où Vi peut tomber en Ambush | 45,0 % |
| **Akali commence** | **46,4 %** (2 080 parties, toutes configurations) |
| **Akali joue en second** | **32,1 %** (2 080 parties) |

- **L'initiative vaut 14 points.** C'est le facteur le plus lourd du matchup après la liste. Quand tu
  commences, tu prends un battlefield avant que son moteur ne tourne ; en second, tu cours derrière.
- **Le mulligan n'est pas un levier** : les trois politiques tiennent dans ±4 points. Garde une main qui pose
  une unité aux tours 1-2, sans te forcer à renvoyer des cartes.
- **Jouer autour de Vi ne coûte rien** (+5 pts, non significatif). L'ancienne étude affirmait −9 points :
  c'était un artefact du moteur simplifié, qui n'avait pas de fenêtre de réaction correcte. Tu peux être
  prudent quand elle a 5 runes prêtes et une carte en main.

## 4. Sideboard (parties 2 et 3)

| | vs LeBlanc de base | vs LeBlanc sidée |
|---|---|---|
| Stock | 33,1 % | 28,1 % |
| C5 | 40,0 % | 35,0 % |
| C3 | 45,0 % | 38,8 % |

La LeBlanc sidée (+ LeBlanc, Everywhere At Once, + 2e Thousand-Tailed Watcher, − 1 Deathgrip, − 1 Hidden
Blade) te coûte 5 à 6 points. Les 5 cartes à rentrer restent les mêmes : **2 Mournful Witness, 1 Akali,
Silent, 1 Ferrous Forerunner, 1 Lonely Poro**, en sortant **3 Defy et 2 Long Sword**.

## 5. Deck recommandé (40 cartes, liste C3/C5)

**Unités (22)** : 1 Akali, Deadly Weapon (champion) · 3 Scuttle Crab · 2 Mischievous Marai ·
2 Mournful Witness · 3 Kai'Sa, Survivor · 3 Stellacorn Herder · 1 Akali, Silent · 1 Adaptatron ·
2 Ferrous Forerunner · 3 Astral Heron · 1 Lonely Poro (version C5)
**Équipement (3)** : 3 Zhonya's Hourglass
**Sorts (15)** : 3 Shuriken Flip · 3 Charm · 3 Discipline · 3 Falling Star · 2 Back Off · 1 En Garde
**Runes** : 6 Fury / 6 Calm · **Battlefields** : au choix (Void Gate, Forgotten Monument, Targon's Peak)

Différence avec la liste Wuhan : **−3 Defy, −2 Long Sword ; +2 Mournful Witness, +1 Akali, Silent,
+1 Ferrous Forerunner, +1 Lonely Poro.**

## 6. Deux parties commentables
`engine/exemple_partie.txt` contient deux parties complètes tour par tour (liste C3 contre la LeBlanc de
l'IQ #5 sur Windswept Hillock), avec chaque carte jouée, chaque combat et chaque mort : utile pour vérifier
que le moteur joue comme toi, et pour repérer les séquences que tu ferais autrement.

## 7. Limites
- **L'IA** est une recherche à un coup avec déroulement par politique simple sur deux tours. Elle ne bluffe
  pas et ne monte pas de combinaison à plusieurs tours. Le niveau absolu (31,6 % pour la liste de base,
  contre ~35 % dans les données réelles) est crédible, mais les écarts entre options restent plus fiables
  que les niveaux.
- **160 parties par configuration** donnent ±4 points d'écart-type : seuls les écarts de 8 points ou plus
  sont concluants. Les tableaux ci-dessus signalent explicitement ce qui est dans le bruit.
- Les bonus de might des équipements et le texte de Pendulum Blade ne sont pas dans la base de cartes ; ils
  viennent de la lecture des cartes (Long Sword +2, Sterak's Gage +3, Pendulum Blade +1). Zhonya's Hourglass
  utilise le texte de l'errata.
- Note tes vraies parties (qui commence, battlefield présenté par chacun, main de départ, score) : c'est ce
  qui permettra de recaler le modèle et de trancher les écarts restés dans le bruit.
