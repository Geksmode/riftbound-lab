---
tags: [riftbound, simulation, outil]
dossier: riftbound/engine/
maj: 2026-10-02
---
# Moteur de règles fidèle

Simulateur 1v1 écrit d'après les **Core Rules du 2026-07-16**, sans simplification de carte. Créé parce que le
premier simulateur simplifié a été rejeté (« il faut savoir modéliser toutes les cartes »). Dossier :
`riftbound/engine/` (README complet sur place).

## Contenu
- `game.py` : état, runes et coûts, chaîne (priorité/focus), cleanups, showdowns, combat, score, couches de might.
- `actions.py` : actions légales et processus de jeu.
- `cards.py` : **85 cartes du pool Akali/LeBlanc** + tokens (Mech, Reflection, Gold), une entrée par nom,
  extensible aux 938 cartes.
- `ai.py` : `SearchAgent` (recherche à 1 coup sur copies de la partie + déroulement par politique sur 2 tours,
  information cachée déterminisée) et `PolicyAgent`.
- `test_cards.py` : **91 tests de scénario**, chaque carte du pool couverte ; aucune erreur sur ~9 000 parties.
- `exp.py`, `exp_gorica.py`, `run.py` : Monte Carlo multiprocess (~1,2 s par partie sur 4 cœurs), résultats
  dans `results.json` et `results_gorica.json`. `exp.run_job(..., offset)` permet des graines neuves.

## Limites
- L'IA est à un coup, identique des deux côtés, ne bluffe pas et ne prépare pas de combinaison sur plusieurs
  tours. **Les écarts entre options sont plus fiables que les niveaux absolus.**
- Le niveau absolu semble optimiste pour Akali (voir [[Akali vs LeBlanc]]).
- Bonus d'équipement codés à la main (Long Sword +2, Sterak +3, Pendulum +1), Zhonya en version errata
  (voir [[Mots-clés]]).
- Non implémentés : 3-4 joueurs, XP/Level hors Scuttle Crab et Safety Inspector, cartes hors pool
  (Vilemaw's Lair, Decrees).

## Calibration
Liste Wuhan stock contre LeBlanc IQ #5 : 37,5 % ± 5 au premier calibrage, 31,6 % sur 320 parties ensuite,
contre ~35 % terrain (non vérifié). L'IA a été modifiée quand elle donnait 52 % : le changement se justifiait,
mais il a été accepté parce qu'il rapprochait de la cible (biais noté au [[Retex étude Akali-LeBlanc]]).

## Ancien simulateur (périmé)
`riftbound/sim/` : moteur simplifié, gardé pour l'historique. Ses conclusions sont fausses : voir
[[Chiffres périmés]]. Règles d'usage des résultats : [[Méthode statistique]].
