# Journal du wiki (le plus récent en bas)

Format : `## [AAAA-MM-JJ] type | titre` (types : ingest, decision, lint, restructure). Une à trois lignes par entrée.

## [2026-10-02] ingest | Premier retex et vault
Premier retex : chiffres surévalués corrigés (voir `docs/vault/Chiffres périmés.md`) ; notes du vault créées.

## [2026-10-06] ingest | Table d'entraînement et enregistrement des parties
Table d'entraînement v10 à v12, `train_games.py`, 17 parties de l'utilisateur ; énergie flottante (anciens chiffres périmés).

## [2026-10-07] restructure | Dépôt GitHub, CI et GitHub Pages
Dépôt `Geksmode/riftbound-lab` créé (PR 1) : `.github/workflows/ci.yml` (tests et build de la table en parallèle, publication sur Pages depuis `main`).

## [2026-10-07] decision | Équipe d'agents en sessions tmux, avec un chef
`scripts/equipe.sh`, rôles dans `.claude/` (PR 2), missions dans `docs/MISSIONS.md`. Détail : `pages/equipe.md`.

## [2026-10-07] restructure | Architecture en wiki
Ajout de `docs/wiki/` (index, conventions, journal, contrôle `scripts/wiki_lint.py`) ; `CLAUDE.md` réduit aux règles ; carte du dépôt et état déplacés en pages.

## [2026-10-07] decision | Ordre des chantiers de process
Porte d'intégration, chiffres versionnés, tests générés depuis le texte des cartes, boucle d'apprentissage des parties. Détail : `pages/etat.md`.

## [2026-10-08] ingest | Première itération des agents et retex
Intégration de `agent/cartes`, `agent/ia`, `agent/design` ; 1034 tests ; retex écrit. Voir `pages/etat.md` et `riftbound/retex/retex-equipe-iteration1.md`.

## [2026-10-08] decision | Gemdragon et Hwei : « ready up to 2 runes »
Le maximum possible jusqu'à 2, rien si toutes les runes sont prêtes (décision de l'utilisateur) ; tests des cas limites ajoutés.

## [2026-10-08] ingest | Problèmes connus corrigés
Repeat accordé (`tg2`), lien Replays, bulle de fin de partie à 360 px, boutons ± de l'éditeur ; robot `train/verif_mobile.mjs`.

## [2026-10-08] ingest | Porte d'intégration
`scripts/verifier.sh` (wiki, tests, fuzz, parties aléatoires, build, robot navigateur), branchée dans la CI. Page `pages/porte.md`.

## [2026-10-08] ingest | Chiffres versionnés
`engine/version.py` (même hachage que le manager), `engine/compare.py` (apparié / indépendant / refus), étiquetage des expériences. Page `pages/resultats.md`.

## [2026-10-08] ingest | Tests générés depuis le texte des cartes
`cardsets/test_texte_auto.py` : 233 sorts et 15 unités contrôlés automatiquement ; 8 faux positifs du classement expliqués ; Deathgrip à trancher.

## [2026-10-08] ingest | Indicateur d'énergie flottante
Badge dans la zone des runes de la table, puissance flottante exportée (`replay.snap` : `pp`) ; test Punch First.

## [2026-10-08] decision | Règles au plus près ; Deathgrip
Nouvelle règle permanente 6 de `CLAUDE.md` : en cas de doute, la lecture la plus fidèle aux Core Rules. Deathgrip : deux cibles alliées obligatoires (355.8).

## [2026-10-08] ingest | Assignation des dégâts de combat par le joueur
`game.assign_by_hand` : question « damage_pick » dans la table ; règles 465.2.c, 815, 826.

## [2026-10-08] ingest | Boucle des parties
Parties gardées dans le navigateur et téléchargeables, commit noté, rejeu par moteur exact (17/17 fidèles), `bilan_parties.py`, workflow « Apprentissage ». Page `pages/parties.md`.

## [2026-10-08] ingest | Match BO1 / BO3 et sideboard (moteur)
`match.py`, API `train.match_*`, sideboard validé (601.1.c). Écart signalé : en BO3 chaque joueur choisit son battlefield (486.5). Page `pages/match.md`.

## [2026-10-08] ingest | Interface du match BO1 / BO3 et du sideboard
Agent design : éditeur (sideboard), mode de match, robot `verif_match.mjs` ajouté à la porte ; chef : `obf` au rejeu.

## [2026-10-08] ingest | Deflect sur les répétitions
Les cibles d'une répétition (Repeat imprimé ou accordé) ne payaient pas le Deflect (809.1.c) : corrigé dans `actions.deflect_total` ; contrôle de tous les sorts `cardsets/test_deflect_auto.py` ; coût affiché dans le libellé du coup et le journal.

## [2026-10-08] decision | Duel entre amis : moteur fait, interface en attente
PeerJS, BO1 / BO3 ; `train.duel_new`, `match_duel_next`, test à deux processus ; spécification de l'interface dans `pages/duel.md`. Reprise à la session suivante.

## [2026-10-08] ingest | Interface du duel entre amis
Agent design : room PeerJS avec code, BO1 / BO3, reprise de l'invité ; robot `verif_duel.mjs` ajouté à la porte et à la CI (paquet `peer@1.0.2`).

## [2026-10-08] ingest | Duel : hôte bloqué « en attente »
Encodage JSON, renvoi du hello, statut côté hôte, journal de connexion copiable. Cause réelle non confirmée (non reproduite en local).

## [2026-10-08] ingest | Hidden : Back Off ne se cachait pas en la glissant
Table : lâcher une carte Hidden sur un battlefield lançait le ciblage pour la jouer quand elle était aussi jouable tout de suite (Back Off) ; les coups qui visent exactement la zone passent maintenant d'abord (`dropAt`). Moteur : Ember Monk, Ava Achiever et Pack of Wonders n'ont pas le mot-clé (images vérifiées) et ne se cachent plus ; Back Off cible « a unit » (amie ou déjà étourdie) pour un humain ; Switcheroo accepte deux unités de même Might pour un humain. Contrôle de toutes les cartes `cardsets/test_hidden_auto.py`, robot `verif_hidden.mjs` dans la porte. Les choix de l'IA ne changent pas, sauf qu'elle ne peut plus cacher ces trois cartes : petit changement de version du moteur.

## [2026-10-08] ingest | Main révélée (Scuttle Crab, Sabotage…)
Les effets « They reveal their hand » (Sabotage, Decree of Strength, Mindsplitter, Insightful Investigator, Bone Skewer, Ashe Focused, Scuttle Crab) passent par `Game.reveal_hand` : la table montre ces cartes pendant la résolution (même pendant la question posée), puis jusqu'au coup suivant joué chaîne vide (le focus est passé ; l'état « révélé » des règles finit à la résolution, 424.1.a.3 : le reste est un aide-mémoire de la table). Scuttle Crab montre aussi les cartes face cachée adverses ce tour. Tests `cardsets/test_main_revelee.py`, robot `verif_revele.mjs`. L'IA n'utilise pas encore la main révélée (sauf Scuttle Crab, déjà pris en compte) : à faire.

## [2026-10-08] decision | Direction artistique « Riftbound Lab — DA » appliquée à la table
Maquette Claude Design (artefact https://claude.ai/artifact/3AHpJfntDdJdWSwxKH4rJ1 : fondations, composants, accueil, table). Couche `train/da.css` injectée après le style existant (`/*DA_CSS*/`, `build.sh`) : papier clair, encre #15172B, néon #CFF26E, couleurs de domaine franches et douces, contours 1,5 px + ombre pleine, polices Bricolage Grotesque / Instrument Sans / Geist Mono. Apparence seulement : gestes, moteur et robots inchangés (porte verte). Le lecteur de replays garde l'ancien look sombre. Les écrans de la maquette qui n'existent pas encore (puzzle du jour, progression, coach, branches « Et si… ») ne sont pas faits.

## [2026-10-08] decision | DA « observatoire » (maquette mise à jour)
L'utilisateur a changé la maquette : nuit étoilée #0E1230, clair de lune #EDEBFA, or d'étoile #F3D58A, domaines en lueurs, pilules sans ombre dure, Instrument Serif / Figtree / Geist Mono ; toi = menthe #72E2BC, l'adversaire = corail #FF8F7E (flèches comprises), sélection = halo bleu #93B9FF. `train/da.css` réécrit, apparence seulement.

## [2026-10-08] ingest | Confort du plateau sur ordinateur
Plateau en pleine largeur sur écran large (`fitBoard` élargit la grille, la hauteur fixe l'échelle), ta main en grand (100 x 140 avant échelle), main adverse en petits dos, bouton « Terminer le tour » compact, barre du haut sur une ligne, aide du bas retirée (Menu > Aide). Vérifié en 1900x905, 1366x768, 1024x700 et 390x844 (pas de défilement horizontal, 0 erreur JS) ; porte « table » verte.

## [2026-10-08] ingest | Fuite : l'annonce d'un cache de l'IA nommait la carte
La bulle du coup adverse (IA ou ami en duel) utilisait `describe`, qui nomme la carte cachée (« cache Hidden Blade à Void Gate ») : `train._opp_describe` dit « cache une carte à … » et la flèche ne porte plus l'identifiant de la carte. Test `test_hidden_auto.the_table_never_names_the_card_the_opponent_hides` (parties réelles, échoue sans le correctif).

## [2026-10-08] ingest | Plateau à taille fixe pendant la partie
Le plateau rapetissait en cours de partie : la chaîne (cartes de 218 px de haut), les unités et les runes qui passaient à la ligne agrandissaient la grille et `fitBoard` réduisait l'échelle. Sur ordinateur : hauteurs fixes par rangée, colonnes de droite et des points hors calcul de hauteur (`contain: size`), ce qui déborde défile dans sa zone. Unités 74 x 104, main 84 x 118, zone légende 200 px, colonne de la chaîne 232 px, chaîne en haut de carte seulement (carte entière au toucher). Mesuré sur deux parties entières (1900x905, 1366x768, chaînes de 2 et 3) : une seule échelle par partie.
Puis (demandes de l'utilisateur) : chaque moitié en grille sur ordinateur, runes à gauche sur toute la hauteur en vraies cartes de rune (illustrations des planches, épuisées tournées et assombries), base au centre, légende + champion choisi agrandis à droite le long de la main, nom / score / deck / défausse dessous ; la main ne fixe plus la hauteur des zones. Plateau 922 px, une seule échelle par partie (mesuré sur deux parties entières).

## [2026-10-08] ingest | Téléphone : plateau plein écran, sans défilement, taille fixe
Sur le modèle d'une table en ligne (capture de l'utilisateur) : barre du haut réduite (logo, Menu, Journal), plateau plein écran (largeur de maquette 360 à 520 px mise à l'échelle de l'écran, hauteur qui remplit l'écran ; `fitBoard`), en haut la main adverse, ses zones et ses runes, au milieu les deux champs de bataille, en bas tes runes, tes zones et ta main ; barre de commande fixe sortie du plateau, chaîne affichée en haut sans rien pousser. Mesuré sur deux parties entières (390x844, 360x740) : pas de défilement, une seule échelle. Piège corrigé : la règle `.main` attrapait aussi les boutons de classe « main » (barre du sideboard de 779 px) ; elle vise maintenant `.shell > .main`.

## [2026-10-08] ingest | Téléphone robuste à la charge
Main, base, unités des champs de bataille et runes ne défilent plus jamais : les cartes se chevauchent selon leur nombre (`--n`, `--nx` pour les cartes épuisées tournées) et la largeur disponible (unités de conteneur) ; unités des champs de bataille à la taille de leur zone (jusqu'à 74 x 104) ; main à plat ; chaîne en une ligne compacte. Robot `verif_charge.mjs` (17 cartes en main, 6 unités de chaque côté par champ, 7 en base, 12 runes ; 393x851, 360x740, 412x915) dans la porte. Limite : au-delà d'environ 8 cartes en main, la partie visible de chaque carte passe sous 44 px (toucher moins sûr).
Puis : main agrandie sur téléphone. Quand la main est serrée (moins de 44 px visibles par carte, en pratique plus de 8 cartes), le premier toucher l'ouvre en grand (cartes 88 x 123, défilement horizontal au doigt) ; toucher une carte montre ses actions, un geste vers le plateau la glisse, un geste horizontal fait défiler sans glisser ; elle se referme après le coup, au ciblage ou en touchant ailleurs. Contrôlé par `verif_charge.mjs` (6 contrôles de gestes).

## [2026-10-08] ingest | Téléphone : ciblage par liste
Sur téléphone, les cibles sont petites ou cachées sous la bulle (ta base : impossible de choisir l'allié à déplacer pour Shuriken Flip). La bulle de ciblage et les questions pendant un effet listent maintenant les choix de l'étape en cours en grandes cartes (nom, camp, emplacement), les zones en boutons ; toucher la liste = toucher le plateau (`targetTap`). Robot `verif_cibles.mjs` (Shuriken Flip joué entièrement par la liste, allié pris en base) dans la porte.

## [2026-10-09] ingest | Retour : Evelynn « pas activée » depuis la face cachée
Reproduit par le chemin de la table : Evelynn jouée depuis la face cachée pendant ton tour déplace bien l'ennemi (Astral Heron en même temps compris). Deux cas sans effet, conformes au texte mais muets : pendant le tour adverse (« on your turn ») et sans unité ennemie ailleurs. Changements : `flush_triggers` ne pose plus la question « utiliser l'effet ? » quand une capacité à cible n'a aucune cible légale (402.4, `trig_target.options`), et le journal explique les deux cas. Petit changement de version du moteur (une question de moins pour l'IA dans ces cas). Cause confirmée par l'utilisateur : Evelynn jouée pendant le tour adverse, donc pas d'effet (fausse alerte, règle appliquée).

## [2026-10-09] retex | Session du 7 au 9 octobre (PR #1 à #15)
Revue autocritique : `riftbound/retex/retex-session-2026-10-07-09.md`. Leçons validées par l'utilisateur et écrites dans `CLAUDE.md` (PID, PR fusionnée, règle 7 robot / vrai téléphone), `docs/memoire/riftbound-training-table.md` et `docs/memoire/riftbound-engine.md`.

## [2026-10-09] ingest | Confirmations de l'utilisateur
Duel entre amis : fonctionne en réseau réel (page `duel.md`). Ciblage par liste sur téléphone : essayé sur son téléphone, fonctionne. Retex mis à jour.

## [2026-10-09] ingest | Flow jouable depuis la défausse dans la table
Le moteur appliquait Flow (829) : les 15 sorts se jouent depuis la défausse au coût de Flow imprimé puis sont bannis (`cardsets/test_flow_auto.py`, 3 tests : coût exact lu sur le texte, pas sans les ressources, pas sans Flow). Mais la table ne montrait aucune carte de la défausse portant ce coup : impossible de jouer Flow à la main. Maintenant un sort jouable avec Flow s'affiche au bout de ta main (marqué FLOW, bord pointillé or) et se joue comme une carte de la main (`train` exporte `trashu`). Robot `verif_flow.mjs` (ordinateur et téléphone) dans la porte.


## [2026-10-09] ingest | Écran de sélection contre l'IA à la Smash Bros, deck par défaut par légende
Demande : un deck par défaut par légende pour l'IA (meilleures listes du web, avec sideboard, même deck en BO1 et BO3) et un menu de sélection comme Smash Bros. Fait : `train.default_decks()` (liste jouable avec sideboard d'abord, puis meilleur classement), `catalog()` exporte les 49 légendes et les infos des listes (joueur, événement, place), écran « Contre l'IA » (grille, panneaux Toi / IA, « ? », niveau, BO1/BO3, options repliées). `decks/check_raw.py` valide un fichier de deck brut. Robot `verif_vs.mjs` (18 contrôles) dans la porte. Limites : seules Akali et LeBlanc ont une liste jouable (les sites de decks sont bloqués depuis le conteneur, la collecte par recherche web a été arrêtée par l'utilisateur) ; l'IA ne sideboarde pas.

## [2026-10-09] ingest | 28 decks par défaut (un par légende) pour l'écran « Contre l'IA »
Collecte par 5 agents (recherche web, transcriptions validées par `decks/check_raw.py`) : 28 légendes sur 49 ont une liste jouable, 25 avec sideboard. 21 restent grisées : listes publiées d'avant les bans (Stacked Deck, The Arena's Greatest, Aspirant's Climb, Ekko) et quota de recherches épuisé. Une carte de sideboard non confirmée (Yi Bladesman) retirée plutôt que devinée. Contrôle : 56 parties (chaque deck des deux côtés), 0 erreur, 0 partie sans fin. Doutes de transcription listés dans `riftbound/decks/README.md`.

## [2026-10-09] ingest | Les 49 légendes ont un deck par défaut ; menu par légende
Demande : pour les légendes manquantes, prendre les Best-Of d'Unleashed ; le menu déroulant ne montre que les variantes de la légende. Fait : 18 Best-Of d'Unleashed (Sydney, Vancouver, Utrecht, Hartford) et 3 Best-Of de Vendetta (Mel, Zed, Renekton, absents d'Unleashed). Les cartes bannies depuis sont jouables et marquées ⚠ ; une liste sans carte bannie passe toujours avant (`train.default_decks`, `_banned`). Lignes ambiguës retirées plutôt que devinées (Renekton, Yi Bladesman). 106 parties de contrôle, 0 erreur. Détail et doutes : `riftbound/decks/README.md`.

## [2026-10-09] ingest | Unités épuisées trop basses sur les battlefields (ordinateur) ; point marqué au tour adverse
Retour de l'utilisateur (capture) : les unités qui arrivent sur un battlefield s'affichent trop bas. Cause : une carte épuisée (tournée) restait calée en haut de son emplacement avant rotation, donc décalée de ~15 px vers le bas ; côté adverse elle passait sous la barre du battlefield. Correctif dans `base.css` (carte centrée puis tournée, survol compris). `verif_charge.mjs` contrôle désormais aussi 1875x902 et 1400x900 (cartes du battlefield dans leur moitié, en hauteur) : il échoue sans le correctif. Constat au passage, non traité : sur ordinateur, 17 cartes en main débordent.
Autre retour : « les points peuvent être gagnés pendant le tour adverse » (Charm sur mon unité). Le moteur le faisait déjà (469.1, aucune condition de tour) ; c'était Forgotten Monument dans la partie de l'utilisateur. Test ajouté : `charm_on_the_opponents_turn_gives_me_the_conquer` (showdown et combat).

## [2026-10-09] ingest | Deflect payé par les capacités déclenchées ; coût optionnel impayable non proposé ; 0 Might
Retours de l'utilisateur :
- **Deflect** : l'effet d'Akali, Deadly Weapon ciblait Master Yi, Tempered (Deflect au niveau 6) sans payer. Cause : `cards.trig_target` ne faisait payer Deflect que si la carte le demandait (`deflect=False` par défaut, héritage des premières cartes). Le défaut est maintenant `True` : toute capacité déclenchée qui choisit une unité ennemie paie Deflect (809.1.c), ce qui touche plusieurs dizaines de cartes. Test `deadly_weapon_pays_deflect_of_master_yi_tempered_at_level_6` (échoue sans le correctif).
- **Valley of Idols** : « oui », mais ni énergie payée ni buff. Le coût optionnel ne pouvait pas être payé, la question était posée quand même et l'effet abandonné en silence. Maintenant `Game.cost_payable` : pas de question si le coût est impayable (`may_pay.can`, sinon essai sur une copie de la partie pour un joueur humain), et le journal dit « pas de quoi payer ». Tests Valley of Idols et Monastery of Hirana (échouent sans le correctif).
- **0 Might** : le moteur appliquait déjà 142.4.b (1 dégât minimum). Test ajouté : 4 dégâts contre Akali, Silent + Scuttle Crab, le Crab survit et le défenseur garde le battlefield.

## [2026-10-09] ingest | Équipement attaché : plus de rééquipement ; flèches de la chaîne
- **Équipement** (retour utilisateur) : un équipement attaché ne peut plus changer d'unité par Equip. 434.1.e : une carte attachée a son texte de règles inactif tant qu'elle reste attachée ; `actions.abilities_of` ne propose plus les capacités imprimées d'un gear attaché (celles données par un autre effet restent). Il redevient équipable quand son unité meurt. Test `attached_equipment_stays_on_its_unit_until_it_leaves` ; le test Svellsongur, qui rééquipait l'équipement d'une unité à l'autre, suit maintenant la règle.
- **Flèches** (capture) : les flèches des sorts de la chaîne partaient du centre de la carte entière, alors que la chaîne n'en montre que le haut, donc du vide sous la carte. Elles partent maintenant de la partie visible. Les flèches du dernier coup de l'IA ne s'affichent plus quand la chaîne a des sorts : elles partaient du premier sort de l'IA même quand elles appartenaient à un autre. Robot `train/verif_fleches.mjs` dans la porte : une partie au hasard en 1893x899, chaque flèche contrôlée à chaque coup ; il trouve 6 flèches mal placées sans le correctif, 0 avec.

## [2026-10-09] ingest | Star Spring « another unit » ; Predict / Vision visibles ; fermetures et copie profonde
- **Star Spring** (retour utilisateur) : l'unité qu'on vient de jouer était proposée. Cause : `u is not played` dans une fermeture ; pour un choix humain, la table rejoue la partie depuis une copie profonde, et les fonctions ne sont pas copiées : l'objet capturé est celui de l'ancienne partie. Comparaison par uid. Test par `train` qui échoue sans le correctif.
- **Audit** de ce piège (agent cartes) sur toutes les fermetures de déclenchements et d'effets : 4 cartes de plus corrigées (Ashe, Focused, qui ne rendait jamais la carte bannie ; Fizz, Trickster, qui ne recyclait pas le sort rejoué ; Evelynn, Entrancing et Tideturner, qui lisaient la position sur un objet périmé). Tests `cardsets/test_table_copie.py` (échouent sans le correctif). Le conseil (`train.hint`) calcule maintenant sur une copie au lieu de remplacer la partie. Les recherches de l'IA sur `g.clone()` voyaient aussi ces cartes mal : quelques choix de l'IA peuvent changer.
- **Predict / Vision** (436, 817) : la question montre les cartes vues en grand (la carte en cours surlignée), avec « la garder sur le dessus » / « la recycler (sous le deck) ». Le moteur retient les cartes de ton deck que tu as vues (`Player.seen`) ; tant qu'elles restent dessus, ton deck porte « 👁 N » et le toucher ouvre le dessus connu. Tests `cardsets/test_predict_vue.py`, robot `train/verif_predict.mjs` dans la porte (ordinateur et téléphone, toucher simulé).

## [2026-10-09] ingest | Mode replay dans la table (« Mes parties » → « Revoir ») ; plus de lien Replays dans le menu
Demande : retirer l'entrée « Replays » du menu (elle ouvrait le lecteur des simulations) et revoir ses propres parties depuis « Mes parties », avec les analyses, dans le même design. Fait : `train.replay_load / replay_frame / replay_hint / replay_exit` (le moteur rejoue la partie enregistrée, une image par étape, l'état gardé à chacune de tes décisions) ; dans la table, même plateau en lecture seule, barre de lecture dans la colonne de droite (⏮ ⇤ ◀ ▶ ⇥ ⏭, curseur, lecture auto), « Analyser ce coup » = conseil de l'IA à ta place avec l'écart et « ton coup » repéré ; sur téléphone, panneau au-dessus de la barre de commande, réductible. La partie en cours revient telle quelle en quittant. Avertissement si le moteur a changé depuis la partie (rejeu qui s'écarte). Duels : pas encore de replay. Tests `cardsets/test_replay_table.py`, robot `train/verif_replay.mjs` (ordinateur et téléphone, toucher simulé). Le lecteur des simulations (`replays.html`) reste publié, hors du menu.

## [2026-10-09] ingest | IA générale : recherche à tirages communs et élimination en passes, mesurée et gardée
Demande : « améliore l'IA de base pour tous les decks ». Mesures appariées sur les decks par défaut (49 légendes, IA générique, `engine/exp_search.py` mode `defaut`, graines neuves à chaque ligne) : « sh » contre l'ancienne recherche à budget de simulations égal **55,8 % ± 1,9** (200 paires) ; tirages communs seuls à budget égal **58,4 % ± 2,3** (160) ; « sh » contre l'ancienne au niveau Normal **65,3 % ± 2,3** (160) ; « sh » contre tirages communs seuls 58,8 % ± 2,8 (120) ; Akali G2 contre LeBlanc IQ#5 avec leurs plans **62,5 % ± 2,9** (120). Défaut `RB_SEARCH=sh` gardé. Essais non retenus (horizon 1 ou 3 tours, poids de l'évaluation, sorts [Reaction] gardés) : rien au-delà de 2 écarts-types sur graines neuves. Temps par décision sur les mêmes positions, Pyodide sous node : 417 → 503 ms en moyenne, p95 2,4 s inchangé. Page [recherche de l'IA](../memoire/riftbound-search-ai.md). Lecteur de replays : noms des joueurs pris dans la partie (decks autres qu'Akali / LeBlanc), 5 parties ajoutées. Auteur : agent IA.

## [2026-10-09] change | Favicon Riftbound LAB
- Icône SVG (losange doré sur fond bleu nuit) en `<link rel="icon">` data-URI dans `riftbound/train/src.html` (table) et `riftbound/replays/viewer.html` (replays). Autorisée par la CSP (`img-src data:`). Non vérifiée dans un navigateur.

## [2026-10-09] fix | Chaîne : capacités activées (légende, Empower, Equip) sans illustration
- Cause : le moteur nomme une capacité activée « Carte: capacité » (« Akali, Rogue Assassin: Empower », « Long Sword: Equip », « Baited Hook: Hook ») ; `cardFor` ne la rattachait à aucune carte, d'où une carte sans image avec le texte brut. `cardFor` (table et replays) retire maintenant le suffixe « : capacité ».
- Vérifié : noms relevés sur 40 parties aléatoires (5 formes, toutes corrigées) et logique de `cardFor` rejouée sous node. **Pas** vérifié dans un navigateur (cache Pyodide absent) : à confirmer sur la table.

## [2026-10-09] fix | Table : « T.replay_load n'est pas une fonction » après publication
- Cause : la page neuve chargeait les fichiers du moteur (`py/*.py`, `data/*`) depuis le cache du navigateur (GitHub Pages : 10 min), donc l'ancien `train.py` sans `replay_load`.
- Correction : `src.html` ajoute `?v=<commit>-<empreinte du moteur>` (constante `BUILD` posée par `build.sh`) à chaque fichier chargé dans Pyodide. Pas essayé sur un vrai téléphone.

## [2026-10-09] fix | Irelia, Blade Dancer : plus de question inutile à chaque « choose »
- Retour utilisateur : la question « utiliser l'effet ? » d'Irelia revenait à chaque choix d'unité alliée, même légende inclinée.
- Correction (`cardsets/legends.py`) : le coût porte `can` ; pas de question si la légende est déjà inclinée, si aucune rune ne peut être payée, ou si l'unité choisie est déjà prête (« ready it » ne ferait rien). Tests `irelia_no_question_when_useless_or_unpayable` et `irelia_second_choice_not_asked_once_legend_exhausted` (échouent sans la correction).

## [2026-10-09] change | Chaîne : flèches d'un seul élément, survol / toucher pour les autres
- Demande : la chaîne affichait les flèches de ciblage de tous ses éléments à la fois, trop brouillon. Maintenant (`riftbound/train/src.html`, `drawArrows`) seules les flèches du haut de la chaîne (dernier ajouté) sont tracées ; survoler (souris) ou toucher (téléphone) un autre élément montre les siennes à la place (contour blanc sur l'élément montré, `base.css` `.sitem.foc`) ; en quittant la chaîne ou quand elle change, retour au haut. Les flèches du dernier coup de l'IA (chaîne vide) ne changent pas.
- Téléphone : les éléments anciens de la chaîne étaient masqués (`display: none`) ; ils sont maintenant visibles dans la bande de la chaîne (qui défile), sinon on ne pouvait pas les toucher. Le toucher ouvre toujours la carte en grand, comme avant.
- Vérifié par robot (`verif_fleches.mjs`) : chaîne de 2 éléments ciblants posée dans l'état affiché, en ordinateur 1893x899 (survol) et en téléphone 360x740 (toucher simulé) : une seule série de flèches par défaut, celles de l'autre élément au survol / toucher, retour au haut ensuite. Pas essayé sur un vrai téléphone.

## [2026-10-09] fix | Flèches de la chaîne : toucher sur téléphone (échec du robot en CI)
- CI (PR #26) : « toucher de l'élément du dessous » restait sur les flèches du haut. Cause : le `mouseover` de compatibilité qui suit un toucher pouvait tomber hors de la chaîne (carte en grand) et ramenait au haut, selon le moment.
- Correction : survol compté seulement pour une vraie souris (`pointerover`, `pointerType === "mouse"`) ; toucher via `pointerdown`. Robot : il touche un point de la carte qui n'est pas recouvert (le haut d'un élément de la chaîne passe sous la main adverse en 360x740), et ferme la carte en grand avant de toucher le haut. 3 passages verts de suite en local ; pas essayé sur un vrai téléphone.

## [2026-10-09] feat | Table : les choix de runes se font en touchant les runes du plateau
- Avant : « Recycler / Prépare quelle rune ? » ouvrait une bulle avec les noms (« rune Fury »…). Maintenant les runes proposées s'allument sur ta rangée de runes (en bas à gauche), on en touche une puis « Valider ». `train.py` : `_ask_uid` reconnaît tes runes, l'état expose `p.runeu` (uid des runes, même ordre que `p.runes`) ; `src.html` : `data-uid` sur les runes de l'ask, halo `.rune.drop-ok / .picked`.
- Vérifié par robot (ordinateur 1400x900, ask posée dans l'état affiché, pas une vraie partie jusqu'à une carte qui recycle) : 2 runes allumées, un clic en choisit une, Valider actif, aucun bouton de nom. Pas essayé sur téléphone ni sur un vrai ask moteur de bout en bout. Tests moteur : 1117/1117.

## [2026-10-10] decision | Tests de l'IA sur des decks variés
- Règle de l'utilisateur : les prochains tests de l'IA se font sur des decks variés (`exp_search.py ... defaut`, environ 80 paires), plus sur Akali contre LeBlanc par défaut. Mémoire `ia-tests-decks-varies.md`, règle 8 de `CLAUDE.md`, agents `ia` et `simulation`.
- En cours : essai `pts_ramp` (points pondérés, idée de l'utilisateur : premiers points moins chers, derniers plus chers) ; réglage désactivé par défaut, criblage r=2, r=4 et courbe 4→10 (`pts_ramp=1.5,pts_hi=0`) en mode defaut, graines 710000+, 711000+, 713000+.

## [2026-10-10] decision | Fin de la règle « replays après chaque simulation »
- Décision de l'utilisateur : on enlève la règle 3 de `CLAUDE.md` (replays systématiques). `add_replays.py` et le lecteur restent, utilisés seulement sur demande. Mémoire `replays-after-every-sim.md` supprimée, mentions retirées des agents, de l'index et de MISSIONS.

## [2026-10-10] ingest | Bandle Tree : les deux cartes cachées affichées
- Signalé par l'utilisateur : la 2e carte cachée sur Bandle Tree était invisible (seule la 1re était transmise à la table et au lecteur de replays). `replay.snap` ajoute `fds` (toutes les cartes cachées, `fd` gardé pour les anciens replays), `train.py` masque celles de l'IA et ajoute `fdus` ; table et lecteur affichent une carte par entrée. Test `table_shows_every_facedown_card_bandle_tree`, robot `verif_hidden.mjs` (bureau ; téléphone 360 px vérifié à part).

## [2026-10-10] ingest | Table : étiquette « COPIE » réservée aux vraies copies
La table affichait « COPIE » sur tout jeton sauf Reflection et Mech (les Recruit de Viktor, Bird, Sprite…). L'instantané (`replay.snap`) porte désormais `cp` quand l'unité copie une autre unité (Reflection, Shady Spectacles) ; les autres jetons sont étiquetés « JETON ». Test `snap_marks_only_real_copies`.

## [2026-10-10] ingest | Table : unités homonymes numérotées
- Demande de l'utilisateur : avec plusieurs fois la même unité en jeu, on ne savait pas laquelle on ciblait. `Game.dup_no` / `Game.label` numérotent (par uid, les deux joueurs ensemble) les objets en jeu qui partagent un nom : badge doré « #2 » en bas à gauche de la carte (`snap` : `dn`), même numéro dans les libellés (cibles, déplacements, questions, bande de choix sur téléphone). Les cibles de tes coups disent « adverse », celles des coups de l'IA « (à toi) ». Seule en jeu, une unité n'a pas de numéro. Test `duplicate_units_are_numbered_in_labels_and_snap`.

## [2026-10-10] ingest | Table : équipement attaché décalé à gauche
- Signalé par l'utilisateur (capture) : une unité équipée sur un battlefield avait sa might coupée, car l'équipement dépassait vers le haut et poussait l'unité vers le bas. L'équipement dépasse maintenant vers la gauche (16 px par équipement), sur la même ligne que l'unité (`base.css`, `.slot.geared`). Vérifié par robot à 1899, 1400, 393 et 360 px (1 et 2 équipements, unité prête ou épuisée) : might visible, équipement à gauche, pas de défilement horizontal.

## [2026-10-10] feat | Table : mode tutoriel « Apprendre à jouer »
- Demande de l'utilisateur : un tutoriel court et ludique pour néophytes ; deck de base Maître Yi (sa demande). Menu → 4 écrans d'intro, puis une vraie partie (moteur complet) Yi Bladesman contre Darius, donne 202, IA débutante (`train.Gentle`), 6 défis à étoiles et un coach. Page [tutoriel](pages/tutoriel.md).
- Moteur : `TGame.emit` note `combat_won` (exposé `cw` dans l'état) ; test `tutorial_game_is_fixed_and_playable`. Vérifié par robot `verif_tuto.mjs` (1400x900 et 393x851, toucher simulé), ajouté à `scripts/verifier.sh`. Pas essayé sur un vrai téléphone.

## [2026-10-10] decision | La table s'appelle « Riftbound Observatory »
- Choix de l'utilisateur après un brainstorm : l'ancien nom « Riftbound Entraînement » ne collait pas à la DA « observatoire ». Titre de l'onglet et du menu changés (`train/src.html`), même lien d'artefact.

## [2026-10-10] decision | Menu de la table en tableau de bord
L'utilisateur a choisi le tableau de bord parmi 5 dispositions : deck d'entraînement (aucun par défaut, choisi par le joueur) et dernière partie en haut, les autres entrées en rangée compacte. Détail : `memoire/riftbound-training-table.md` ; robot `train/verif_dash.mjs` dans la porte.
