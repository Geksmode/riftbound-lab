# Conventions du wiki Riftbound LAB

Objectif : qu'une session ou un agent charge **le minimum de contexte** pour sa tâche. On ne lit jamais un dossier
entier : on part de [index.md](index.md), on ouvre seulement les pages utiles.

## Trois couches
1. **Sources** (ne se résument pas, on les consulte) : code et tests (`riftbound/engine/`), règles (`riftbound/rules/`),
   cartes (`riftbound/cards/`), données et résultats (`riftbound/**/results*.json`).
2. **Pages** (le savoir, une idée par page) : `docs/wiki/pages/` (état, dépôt, équipe, publication),
   `docs/memoire/` (faits et décisions, un fait par fichier, index `MEMORY.md`), `docs/vault/` (notes Obsidian du projet,
   liens `[[...]]`, index `Riftbound - Accueil.md`).
3. **Schéma** : ce fichier + `CLAUDE.md` (règles permanentes de l'utilisateur, toujours chargées).

## Lire (query)
1. Lis `index.md` (moins de 100 lignes). 2. Ouvre les pages de la ligne « Par tâche » qui te concerne. 3. Descends aux
sources seulement pour vérifier. Une réponse qui synthétise plusieurs pages et reste utile devient une page.

## Écrire (ingest) : à chaque résultat, décision ou changement durable
1. Mets à jour la page concernée, ou crée-en une (un sujet, 80 lignes au plus, en-tête ci-dessous).
2. Ajoute-la à `index.md` (une ligne : lien + résumé).
3. Ajoute une ligne à `log.md` (date, type, titre, auteur).
4. Un chiffre remplacé par une nouvelle mesure : mets l'ancien dans `docs/vault/Chiffres périmés.md`.
Chaque agent tient à jour les pages de **son** périmètre dans le même commit que son travail ; `retex` relit.

## Format d'une page (`docs/wiki/pages/`)
```
---
titre: <court>
resume: <une ligne, ce que la page permet de savoir>
maj: AAAA-MM-JJ
sources: <chemins ou liens vérifiables>
---
```
Puis le contenu : faits datés, chemins exacts, « à confirmer » pour ce qui n'est pas vérifié. Liens relatifs en Markdown (texte entre crochets, chemin entre parenthèses)
dans `docs/wiki/`. Pas de copie d'un contenu qui existe ailleurs : un lien.

## Nettoyer (lint)
`python3 scripts/wiki_lint.py` : pages sans en-tête, pages trop longues, liens cassés, pages absentes de l'index,
fichiers de `docs/memoire/` absents de `MEMORY.md`. Il tourne dans la CI. `retex` le lance en fin de mission.

## Limites
- Le vrai vault Obsidian de l'utilisateur n'est jamais modifié sans demande explicite.
- Aucune image de carte Riot dans le dépôt de la doc ; aucun secret.
