# Index du wiki Riftbound LAB

Point d'entrée de toute session. Conventions : [WIKI.md](WIKI.md). Journal : [log.md](log.md). Règles permanentes : `CLAUDE.md`.

## Par tâche : lis seulement ces pages
| Je dois… | Lire |
|---|---|
| savoir où en est le projet | [pages/etat.md](pages/etat.md) |
| trouver un fichier, lancer tests ou build | [pages/depot.md](pages/depot.md) |
| intégrer une branche, vérifier avant de pousser | [pages/porte.md](pages/porte.md) |
| modéliser une carte ou déboguer une interaction | `riftbound/engine/README.md`, `riftbound/engine/cardsets/GUIDE.md`, [moteur](../memoire/riftbound-engine.md) |
| travailler sur l'IA ou les plans | [recherche de l'IA générale](../memoire/riftbound-search-ai.md), [plans](../memoire/riftbound-gameplans.md), [IA tempo](../memoire/riftbound-tempo-ai.md), [LeBlanc réel](../memoire/leblanc-real-play.md), [parties](../memoire/riftbound-train-games.md) |
| analyser les parties de l'utilisateur | [pages/parties.md](pages/parties.md) |
| comparer ou regrouper des résultats | [pages/resultats.md](pages/resultats.md) |
| lancer une simulation ou un retex | [méthode retex](../memoire/retex-method.md), [replays après chaque sim](../memoire/replays-after-every-sim.md), [manager](../memoire/riftbound-manager.md) |
| toucher la table, le menu, l'éditeur de deck | [table](../memoire/riftbound-training-table.md), [éditeur](../memoire/riftbound-deckbuilder.md), [publication](pages/publication.md) |
| coordonner des agents | [pages/equipe.md](pages/equipe.md), `docs/MISSIONS.md`, `.claude/chef.md` |
| connaître le jeu, les règles, la méta | `docs/vault/Riftbound - Accueil.md` (index du vault), `riftbound/rules/` |
| reprendre le compte, la routine, les artefacts | `docs/REPRISE.md` (§2 artefacts, §3 publication, §4 routine) |

## Pages du wiki
- [État du travail](pages/etat.md) : fait, en cours, à faire (2026-10-07)
- [Carte du dépôt et commandes](pages/depot.md) : dossiers, tests, build
- [Équipe d'agents](pages/equipe.md) : chef, agents tmux, intégration
- [Publication de la table](pages/publication.md) : Pages, artefact, pièges
- [Chiffres versionnés](pages/resultats.md) : version du moteur dans chaque résultat, `compare.py`
- [Duel entre amis](pages/duel.md) : room avec code (PeerJS), BO1 / BO3, ce qui est testé et ce qui ne l'est pas
- [Match BO1 / BO3 et sideboard](pages/match.md) : règles du match, code, choix de l'IA
- [Boucle des parties](pages/parties.md) : garder, déposer, rejouer fidèlement, analyser tes parties
- [Porte d'intégration](pages/porte.md) : `scripts/verifier.sh`, ce qu'il vérifie, CI

## Mémoire des agents (`docs/memoire/`, index [MEMORY.md](../memoire/MEMORY.md))
Un fait par fichier : sources, objectif, Akali vs LeBlanc et Yi, moteur, decklists, plans, retex, manager, replays, table, éditeur de deck.

## Vault du projet (`docs/vault/`, index `Riftbound - Accueil.md`)
Règles du jeu, mots-clés, méta, fiches de légendes, matchups, méthode statistique, retex, chiffres périmés, questions ouvertes.

## Sources à consulter (pas à résumer)
Règles : `riftbound/rules/source/core_rules_2026-07-16.txt` · cartes : `riftbound/cards/cards_unique.csv` · decks : `riftbound/decks/decks.json` ·
parties de l'utilisateur : `riftbound/train/games/` · résultats de simulation : `riftbound/**/results*.json`, `riftbound/manager/sessions/`.
