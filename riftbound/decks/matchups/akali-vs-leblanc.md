# Akali (Rogue Assassin) vs LeBlanc (Deceiver)

Rédigé le 2026-10-02. Échantillon : 6 listes Akali + 6 listes LeBlanc (tournois août-sept. 2026), voir `../decks.json` et `../card_frequency.csv`.
> **Mise à jour 2026-10-02 (2) : le document de référence est maintenant `akali-vs-leblanc-moteur-fidele.md`** (moteur fidèle, toutes les cartes modélisées).
> **Mise à jour 2026-10-02 :** les simulations (`akali-vs-leblanc-simulations.md`) contredisent 3 points de ce document. Not So Fast et le retrait d'équipement contre Baited Hook n'aident pas. Targon's Peak fait mieux que Void Gate. Defy est la carte à sortir en priorité. Le deck et le plan à jour sont dans ce fichier-là.

Légende : **[données]** = chiffre trouvé en ligne · **[cartes]** = déduit du texte des cartes de la base · **[à tester]** = hypothèse à valider en partie.

## 1. Résultats empiriques

| Source | Résultat | Remarque |
|---|---|---|
| Riftools (agrégat tournois) | **Akali 35,2 %** contre LeBlanc (83 V – 153 D, 236 matchs) | meilleur échantillon trouvé ; cité comme « pire matchup d'Akali par volume » |
| Yomi's Place, RQ Barcelone | LeBlanc ~80 % contre Akali (10 matchs) | petit échantillon, va dans le même sens |
| Méta globale (riftdecks / riftbound.gg, oct. 2026) | Akali ~46 % de winrate global, LeBlanc ~54 % | |
| Riftlite (parties communautaires) | chiffre contradictoire (19 parties) | échantillon trop faible, ignoré |
| Riftdecks « matchup swing » (89 decks LeBlanc vs Akali) | existe, mais illisible via la recherche | page à récupérer quand riftdecks.com sera autorisé |

Les chiffres viennent de résumés de moteur de recherche, pas des pages elles-mêmes (sites bloqués par le réseau). **Conclusion : matchup nettement défavorable, environ 35/65.** Le plan de jeu ci-dessous vise à grappiller ces points.

## 2. Listes types

### Akali, Rogue Assassin (Fury/Calm), cœur à 100 % des 6 listes
1 Akali, Deadly Weapon (champion) · 3 Astral Heron · 3 Stellacorn Herder · 3 Zhonya's Hourglass · 3 Shuriken Flip · 3 Defy · 3 Discipline · 2-3 Falling Star · 2 Back Off · 2-3 Long Sword.
Flex fréquents : Kai'Sa, Survivor x3 (5/6), Scuttle Crab x2-3 (5/6), Mischievous Marai x2 (4/6), Ferrous Forerunner x1-2, Charm x2-3, Mournful Witness, Block, Darius, En Garde.
Runes 6/6 ou 7 Fury/5 Calm. Battlefields : **Void Gate (6/6)**, Forgotten Monument (4/6), Targon's Peak (3/6), Sigil of the Storm (2/6).
Sideboard type : Akali, Silent (5/6), Not So Fast (5/6), Ferrous Forerunner, Irelia, Fervent, Decree of Focus, Crumbling Sands, retrait d'équipement (Disarming Rake, Brittle Steel, Tomb-Raider Barbara, Adaptatron).

### LeBlanc, Deceiver (Mind/Order), liste quasi figée
1 LeBlanc, Fragmented · 3 Karthus, Eternal · 3 Glasc Mixologist · 3 Vi, Peacekeeper · 3 Ruined Rex · 3 Harnessed Dragon · 3 Rift Herald · 3 Baited Hook · 3 Sacrifice · 3 Soaring Scout · 3 Black Rose Dignitary · 2-3 Honest Broker · 2 Watchful Sentry · 1 Thousand-Tailed Watcher · 2 Mirror Image · 1-2 Hidden Blade · 1-2 Deathgrip.
Runes **4 Mind / 8 Order** dans 6/6 listes. Battlefields : **Star Spring + Windswept Hillock (6/6)**, Dusk Rose Lab (4/6), Forbidding Waste (1/6).
Sideboard type : Salvage, Decree of Insight, Decree of Unity, LeBlanc, Everywhere At Once, Thousand-Tailed Watcher, Bellows Breath, Turn to Dust, Safety Inspector.

## 3. Comment LeBlanc gagne [cartes]
- **Moteur Deathknell** : presque toutes ses unités rapportent quelque chose en mourant (rune, carte, or, unité gratuite, 4 dégâts). **Karthus double tous les Deathknell.** Sacrifice, Deathgrip, Baited Hook et Dusk Rose Lab tuent ses propres unités volontairement.
- **Baited Hook** (x3 dans 5/6 listes) : sacrifie une unité et met en jeu gratuitement une unité plus grosse du top 5 (Mixologist 5 → Harnessed Dragon 6 / Rift Herald 7). C'est le moteur de tricherie de coût.
- **Gros finishers** : Harnessed Dragon (tue une unité à l'arrivée), Rift Herald, Thousand-Tailed Watcher (-3 might à toutes tes unités ce tour, peut arriver prêt). Ruined Rex inflige 4 en mourant (8 avec Karthus).
- **Légende** : quand LeBlanc conquiert ou tient, elle crée une copie temporaire d'une autre unité là-bas. Chaque battlefield que tu lui laisses lui donne une unité de plus.
- **Vi, Peacekeeper (Ambush)** : peut tomber en réaction sur un battlefield où elle a déjà des unités, et stun une de tes unités quand elle attaque.

## 4. Plan de jeu Akali

**Idée générale : gagner aux points avant que le moteur ne tourne, en tuant le moins possible.** Chaque unité LeBlanc que tu tues la nourrit. Akali gagne des points en conquérant et en tenant, pas en vidant le board.

1. **Tempo tours 1-4** : poser tôt de la pression qui marque (Scuttle Crab, Marai, Mournful Witness, Kai'Sa avec Accelerate) et prendre les battlefields vides. LeBlanc démarre lentement (Scout, Dignitary, Broker sont des 1-2 might qui veulent mourir).
2. **Bouger plutôt que tuer** : Charm (déplacer une unité ennemie), Back Off (stun), Blitzcrank et la capacité de ta légende (ramener ton unité en base en plein showdown, la redresser si Empowered) gagnent des combats **sans déclencher de Deathknell**. Chaque mouvement d'Akali, Deadly Weapon ou de Stellacorn Herder rapporte aussi un ping ou une pioche.
3. **Les seules cibles à tuer à vue** : **Karthus** (3 might : Shuriken Flip + ping de Deadly Weapon, Falling Star, Marai). Sans Karthus, son deck perd la moitié de sa valeur. Ensuite **Baited Hook** (voir side).
4. **Ne pas tuer Ruined Rex au mauvais moment** : ses 4 dégâts (5 sur Void Gate) partent sur ta meilleure unité. Tue-le quand tu as Zhonya caché, ou quand tes unités sur le board sont peu précieuses.
5. **Garder les réponses pour les gros tours** : Zhonya's Hourglass caché contre Harnessed Dragon / Hidden Blade / Ruined Rex. Defy contre Hidden Blade, Sacrifice ou Deathgrip (il ne contre pas Mirror Image, qui coûte 2 runes, ni les unités).
6. **Respecter Thousand-Tailed Watcher** (7 énergie, ou 8 en Accelerate) : ne pas engager tout ton board dans un combat décisif quand elle a 7+ ressources et des cartes en main. Discipline en réaction (+2) compense en partie le -3.
7. **Respecter Vi en Ambush** : attaquer un battlefield où elle a déjà une unité avec 5+ énergie disponible, c'est s'exposer à un Vi en réaction.
8. **Astral Heron** : ton finisher. Posé sur un battlefield, il réduit le coût de ta carte suivante ; il mange Harnessed Dragon, donc garde Zhonya ou Not So Fast pour le protéger.

### Mulligan [cartes, à tester]
- **Garder** : 2-drops qui marquent (Scuttle Crab, Marai, Mournful Witness), Stellacorn Herder, Kai'Sa, Shuriken Flip, Zhonya's Hourglass.
- **Une seule copie max** : Defy, Discipline, Back Off (réactives, utiles mais pas pour démarrer).
- **Renvoyer** : Astral Heron en double, Sky Splitter, Long Sword sans unité.
- Tu peux mettre de côté jusqu'à 2 cartes ; vise une main qui pose une unité aux tours 1 et 2.

### Choix de battlefield (en BO3 on choisit à chaque partie) [cartes, à tester]
- **Void Gate** : tes sorts et capacités font +1 (Shuriken Flip 3, Falling Star 4+4, Deadly Weapon 2/3), ce qui permet de tuer Karthus et les unités à 5 might. Attention, ça boost aussi le Deathknell de Ruined Rex et Bellows Breath.
- **Forgotten Monument** (personne ne marque avant son 3e tour) : il ralentit surtout ton propre départ. À éviter contre LeBlanc, sauf si tu es en défense.
- **Targon's Peak / Sigil of the Storm** : accélèrent ton développement, ce qui est bon dans une course.
- Ses battlefields : **Windswept Hillock** donne Ganking à tout le monde (tes moves déclenchent Herder et Deadly Weapon, c'est un battlefield qui t'arrange aussi). **Star Spring** : quand tu poses une unité là, tu peux renvoyer une autre unité en base ; c'est un « move » gratuit pour Herder. **Dusk Rose Lab** est pour elle : conteste-le tôt.

### Sideboard (BO3) [cartes, à tester]
**Entrer (≈6-7)** :
- **Not So Fast** x2 : contre l'ordre « tue une unité ennemie » de Harnessed Dragon, Hidden Blade, le Deathknell de Ruined Rex et le stun de Vi (tout ce qui choisit ton unité ou ton équipement).
- **Akali, Silent** : ne peut pas être choisie hors combat, donc immunisée à Dragon, Hidden Blade et Rex hors combat. C'est la menace la plus dure à gérer pour LeBlanc.
- **Retrait d'équipement** contre Baited Hook (x3 dans 5/6 listes) : Disarming Rake, Brittle Steel, Adaptatron ou Tomb-Raider Barbara. Tuer Hook coupe son moteur de tricherie.

**Sortir** : Decree of Focus (ne vise que les unités Fury, inutile ici), Crumbling Sands (elle joue peu de sorts par tour), Long Sword (2-3), Block, En Garde. Garder Charm et Back Off, qui sont tes meilleures cartes du matchup.

**Ce qu'elle va rentrer contre toi** : Salvage / Turn to Dust (contre Zhonya et Long Sword), Bellows Breath (balaye tes unités à 1-2 might, 2 dégâts chacune sur Void Gate), LeBlanc, Everywhere At Once. Ses Decrees (Insight contre Body, Unity contre Chaos) sont morts contre toi et vont sortir.

## 5. Limites et prochaines étapes
- Quantités issues de résumés de recherche : 9/12 listes valident 40/12/3, et les 3 autres sont signalées dans `../validation.txt`. Tous les noms de cartes correspondent à la base.
- Pas encore de données carte par carte du matchup (page « matchup swing » de riftdecks) ni de guide de joueur pro.
- Pour aller plus loin : autoriser riftdecks.com (et riftools.app, riftbound.gg) dans le réseau, puis récupérer toutes les listes top 32 et la page matchup-swing. Noter tes propres parties contre LeBlanc (mulligan, battlefield, résultat) pour valider les points [à tester].
