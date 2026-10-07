---
tags: [riftbound, methode, simulation]
maj: 2026-10-02
---
# Méthode statistique (leçons du retex)

Règles adoptées après le [[Retex étude Akali-LeBlanc]] pour toute étude sur le [[Moteur de règles fidèle]].

## Ordres de grandeur
- Une ligne de 160 parties : **±4 points** d'écart-type.
- Une différence entre deux lignes de 160 : **±5,5 points**.
- Pour conclure à un écart à 160 parties par ligne, il faut **~11 points** (2 écarts-types) ; à 640 parties,
  ~5,5 points.

## Règles
1. **Ne jamais chiffrer une option sur le lot qui a servi à la choisir.** Le meilleur de 5 tirages est gonflé
   par le hasard. Toute option retenue est rejouée sur des graines neuves (`run_job(..., offset)`), et seul ce
   chiffre est annoncé.
2. **Ne pas regrouper des lignes jouées sur les mêmes graines** comme si elles étaient indépendantes
   (erreur des « 4 480 parties » et de l'initiative « 14 points »).
3. **Annoncer chaque gain avec son intervalle**, et dire « on ne sait pas » sous 2 écarts-types.
4. **Une source terrain n'est citée comme chiffre que si elle a été lue sur la page**, pas dans un résumé
   de moteur de recherche.
5. **Comparer des quantités comparables** : un winrate contre tout le field n'est pas un winrate de matchup.
6. **Se méfier du premier lot** : à chaque fois, la confirmation a fait baisser le chiffre.
7. Si une demande est ambiguë, poser une question fermée avant de construire.

## Vocabulaire
« Retex », dans ce projet, veut dire autocritique factuelle de ce qui a été affirmé : ce qui tient, ce qui
était surévalué, quels biais, quoi changer.
