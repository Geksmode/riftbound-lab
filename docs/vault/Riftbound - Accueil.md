---
tags: [riftbound, index]
maj: 2026-10-02
---
# Riftbound : accueil du vault

Tout ce que le projet Riftbound LAB a appris jusqu'au 2026-10-02. Les chiffres de simulation sont ceux
**corrigés par le retex** ; les anciens chiffres surévalués sont signalés comme tels dans
[[Chiffres périmés]].

## Le jeu
- [[Règles du jeu]] : tour, ressources, score, combat, chaîne
- [[Construction de deck]] : 40 cartes, 12 runes, 3 battlefields, sideboard
- [[Mots-clés]] : tableau des mots-clés (800-829)
- [[Règles de tournoi]] : BO3, temps, sanctions juge
- [[Ban list]] : cartes bannies en Standard au 2026-10-02

## Méta et decks
- [[Méta Vendetta (septembre 2026)]] : qui gagne, tier list, tendances
- [[Akali, Rogue Assassin]] : ma légende, cœur des listes, cartes clés
- [[Akali Heron (Gorica)]] : **ma liste**, et son plan de side contre LeBlanc
- [[LeBlanc, Deceiver]] : comment elle gagne, sa liste type
- [[Akali vs LeBlanc]] : le matchup, ce qui est mesuré et ce qui ne l'est pas
- [[Akali vs Yi Bladesman]] : fiche de jeu (texte des cartes, non mesuré) et puzzles https://claude.ai/artifact/Ds4TSVoR6tcNRbQKBKYpcP
- [[Carnet de parties]] : une entrée par vraie partie, pour trouver les erreurs récurrentes

## Simulation et méthode
- [[Moteur de règles fidèle]] : le simulateur du projet et ses limites
- [[Méthode statistique]] : règles apprises au retex pour ne plus surévaluer
- [[Retex étude Akali-LeBlanc]] : ce qui tient, ce qui était faux, les biais
- [[Retex table d'entraînement]] : l'outil de jeu contre l'IA, ses erreurs et ce qu'on ne sait pas
- [[Chiffres périmés]] : anciens chiffres à ne plus citer
- [[Agent manager]] : sessions automatiques de simulation et leurs retex

## Données et suite
- [[Sources et données]] : fichiers du projet, sites, API, accès réseau
- [[Questions ouvertes]] : ce qu'on ne sait pas encore et comment le trancher

## Comment lire les chiffres
Une ligne de simulation à 160 parties a environ **±4 points** d'écart-type ; une différence entre deux
lignes, **±5,5 points**. Sous 2 écarts-types, la réponse honnête est « on ne sait pas ».
Les écarts entre options sont plus fiables que les niveaux absolus (voir [[Moteur de règles fidèle]]).
