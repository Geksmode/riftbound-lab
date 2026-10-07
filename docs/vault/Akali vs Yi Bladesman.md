---
tags: [riftbound, matchup, akali, fiche]
maj: 2026-10-04
---
# Akali vs Master Yi, Wuju Bladesman

Fiche de jeu pour [[Akali Heron (Gorica)]]. Liste de référence côté Yi : Ekpilot, 3e du RQ Singapour
(2026-09-05), même tournoi que Gorica. Fichier : `riftbound/decks/raw/yi-bladesman_ekpilot_rq-singapore_3rd.txt`
(récupérée via un résumé de recherche, 40/12/3 validé).

**Statut :** tiré du texte des cartes et des règles, **rien n'est mesuré** (Yi n'est pas modélisé dans le
moteur). Les lignes « hypothèse » sont à tester en partie réelle.

## Ce qu'il fait
- **Légende :** une unité qui défend **seule** gagne +2 de puissance. Rien en attaque, rien si deux de ses unités
  défendent ensemble.
- **Petites unités pas chères** pour tenir les champs de bataille : Lonely Poro 2, Pit Rookie 2, Scuttle Crab 0
  (peut tenir), First Mate 3. Puis Master Yi, Tempered 4 (Hunt 2, au niveau 6 : Deflect et Ganking),
  Rengar 6 (Ambush), Ruin Runner 5 (**impossible à cibler** par tes sorts).
- **Beaucoup de pompes (10 cartes)** qui se cumulent avec la légende.
- **Il voit ta main** : Sabotage (révèle et recycle un non-unité, donc ton Back Off ou ton Defy) et Scuttle Crab
  à sa mort (révèle ta main et tes cartes cachées).

## Ses tours de passe-passe, et ta réponse
| Carte | Coût | Quand | Effet | Defy ? |
|---|---|---|---|---|
| En Garde | 1 énergie | Réaction (n'importe quand) | +1, puis +1 encore s'il est seul. Avec la légende : **+4** | oui |
| Discipline | 2 énergie | Réaction | +2 et pioche | oui |
| **Punch First** | 1 énergie + **2 runes** | Action (son tour **ou en combat**) | **+5** | **non** (2 runes) |
| Alpha Strike | 3 énergie + 1 rune | Action | Son unité inflige sa puissance, répartie sur tes unités | oui |
| Rengar | 5 énergie + 1 rune | Ambush (en réaction) | Unité 6 posée sur un champ de bataille en plein combat | non (unité) |
| Onslaught | 4 énergie | **Son tour seulement** | +6, rejouable du cimetière (Flow 4) | oui |
| Rampage | 3 énergie (+1 Body) | **Son tour seulement** | Combat forcé entre son unité et la tienne | oui |
| Charm | 1 énergie + 1 rune | **Son tour seulement** | Déplace une de tes unités | oui |

Pendant **ton** attaque, il n'a donc que : En Garde, Discipline, Punch First, Alpha Strike, Rengar, Defy
(et Decree of Focus en manches 2-3). Et **avant** le combat (en réponse à ton sort), seulement ses Réactions :
En Garde, Discipline, Defy. Charm, Rampage et Onslaught arrivent à son tour : c'est **à la fin de ton tour**
qu'il faut garder Defy ouvert pour protéger le Héron.

Exemple : son Pit Rookie (2) qui défend seul vaut 4 avec la légende, 8 avec En Garde, 9 avec Punch First.
**Compte toujours son énergie et ses runes Body ouvertes avant d'attaquer.** 1 énergie + 2 Body ouvertes =
Punch First possible, et ton Defy ne l'arrête pas.

## Plan de jeu
1. **Ne pas attaquer un défenseur seul sur ses mana ouvertes.** C'est exactement ce que sa légende punit.
2. **Tuer le défenseur avant le combat.** Ses unités font 0 à 3 de puissance : Shuriken Flip (2), Falling Star
   (3 + 3), Mischievous Marai (2), le ping d'Akali, Deadly Weapon quand elle bouge. Un champ de bataille vide se
   prend sans combat.
3. **Back Off sur le défenseur.** Une unité étourdie ne fait pas de dégâts de combat : ses +2, +4 ou +5 ne
   tuent rien. Ça protège tes attaquants. Attention : s'il reste plus fort que tes dégâts, il survit et tu ne
   prends pas le champ de bataille.
4. **Le Héron ne marche pas jusqu'au champ de bataille, il s'y pose.** Règle 355.2 : une unité se joue dans ta
   base **ou sur un champ de bataille que tu contrôles**. Donc tu prends d'abord un champ de bataille avec une
   unité pas chère, puis tu joues le Héron directement dessus. Tu évites ainsi d'attaquer son défenseur seul
   avec.
5. **Garde Defy ouvert à la fin de ton tour** quand le Héron est en jeu : Charm et Rampage se jouent à son tour, et les deux passent sous Defy. Not So Fast
   contre aussi les deux (ils choisissent ton unité).
6. **Quand c'est lui qui attaque, sa légende ne joue pas.** Tes défenseurs ont Block (+3, Tank) et Discipline.
   Laisse-le venir sur tes champs de bataille plutôt que de foncer sur les siens (hypothèse).
7. **Ne compte pas sur la surprise** après un Sabotage ou la mort d'une Scuttle Crab : il connaît ta main.
8. **Ruin Runner** ne peut pas être ciblé : ni Back Off, ni Falling Star, ni Shuriken Flip. Seul le combat le
   tue (le Héron, 7, le bat sans pompe).

## Manches 2 et 3 : ce qu'il peut rentrer
- **Decree of Focus (×2)** : réaction à 1 énergie, **+4** à son unité en combat contre une de tes unités Fury
  (Akali Deadly Weapon, Kai'Sa, Marai, Noxus Hopeful, Darius) ou ciblée par un de tes sorts Fury (Falling Star,
  Shuriken Flip). Defy le contre. Tes unités Calm (Héron, Mournful Witness, Stellacorn Herder, Blitzcrank) n'y
  sont pas exposées.
- **Disarming Rake (×2)** : tue un équipement en arrivant (Zhonya's, Long Sword, Sterak's).
- **+2 Not So Fast, +2 Rampage.**
- Hypothèse côté Akali : si tu vois Disarming Rake, Long Sword perd de la valeur. Le side de Gorica n'est pas
  publié.

## Ce qui est une erreur, et ce qui ne l'est pas
- **Erreurs** : attaquer un défenseur seul en face de 1 énergie + 2 Body ouvertes sans Back Off ; faire marcher
  le Héron vers un champ de bataille tenu au lieu de le poser sur le tien ; poser le Héron sans Defy ouvert
  quand il a de quoi jouer Charm ou Rampage.
- **Matchup** : une légende qui donne +2 à chaque défense, plus 10 pompes. Une partie de tes combats perdus
  n'était pas jouable autrement.

## À faire
- Noter les vraies parties contre Yi dans le [[Carnet de parties]].
- Modéliser Yi dans le [[Moteur de règles fidèle]] pour mesurer le matchup et tester le plan Héron.
