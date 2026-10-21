---
titre: Extension Radiance (RAD)
resume: les 167 cartes de Radiance dans la base et le moteur, les nouveaux mots-clés, les corrections de la source, ce qui manque
maj: 2026-10-21
sources: riftbound/engine/cardsets/radiance.py, riftbound/engine/cardsets/test_radiance.py, riftbound/cards/corrections_rad.py, riftbound/rules/source/core_rules_2026-10-09.txt, riftbound/engine/cardsets/NEEDS_radiance.md
---
**Ajoutée le 2026-10-19** (demande de l'utilisateur : « toutes les cartes de Radiance et leur effet »). Sortie de
l'extension : 2026-10-23.

**Base de cartes**
- Source : `api.rifthunt.com/bulk/cards` (riftcodex, relevé du 2026-10-15), set `RAD` ajouté à `cards/build_cards.py`.
  167 cartes uniques : 84 unités, 38 sorts, 26 équipements, 10 champs de bataille, 9 légendes (+ runes, déjà là).
- La source est fausse pour beaucoup de cartes (pré-sortie) : **108 cartes corrigées** d'après les images officielles
  (couleur du cadre, bandeau des tags) dans `cards/corrections_rad.py` : domaines (ex. Amateur Demolitionist est
  Fury, pas Chaos ; Supernova est Mind), tags de région, tag interne « Device » retiré (imprimé nulle part), Ziggs,
  Explosive est un champion, nom « Neeko, Blending In » (« Bleeding In » dans la source), coût de Runeblade Anvil
  ([2][Fury], pas Chaos), Nazuman Ntofo et Protector's Shield sont des Equipment. Coûts et Might : conformes.
- Bonus de Might et texte d'effet des Equipment lus sur les images : Nazuman Ntofo +0 et [Shield 2], Protector's
  Shield +1 et [Tank], Blade of the Exile +3 et [Ganking].
- Images de la table : planches `train/atlas/a16.jpg` à `a18.jpg` (167 vignettes, `atlas/index.json`).

**Règles** : Core Rules du 2026-10-09 enregistrées (`rules/source/core_rules_2026-10-09.txt`). Le moteur reste écrit
d'après celles du 2026-07-16 ; seules les règles nouvelles utiles à Radiance sont branchées :
- [Deploy] (830) : équipement joué seulement sur un champ de bataille (n'importe lequel), pas rappelé au cleanup,
  détruit quand un adversaire y tient ; peut être déplacé.
  Affichage (2026-10-19) : l'instantané `replay.snap` met ces équipements dans `bfs[i].g` (`u`, `n`, `c`, `x`) au
  lieu de la base ; la table (`train/src.html`, `bfHTML`) et le lecteur de replays les dessinent sur leur battlefield,
  du côté de leur contrôleur, après les unités (touchables comme cibles). Avant, ils apparaissaient dans la base.
- [Disarm X] (831) : déclencheur d'attaque, -X sur une unité ennemie ici ; valeurs additionnées.
- [Show Off] (832) : montrer une carte de la main ou un objet allié en jouant ; l'information est prise au moment
  où on la montre (832.1.d.1).
- Jeton Bomb (187.12) ; « devient épuisé » (évènement `exhausted`, une action = un évènement) ; « unit gear »
  (Orianna, Lady of Clockwork) ; jouer une carte depuis l'exil jusqu'à la prochaine Ending Phase.
- Les autres changements des règles 2026-10-09 (cleanup, Contested, « once each turn »…) ne sont **pas** audités
  dans le moteur.

**Modélisation** : 166 cartes sur 167 dans `cards.IMPL` (`cardsets/radiance.py`). Choix de lecture :
- Forbidden Technique : « Banish this » est une instruction du sort, il est banni à chaque résolution.
- « The next unit you play » (Ekko, Boy Who Shattered Time ; Last Caress ; Firelights' Hideout) : les jetons ne
  comptent pas (ce ne sont pas des cartes jouées de la main).
- Seraphine, Inspiring : « trois unités épuisées en même temps » = par la même action (un déplacement, un coût).
- Mechanical Bandit : à égalité de coût le plus bas, on pioche.
- Runeblade Anvil (et Akali, Brash) : la carte bannie se joue en payant son coût jusqu'à **ta** prochaine Ending
  Phase, même si l'enclume quitte le plateau au tour adverse ; deux enclumes = deux permissions séparées (427.3.a) ;
  deck vide = rien de banni. Sur la table : pile « Bann. » (Banishment publique, 108.6.e) et carte BANNIE au bout de
  la main (2026-10-21, robot `train/verif_banish.mjs`).

**Bloquée** : Neeko, Blending In (plusieurs légendes en jeu et choix à la construction du deck ; le moteur n'a
qu'une légende par joueur) : `cardsets/NEEDS_radiance.md`.

**Vérifié** (2026-10-19) : `cardsets/test_radiance.py` 181/181 (au moins un test par carte, plus les mots-clés) ;
`fuzz_cards.py 300 --module radiance` : 0 exception (144 cartes jouées au moins une fois). Robot `train/verif_deploy.mjs` (vérifié par robot, glisser à la souris et toucher simulé ;
pas essayé sur un vrai téléphone) : Pillaged Armory glissé sur un battlefield, affiché là, partie jouée jusqu'au bout.
Pas de partie jouée à la main avec les autres cartes ; l'IA n'a pas de plan pour les nouvelles légendes.
