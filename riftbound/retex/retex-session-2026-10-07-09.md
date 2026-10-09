---
tags: [riftbound, retex, outil, moteur]
maj: 2026-10-09
---
# Retex : session du 7 au 9 octobre 2026 (PR #1 à #15, 55 commits)

Revue autocritique factuelle (méthode : `docs/memoire/retex-method.md`). Périmètre : CI et GitHub Pages, équipe
d'agents, wiki, porte d'intégration, chiffres versionnés, boucle des parties, règles (Deathgrip, Deflect, dégâts de
combat, Hidden, main révélée, Evelynn), match BO1/BO3 et sideboard, duel entre amis, deux directions artistiques,
confort du plateau sur ordinateur, version téléphone.

## Ce qui tient
- **La porte d'intégration** (`scripts/verifier.sh`) a attrapé des régressions avant l'utilisateur : la règle CSS
  `.main` qui étirait le bouton « Valider les échanges » à 779 px (robot du match), le sideboard bloqué pour le duel.
- **Tests discriminants** : presque chaque correctif du moteur a un test qui échoue sans lui (Deflect sur les
  répétitions, scan Hidden, Deathgrip, main révélée, annonce d'une carte cachée, Evelynn).
- **Fidélité aux règles** : Hidden vérifié sur les 39 cartes à mot-clé (cacher, jouer depuis la face cachée au tour
  suivant, jamais le tour même) ; trois cartes qui ne faisaient que mentionner le mot-clé retirées (images vérifiées).
- **Chiffres** : l'IA générale est restée annoncée « on ne sait pas » (51,9 % ± 3,0).
- **Mesures de mise en page** : depuis les PR #12 à #15, l'échelle du plateau est mesurée sur des parties entières
  (une seule valeur) et un robot de charge (17 cartes en main, 6 unités par côté) contrôle débordement et défilement.

## Ce qui était surévalué
1. **« Vérifié » sur téléphone.** Les robots verts n'ont pas empêché quatre retours de l'utilisateur sur son téléphone :
   main qui déborde avec beaucoup de cartes, base qui défile, unités illisibles sur les champs de bataille, base
   inaccessible au ciblage (Shuriken Flip).
2. **Ce que montraient mes propres captures.** Boutons hors du cadre (« tu vois bien que… »), runes coupées, plateau qui
   rapetissait en cours de partie : c'est l'utilisateur qui l'a vu. Cause du dernier : ma mise à l'échelle dépendait du
   contenu (la chaîne affichait des cartes de 218 px).
3. **Premier robot Hidden.** Il passait avec Zhonya (sans cible) alors que Back Off, jouable et cachable à la fois, ne se
   cachait pas au glisser.
4. **Duel entre amis.** Le correctif « hôte bloqué en attente » n'est pas confirmé sur un vrai réseau (non reproduit en
   local, serveur PeerJS local différent du serveur public).
5. **Evelynn.** J'ai d'abord cru reproduire un bug : mon test lisait une copie périmée de l'unité (état restauré par
   copie profonde). Le moteur l'appliquait bien ; seul défaut réel : la question « utiliser l'effet ? » posée sans cible.

## Biais
- Travail pensé d'abord pour l'ordinateur, alors que l'utilisateur joue surtout sur téléphone.
- Tests du cas simple plutôt que du cas difficile.
- Confiance dans mes propres robots et oracles de test.
- Longues commandes groupées (vérification + commit + push) : l'utilisateur a dû interrompre deux fois pour ajouter un retour.
- `pkill -f` réutilisé après avoir été noté comme piège : le shell a été tué au moins quatre fois (code 144).
- Mise à jour d'une PR déjà fusionnée (PR #5).

## Ce qu'on change (validé par l'utilisateur le 2026-10-09)
- `CLAUDE.md` : arrêter les processus par PID ; vérifier qu'une PR est fusionnée avant de committer ; règle 7 précisée
  (« vérifié par robot » vs « essayé sur un vrai téléphone »).
- `docs/memoire/riftbound-training-table.md` : tailles indépendantes du contenu et vérification par mesure ; cas
  difficile ; tout choix faisable hors du plateau sur téléphone ; pièges CSS ; rien de caché dans les textes adverses.
- `docs/memoire/riftbound-engine.md` : mot-clé lu dans le texte imprimé ; effet qui ne s'applique pas expliqué, pas de
  question sans cible ; objets relus par `g.obj(uid)` dans les tests via `train` ; choix humains selon le texte.

## Questions ouvertes
- ~~Duel en réseau réel~~ : confirmé par l'utilisateur le 2026-10-09 (partie en réseau qui fonctionne).
- ~~Cas Evelynn~~ : confirmé par l'utilisateur le 2026-10-09, c'était le tour adverse, donc pas d'effet (« on your turn ») : fausse alerte, le moteur était juste. Le journal le dit désormais.
- Ciblage par liste sur téléphone : essayé par l'utilisateur sur son téléphone le 2026-10-09, il fonctionne. Les autres changements téléphone (rangées resserrées, main agrandie) : pas de retour explicite.
