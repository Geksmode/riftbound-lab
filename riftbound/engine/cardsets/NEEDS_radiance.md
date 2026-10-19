# Radiance : carte non modélisée

- **Neeko, Blending In** (radiance) — « If Neeko is in your deck, choose 3 different legends in addition to your
  starting legend. When you play me, choose one of those legends and add it to your Legend Zone while I'm on the
  board. » Il faudrait (1) un choix à la construction du deck (règle 103.1 des Core Rules du 2026-10-09 : trois
  légendes de plus, enregistrées avec la liste, éditeur de deck et `decks.json` compris) et (2) plusieurs légendes
  dans la Legend Zone (107.4.d.1-2). Le moteur n'a qu'une légende par joueur (`Player.legend`, `legend_name`) et
  chaque légende déjà modélisée agit sur `g.p[pid].legend` : une légende ajoutée par Neeko épuiserait ou
  renforcerait la légende du joueur au lieu d'elle-même. Carte bloquée dans l'éditeur (règle 4 de CLAUDE.md)
  jusqu'à ce que le moteur gère plusieurs légendes.
