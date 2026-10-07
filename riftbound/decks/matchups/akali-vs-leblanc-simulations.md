# Akali vs LeBlanc : simulations (moteur simplifié — PÉRIMÉ)

> **Périmé depuis le 2026-10-02.** Ce document vient du moteur simplifié, remplacé par le moteur fidèle
> (`akali-vs-leblanc-moteur-fidele.md`, règles complètes et 85 cartes modélisées). Trois conclusions d'ici
> ne tiennent plus : Targon's Peak n'est pas meilleur que Void Gate, le mulligan agressif n'est pas un levier,
> et jouer autour de Vi ne coûte pas 9 points. Gardé pour l'historique.


Rédigé le 2026-10-02. Simulateur et scripts : `riftbound/sim/` (voir `sim/README.md`). Résultats bruts : `sim/results.json`, `sim/swaps.json`, `sim/final_results.json`. Exemples de parties commentables : `sim/logs/`.

## En bref
- Dans le simulateur, la liste Akali « Wuhan top 8 » gagne **31-32 %** contre la liste LeBlanc gagnante de l'IQ #5. Le chiffre réel trouvé en ligne est de 35 %, donc le modèle est dans le bon ordre de grandeur (l'IA joue un peu moins bien qu'un humain).
- La meilleure combinaison trouvée monte à **47 % contre la liste LeBlanc de base et 41 % contre une LeBlanc qui a sidé contre nous**, sur 4 000 parties chacune :
  1. plus d'unités pas chères et moins de cartes réactives (−3 Defy, −2 Long Sword) ;
  2. **Targon's Peak** au lieu de Void Gate ;
  3. un mulligan agressif pour avoir des unités à 4 énergie ou moins.
- Les chiffres sont des **ordres de grandeur pour comparer des choix**, pas des winrates à prendre au pied de la lettre (voir « Limites »).

## 1. Méthode
- Moteur simplifié écrit pour ce matchup. Il gère les runes (épuiser = 1 énergie, recycler = 1 power), 2 battlefields, la conquête et la tenue jusqu'à 8 points (avec la règle du dernier point), le combat à la might, les Deathknell doublés par Karthus, Baited Hook, la copie de la légende LeBlanc, la retraite de la légende Akali, Zhonya, les counters, les stuns et les effets temporaires. Coûts et might viennent de la base de cartes.
- Les deux decks sont joués par des heuristiques : déployer, dégager les bloqueurs, attaquer quand c'est gagnant, conquérir les battlefields vides, défendre.
- Les listes viennent des vraies decklists : Akali = 东东卡牌妙妙屋 (Wuhan, 5e), LeBlanc = GYATarina (IQ #5, 1re). La « LeBlanc sidée » remplace 1 Deathgrip et 1 Hidden Blade par LeBlanc, Everywhere At Once et un 2e Thousand-Tailed Watcher.
- Environ **150 configurations** testées, de 800 à 4 000 parties chacune, avec les mêmes mélanges pour chaque build afin de réduire le bruit. Écart-type : ±1,6 pt à 800 parties, ±0,7 pt à 4 000.

## 2. Résultats : deck building

| Build (Void Gate, plan anti-LeBlanc) | vs LeBlanc | vs LeBlanc sidée |
|---|---|---|
| Stock (Wuhan) | 30,8 % | 26,5 % |
| C1 : −2 Defy, +2 Mournful Witness | 36,6 % | 29,8 % |
| C2 : C1, −1 Long Sword, +1 Akali, Silent | 37,6 % | 28,7 % |
| C3 : C2, −1 Long Sword, +1 Ferrous Forerunner | 38,6 % | 30,3 % |
| C4 : C3, −1 En Garde, +1 Not So Fast | 33,3 % | 26,7 % |
| **C5 : C3, −1 Defy, +1 Lonely Poro** | **40,0 %** | **33,4 %** |
| C6 : C3, −1 Charm, +1 Not So Fast | 35,6 % | 30,1 % |

Ce que montrent les échanges carte par carte (`sim/swaps.json`, 800 parties chacun) :
- **Defy est la carte la plus faible du matchup.** LeBlanc n'a que 6 à 7 sorts qu'il peut contrer (Sacrifice, Hidden Blade, Deathgrip). Contrer Sacrifice ne sauve rien, car l'unité est tuée en coût. Mirror Image coûte 2 runes et passe. Garder 2 runes ouvertes pour Defy coûte un tempo qu'Akali n'a pas.
- **Les unités pas chères gagnent** (Mournful Witness, Lonely Poro, Marai) : elles conquièrent tôt et occupent les battlefields.
- **Akali, Silent et Ferrous Forerunner** sont les meilleures menaces ajoutées : Silent échappe à Dragon, Hidden Blade et Rex hors combat ; Forerunner laisse 2 Mechs 3/3 en mourant.
- **Contredit ma première analyse** : Not So Fast est neutre à légèrement négatif (il faut garder 3 runes) et le retrait d'équipement contre Baited Hook (Brittle Steel, Disarming Rake) est négatif (−1 à −4 pts), parce que la carte est morte quand Hook n'est pas en jeu.

## 3. Résultats : plan de jeu (build C5)

| Choix | Winrate Akali | Écart |
|---|---|---|
| Battlefield Void Gate (référence) | 40,0 % | |
| **Battlefield Targon's Peak** | **46,0 %** | +6 |
| Battlefield Threshold of the Gray | 46,1 % | +6 (effet peut-être surestimé, voir limites) |
| Battlefield Forgotten Monument | 40,2 % | 0 |
| Battlefield Back-Alley Bar | 39,0 % | −1 |
| LeBlanc choisit Windswept Hillock | 33,9 % | −6 (son meilleur battlefield contre toi) |
| LeBlanc choisit Dusk Rose Lab / Star Spring | 42,8 % / 42,9 % | |
| **Mulligan agressif** (garder unités ≤ 4 énergie et Shuriken Flip) | 42,2 % | +2 |
| Attaquer prudemment par peur de Vi (Ambush) | 31,0 % | **−9** |
| Plan naïf « tuer tout ce qui bouge » | 37,1 % | **−3** |
| Ne pas empower la légende Akali | 40,8 % | ≈ 0 |
| Akali joue en premier / en second | 44,2 % / 35,8 % | 8 pts d'écart |

Combinaison finale (4 000 parties, autres mélanges) :

| | Void Gate, mulligan normal | Targon's Peak, mulligan agressif |
|---|---|---|
| Stock vs LeBlanc | 32,3 % | 35,2 % |
| **C5 vs LeBlanc** | 39,3 % | **47,5 %** |
| C5 vs LeBlanc sidée | 32,6 % | **41,0 %** |

## 4. Deck recommandé contre LeBlanc (40 cartes)

**Unités (22)** : 1 Akali, Deadly Weapon (champion) · 3 Scuttle Crab · 2 Mischievous Marai · 2 Mournful Witness · 1 Lonely Poro · 3 Kai'Sa, Survivor · 3 Stellacorn Herder · 1 Akali, Silent · 1 Adaptatron · 2 Ferrous Forerunner · 3 Astral Heron
**Équipement (3)** : 3 Zhonya's Hourglass
**Sorts (15)** : 3 Shuriken Flip · 3 Charm · 3 Discipline · 3 Falling Star · 2 Back Off · 1 En Garde
**Runes** : 6 Fury, 6 Calm · **Battlefields** : Targon's Peak, Threshold of the Gray, Void Gate

Différence avec la liste Wuhan : **−3 Defy, −2 Long Sword ; +2 Mournful Witness, +1 Lonely Poro, +1 Akali, Silent, +1 Ferrous Forerunner.**
Si tu gardes ta liste habituelle en partie 1, ces 5 cartes forment le cœur du sideboard pour les parties 2 et 3 (5 slots sur 10).

## 5. Plan de jeu (ce que les simulations confirment)
1. **Mulligan** : garde une main avec 2 unités à 4 énergie ou moins. Renvoie Astral Heron, Forerunner et les réactions en trop. Ça vaut +2 pts.
2. **Pose et conquiers dès le tour 1-2.** LeBlanc rampe vite grâce à Scout et Dignitary (dans une partie simulée, elle pose Harnessed Dragon dès son 3e tour grâce aux runes de ses Deathknell). Chaque point pris tôt compte.
3. **Attaque sans avoir peur de Vi.** Jouer autour de l'Ambush coûte 9 points de winrate. Garde Back Off ou Discipline pour répondre, mais engage.
4. **Ne tue que ce qui bloque une conquête, et Karthus.** Pour dégager un défenseur, Charm passe d'abord (il ne déclenche pas de Deathknell), puis Shuriken Flip, puis Falling Star. Le plan « tuer tout » coûte 3 points.
5. **Tiens les deux battlefields.** Dans les parties gagnées, Akali finit souvent en tenant les 2 battlefields (2 points par tour). Chaque hold de LeBlanc lui donne aussi une copie gratuite d'une unité.
6. **Zhonya** se garde pour Kai'Sa, Herder, Heron ou Silent (les unités à 4+ de valeur).
7. **Battlefield** : Targon's Peak est le meilleur choix en BO3. Les 2 runes redressées en fin de tour te laissent du mana pour Back Off ou Discipline pendant son tour.

## 6. Exemples de parties simulées
Les fichiers sont dans `sim/logs/`, tour par tour, avec mains, combats et morts.
- `partie_4_Akali.txt` (victoire 8-2) : Falling Star sur LeBlanc Everywhere At Once, Charm pour conquérir, puis Falling Star sur Karthus et sa copie. Akali tient les deux battlefields à partir du tour 10 et finit tour 12.
- `partie_1_LeBlanc.txt` (défaite 6-8) : Akali mène 4-2, puis Watcher (−3 à toutes tes unités) et Rift Herald reprennent Dusk Rose Lab. Dragon et Hidden Blade tuent les défenseurs. LeBlanc gagne en tenant.
- `partie_0_Akali.txt`, `partie_2_LeBlanc.txt` : une autre victoire et une autre défaite.

## 7. Limites (à lire avant de croire un chiffre)
- **IA heuristique** : elle ne bluffe pas, ne lit pas la main adverse et séquence moins bien qu'un bon joueur. Les écarts entre options sont plus fiables que les niveaux absolus.
- **Simplifications** :
  - un seul aller-retour de réactions par combat ;
  - équipement Long Sword à +2 might (valeur supposée, l'icône n'est pas dans la base) ;
  - pas de Hidden de Marai en défense dans la plupart des cas ;
  - la réduction de coût d'Astral Heron n'est pas modélisée ;
  - Star Spring et Sigil of the Storm n'ont pas d'effet ;
  - l'énergie de Threshold of the Gray est gardée jusqu'à la fin du tour, ce qui la surestime probablement ;
  - le joueur qui commence en second canalise 3 runes à son premier tour (supposé).
- **Le meilleur deck est optimisé contre ce simulateur.** Il faut le confirmer en vraies parties. Note tes résultats (main de départ, battlefield, qui commence, score) pour qu'on recale le modèle.
