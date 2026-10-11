---
titre: Plans de jeu par légende
resume: un plan sourcé pour chaque légende des decks par défaut, hypothèses non mesurées, codées dans engine/plans_legendes.py
maj: 2026-10-11
sources: riftbound.gg (guides de Den), mobalytics, cardsrealm ; voir chaque fiche
---
# Plans de jeu par légende (49 decks par défaut)

Fichier généré par `python3 riftbound/engine/plans_legendes.py` depuis la table `LEG` : ne pas l'éditer à la main.

**Ce sont des hypothèses, pas des résultats.** Aucun plan n'est mesuré. Mesure proposée (en local) :
`python3 exp_search.py 80 <graines neuves> "sh@plan=leg" sh defaut` (A = plans par légende, B = IA sans plan),
puis confirmation sur d'autres graines neuves ; « on ne sait pas » sous 2 écarts-types.

Confiance : 40 guides lus en entier, 3 plans déduits du deck (guide d'un autre archétype), 6 sans guide lisible (sites autres que riftbound.gg bloqués le 2026-10-11).
Principes communs : [guide du score](https://riftbound.gg/the-in-depth-guide-to-scoring-in-riftbound/) (conquérant / tenant, zone létale à 6 points).

Ce que l'IA en fait : `axe` C (conquérant : valorise la might sur les battlefields) ou T (tenant : valorise les battlefields tenus et les runes prêtes quand l'adversaire approche de la victoire) ; `garde` = jamais renvoyées au mulligan ; `cles` = valorisées en jeu ; contrôle et combo valorisent la main.

## Ahri, Nine-Tailed Fox
- Source : [riftbound.gg](https://riftbound.gg/ahri-nine-tailed-fox-guide/) (maj 2026-07-17) ; confiance : guide lu en entier.
- Deck par défaut : `ahri_kawaii-tsukiko_rq-sydney_127th` ; champion Ahri, Inquisitive ; courbe 1:7, 2:15, 3:3, 4:10, 5:2, 7+:2.
- Plan codé : axe T, style controle ; garde : Ravenbloom Student, Sprite Fountain ; clés : Ravenbloom Student, Thousand-Tailed Watcher, Ahri, Inquisitive.
- Résumé : Jeu de tenue : ancre tôt (Ravenbloom Student), garder une rune ouverte pour réagir aux conquêtes, Sprite pour tenir ; Thousand-Tailed Watcher conquiert en fin de partie. Le 8e point se prend plutôt en conquérant dans son tour qu'en tenant (une tenue rend souvent un tour à l'adversaire).

## Akali, Rogue Assassin
- Source : [riftbound.gg](https://riftbound.gg/akali-rogue-assassin-guide/) (maj 2026-09-14) ; confiance : guide lu en entier.
- Deck par défaut : `akali_dongdong_wuhan-open_5th` ; champion Akali, Deadly Weapon ; courbe 1:10, 2:16, 3:2, 4:7, 6:1, 7+:3.
- Plan utilisé : plan existant « Gorica » (`plans.py`).
- Résumé : Plan existant Gorica (plans.py) gardé. Guide : removal efficace, contrôle du plateau, deux moteurs de pioche à protéger (Stellacorn Herder, Kai'Sa, Survivor), poser Akali (fragile) seulement sur un plateau sûr ; Astral Heron ancre la fin de partie.

## Ambessa, Matriarch of War
- Source : [riftbound.gg](https://riftbound.gg/ambessa-matriarch-of-war-guide/) (maj 2026-08-14) ; confiance : guide lu en entier.
- Deck par défaut : `ambessa_tss-riftboundtee_rq-la_201st` ; champion Ambessa, The Wolf ; courbe 1:4, 2:15, 3:11, 4:4, 5:5.
- Plan codé : axe C, style midrange ; garde : Legion Marauder, Irresistible Faefolk ; clés : Legion Marauder, Ambessa, The Wolf.
- Résumé : Plateau : prendre le plateau tôt et forcer l'adversaire à défendre ; la légende prépare une unité (Empower) pour marquer ; removal en base. Légende faible en partie longue : recycler les runes sans hésiter en fin de partie.

## Annie, Dark Child
- Source : [riftbound.gg](https://riftbound.gg/annie-dark-child-guide/) (maj 2026-07-03) ; confiance : guide lu en entier.
- Deck par défaut : `annie_prismaticism_rq-hartford_7th` ; champion Annie, Stubborn ; courbe 1:5, 2:16, 3:7, 4:8, 6:3.
- Plan codé : axe C, style aggro ; garde : Traveling Merchant, Inferna, Rengar, Pouncing ; clés : Traveling Merchant, Kai'Sa, Survivor.
- Résumé : Aggro : conquérir chaque tour jusqu'à 8. La légende prépare 2 runes en fin de tour : tout dépenser dans son tour et garder ces 2 runes pour réagir. Pioche indispensable (Traveling Merchant, Kai'Sa) ; les attaquants [Assault] tiennent mal ; Vex et Star-Crossed contre une grosse unité adverse.

## Azir, Emperor of the Sands
- Source : [riftbound.gg](https://riftbound.gg/azir-emperor-of-the-sands-guide/) (maj 2026-09-08) ; confiance : guide lu en entier.
- Deck par défaut : `azir_mice-squirtle_rq-la_3rd` ; champion Azir, Sovereign ; courbe 1:14, 2:13, 3:6, 4:3, 6:3.
- Plan codé : axe T, style controle ; garde : Doran's Shield, Soul Sword, Eye of the Herald ; clés : Azir, Sovereign.
- Résumé : Contrôle : 13 équipements bon marché pour créer des Sand Soldiers ; Arise! (sort signature) finit la partie. Contre un deck lent : poser chaque tour et marquer ; contre un deck rapide : valeur, removal.

## Darius, Hand of Noxus
- Source : [riftbound.gg](https://riftbound.gg/darius-hand-of-noxus-guide/) (maj 2026-07-06) ; confiance : guide lu en entier.
- Deck par défaut : `darius_mice-diamondhat_rq-utrecht_6th` ; champion Darius, Trifarian ; courbe 0:2, 2:16, 3:9, 4:6, 5:5, 6:1.
- Plan codé : axe C, style aggro ; garde : Baited Hook, Carrion Dredger, Noxus Hopeful, Honest Broker ; clés : Baited Hook.
- Résumé : Aggro / tempo pressé : développer au maximum tôt (Legion plus fort tôt), Baited Hook sacrifie une petite unité pour en poser une plus grosse en base ; removal Order. Peu de retour possible : ne pas prendre de retard.

## Diana, Scorn of the Moon
- Source : [riftbound.gg](https://riftbound.gg/diana-scorn-of-the-moon-guide/) (maj 2026-09-14) ; confiance : guide lu en entier.
- Deck par défaut : `diana_asc-nalkpas_rq-los-angeles_22nd` ; champion Diana, Lunari ; courbe 1:6, 2:19, 3:8, 4:3, 5:3.
- Plan codé : axe C, style tempo ; garde : Ravenbloom Student, Traveling Merchant ; clés : Ravenbloom Student, Hwei, Brooding Painter.
- Résumé : Rapide : dominer les showdowns tôt (Ravenbloom Student, sorts pas chers payés par la légende), removal contre les grosses unités au milieu, Ride The Wind pour finir (marque aussi au tour adverse).

## Draven, Glorious Executioner
- Source : [riftbound.gg](https://riftbound.gg/draven-glorious-executioner-guide/) (maj 2026-09-14) ; confiance : guide lu en entier.
- Deck par défaut : `draven_dynamic_rq-utrecht_51st` ; champion Draven, Showboat ; courbe 1:6, 2:19, 4:9, 5:3, 6:2.
- Plan codé : axe T, style midrange ; garde : Kai'Sa, Survivor, Vex, Apathetic ; clés : Vex, Apathetic, Kai'Sa, Survivor, Draven, Showboat.
- Résumé : Midrange réactif : Vex, Apathetic et Kai'Sa, Survivor gagnent la partie s'ils restent en vie, les protéger ; le champion est lent (4 points avant d'être une menace). Faible derrière et contre les decks rapides.

## Ezreal, Prodigal Explorer
- Source : [riftbound.gg](https://riftbound.gg/ezreal-prodigal-explorer-guide/) (maj 2026-09-14) ; confiance : guide lu en entier.
- Deck par défaut : `ezreal_bradykin_rq-hartford_3rd` ; champion Ezreal, Prodigy ; courbe 1:13, 2:7, 3:12, 4:5, 7+:2.
- Plan codé : axe T, style controle ; garde : Bewitching Spirit, Fizz, Trickster ; clés : Vex, Apathetic, Thousand-Tailed Watcher.
- Résumé : Contrôle / perturbation : plateau faible, removal en continu, gagner à l'usure quand l'adversaire n'a plus de cartes ; la légende évite de manquer de cartes ; Thousand-Tailed Watcher conclut.

## Fiora, Grand Duelist
- Source : [riftbound.gg](https://riftbound.gg/fiora-grand-duelist-guide/) (maj 2026-09-11) ; confiance : guide d'un autre archétype, plan déduit du deck par défaut.
- Deck par défaut : `fiora_fr0zen_rq-singapore_8th` ; champion Fiora, Worthy ; courbe 1:7, 2:12, 3:3, 5:6, 7+:11.
- Plan codé : axe T, style ramp ; garde : Mobilize, Dazzling Aurora ; clés : Dazzling Aurora, Elder Dragon.
- Résumé : Le deck par défaut est la version Aurora du guide : monter en runes (Mobilize, Catalyst of Aeons), poser Dazzling Aurora qui met en jeu des dragons ; Rengar et Rift Herald entre-temps.

## Garen, Might of Demacia
- Source : [riftbound.gg](https://riftbound.gg/garen-might-of-demacia-guide/) (maj 2026-03-19) ; confiance : guide d'un autre archétype, plan déduit du deck par défaut.
- Deck par défaut : `garen_duality_rq-sydney_288th` ; champion Garen, Commander ; courbe 1:5, 2:14, 4:3, 5:1, 7+:16.
- Plan codé : axe T, style ramp ; garde : Mobilize, Catalyst of Aeons, Dazzling Aurora ; clés : Dazzling Aurora, Elder Dragon, Vanguard Armory.
- Résumé : Le guide (ancien) parle de jetons ; les listes compétitives jouent Dazzling Aurora, comme le deck par défaut : monter en runes, Aurora, dragons, Vanguard Armory pour les jetons. Légende la plus faible selon le guide.

## Irelia, Blade Dancer
- Source : [riftbound.gg](https://riftbound.gg/irelia-blade-dancer-guide/) (maj 2026-09-08) ; confiance : guide lu en entier.
- Deck par défaut : `irelia_tm2_rq-los-angeles_5th` ; champion Irelia, Fervent ; courbe 1:8, 2:18, 3:6, 4:4, 7+:3.
- Plan codé : axe T, style controle ; garde : Stellacorn Herder, Astral Heron ; clés : Stellacorn Herder, Astral Heron, Irelia, Fervent.
- Résumé : Contrôle : se développer sans pousser trop tôt, la légende pousse les points au milieu ; Astral Heron tient en fin de partie ; les autres unités clés rentrent en base en fin de tour ; pas de showdown contre une grosse unité Body / Fury sans réponse.

## Ivern, Green Father
- Source : [mobalytics.gg](https://mobalytics.gg/riftbound/decks/ivern-green-father-barcelona-regional-qualifier-best-of-aserty) (maj 2026-08) ; confiance : résumés de recherche et listes, pas de guide lisible.
- Deck par défaut : `ivern_aserty_rq-barcelona_277th` ; champion Ivern, Nurturer ; courbe 1:6, 2:18, 3:7, 4:2, 5:1, 6:2, 7+:3.
- Plan codé : axe T, style midrange ; garde : Mutated Mouser, Stalwart Poro, Trusty Ramhound ; clés : Ivern, Nurturer.
- Résumé : Pas de guide trouvé. Listes Calm / Order : petites unités (Mutated Mouser, Stalwart Poro, Trusty Ramhound), Defy, Discipline, Friendship ; plan de tenue déduit de la liste.

## Jax, Grandmaster At Arms
- Source : [riftbound.gg](https://riftbound.gg/jax-grandmaster-at-arms-guide/) (maj 2026-02-11) ; confiance : guide lu en entier.
- Deck par défaut : `jax_t1win_rq-los-angeles_162nd` ; champion Jax, Unmatched ; courbe 1:5, 2:20, 3:9, 5:5.
- Plan codé : axe C, style midrange ; garde : Brutalizer, Doran's Blade, Guardian Angel ; clés : Jax, Unmatched.
- Résumé : Équipements : des unités dures à retirer qui provoquent des showdowns ; la légende demande un équipement attaché et deux unités ; protéger les cartes clés avec les sorts Calm.

## Jayce, Defender of Tomorrow
- Source : [riftbound.gg](https://riftbound.gg/jayce-defender-of-tomorrow-guide/) (maj 2026-09-14) ; confiance : guide lu en entier.
- Deck par défaut : `jayce_justmilkey_rq-singapore_35th` ; champion Jayce, Brilliant Inventor ; courbe 1:7, 2:9, 3:3, 4:6, 5:5, 7+:9.
- Plan codé : axe T, style ramp ; garde : Mobilize, Platewyrm Egg, Catalyst of Aeons, Dazzling Aurora ; clés : Elder Dragon, Dazzling Aurora.
- Résumé : Ramp / pioche : mettre une grande menace en jeu tôt pour tenir un battlefield (Dazzling Aurora, Promising Future) ; pas de combos d'équipement (le retrait d'équipement est partout).

## Jhin, Virtuoso
- Source : [riftbound.gg](https://riftbound.gg/jhin-virtuoso-guide/) (maj 2026-07-17) ; confiance : guide lu en entier.
- Deck par défaut : `jhin_ksdden_rq-hartford_29th` ; champion Jhin, Meticulous Killer ; courbe 0:3, 2:4, 3:4, 4:11, 5:8, 6:6, 7+:3.
- Plan codé : axe T, style combo ; garde : Seal of Insight, Energy Conduit, Promising Future, Time Warp ; clés : Energy Conduit.
- Résumé : Combo : tours 1 à 4, tenir, gagner des runes (les sorts de 4 et plus en donnent avec la légende), piocher, retirer des unités, sans surengager ; concentrer le removal sur un battlefield ; Time Warp au bon tour. Ignorer le score adverse tant qu'il n'approche pas 8.

## Jinx, Loose Cannon
- Source : [riftbound.gg](https://riftbound.gg/jinx-loose-cannon-guide/) (maj 2026-02-20) ; confiance : guide lu en entier.
- Deck par défaut : `jinx_htpiper151_rq-sydney_86th` ; champion Jinx, Demolitionist ; courbe 0:3, 1:7, 2:16, 3:10, 4:3.
- Plan codé : axe C, style aggro ; garde : Traveling Merchant, Chemtech Enforcer, Overzealous Fan ; clés : Traveling Merchant.
- Résumé : Aggro défausse : la légende pioche 2 quand la main a 1 carte ou moins, donc défausser sans retenue et vider sa main ; pression constante contre un deck lent ; contre un deck aussi rapide, dépenser moins de cartes.

## Kai'Sa, Daughter of the Void
- Source : [riftbound.gg](https://riftbound.gg/kaisa-daughter-of-the-void-guide/) (maj 2026-07-13) ; confiance : guide lu en entier.
- Deck par défaut : `kaisa_william-n_rq-los-angeles_9th` ; champion Kai'Sa, Survivor ; courbe 1:7, 2:14, 3:4, 4:4, 6:5, 7+:5.
- Plan codé : axe C, style midrange ; garde : Watchful Sentry, Lecturing Yordle, Plundering Poro ; clés : Thousand-Tailed Watcher, Kai'Sa, Survivor.
- Résumé : Midrange proactif : coller des unités au plateau, puis sorts et pioche pour prendre les battlefields ; la légende ignore un coût de puissance par tour, surtout utile une fois une unité posée (removal = conquête).

## Kennen, Heart of the Tempest
- Source : [riftbound.gg](https://riftbound.gg/playing-building-kennen-in-the-post-ban-metagame/) (maj 2026-10-07) ; confiance : guide lu en entier.
- Deck par défaut : `kennen_dsg-awesumrosum_rq-los-angeles_13th` ; champion Kennen, Storm of Shuriken ; courbe 0:3, 1:5, 2:16, 3:9, 4:3, 7+:3.
- Plan codé : axe T, style controle ; garde : Traveling Merchant, Shadow Order Disciple, Lightning Rush ; clés : Rhasa the Sunderer, Kennen, Storm of Shuriken.
- Résumé : Midrange / contrôle : respecter le début adverse, contrôler la vitesse du score, remplir la défausse pour un gros tour (Rhasa the Sunderer) ; ne pas faire la course. Mauvais contre Rengar et Master Yi.

## Kha'Zix, Voidreaver
- Source : [riftbound.gg](https://riftbound.gg/khazix-voidreaver-guide/) (maj 2026-07-05) ; confiance : guide lu en entier.
- Deck par défaut : `khazix_cuoemrei_rq-los-angeles_20th` ; champion Kha'Zix, Mutating Horror ; courbe 1:5, 2:14, 3:13, 4:4, 5:3.
- Plan codé : axe C, style midrange ; garde : Vex, Apathetic, Irresistible Faefolk, Kinkou Initiate ; clés : Vex, Apathetic, Kha'Zix, Mutating Horror.
- Résumé : Midrange Body / Chaos : Vex, Apathetic tôt sur un battlefield, menaces au milieu, [Ambush] pour soutenir les petites unités ; une fois devant, perturbation Chaos.

## LeBlanc, Deceiver
- Source : [riftbound.gg](https://riftbound.gg/leblanc-deceiver-guide/) (maj 2026-07-03) ; confiance : guide lu en entier.
- Deck par défaut : `leblanc_gyatarina_ccs-iq5_1st` ; champion LeBlanc, Fragmented ; courbe 1:3, 2:10, 3:11, 4:1, 5:6, 6:1, 7+:7.
- Plan utilisé : plan existant « Hook tempo » (`plans.py`).
- Résumé : Plan existant Hook tempo (plans.py) gardé. Guide : [Deathknell] d'abord, reflets faits pour mourir, Baited Hook (version occidentale) pour poser plus gros en déclenchant les Deathknell.

## Lee Sin, Blind Monk
- Source : [riftbound.gg](https://riftbound.gg/lee-sin-blind-monk-guide/) (maj 2026-03-19) ; confiance : guide lu en entier.
- Deck par défaut : `leesin_terrythecactus_rq-vancouver_153rd` ; champion Lee Sin, Centered ; courbe 1:11, 2:15, 3:5, 4:3, 5:5.
- Plan codé : axe T, style midrange ; garde : Lonely Poro, Scuttle Crab ; clés : Lee Sin, Centered, Wizened Elder, Irelia, Fervent.
- Résumé : Midrange unités + buffs : empiler les unités sur un seul lieu et les buffer ; la position avant les points tôt ; buffer la plus faible de deux unités pour qu'elles échangent.

## Leona, Radiant Dawn
- Source : [riftbound.gg](https://riftbound.gg/leona-radiant-dawn-guide/) (maj 2026-02-20) ; confiance : guide lu en entier.
- Deck par défaut : `leona_elle-xv_rq-sydney_140th` ; champion Leona, Zealot ; courbe 1:4, 2:14, 3:12, 4:4, 5:5.
- Plan codé : axe T, style midrange ; garde : Nami, Headstrong, Stalwart Poro ; clés : Leona, Zealot, Fiora, Victorious.
- Résumé : Showdowns : étourdir les unités adverses pour buffer les siennes (légende) ; se développer une fois un battlefield contrôlé ; protéger les unités clés. Faiblesse : les unités dans la base adverse.

## Lillia, Bashful Bloom
- Source : [riftbound.gg](https://riftbound.gg/lillia-bashful-bloom-guide/) (maj 2026-07-12) ; confiance : guide lu en entier.
- Deck par défaut : `lillia_duncan_rq-sydney_33rd` ; champion Lillia, Fae Fawn ; courbe 1:7, 2:18, 3:3, 5:3, 7+:8.
- Plan codé : axe T, style controle ; garde : Sprite Fountain, Ravenbloom Student, Plundering Poro ; clés : Thousand-Tailed Watcher.
- Résumé : Valeur : échanger les Sprites temporaires contre les vraies unités adverses, puis les grosses cartes de fin.

## Lucian, Purifier
- Source : [riftbound.gg](https://riftbound.gg/lucian-purifier-guide/) (maj 2026-02-14) ; confiance : guide lu en entier.
- Deck par défaut : `lucian_banrate-demon_rq-barcelona_25th` ; champion Lucian, Merciless ; courbe 1:6, 2:15, 3:7, 4:8, 6:3.
- Plan codé : axe C, style midrange ; garde : Doran's Blade, Long Sword, Kai'Sa, Survivor ; clés : Lucian, Merciless, Kai'Sa, Survivor.
- Résumé : Conquérant à équipements : conquérir chaque tour en gardant ses unités en vie pour reconquérir ; Relentless Pursuit marque aussi au tour adverse.

## Lux, Lady of Luminosity
- Source : [riftbound.gg](https://riftbound.gg/lux-lady-of-luminosity-guide/) (maj 2026-07-13) ; confiance : guide lu en entier.
- Deck par défaut : `lux_ctcg-relivia_rq-hartford_6th` ; champion Lux, Crownguard ; courbe 0:3, 1:7, 2:17, 5:5, 6:3, 7+:4.
- Plan codé : axe T, style combo ; garde : Seal of Insight, Forge of the Future, Ekko, Recurrent, Time Warp ; clés : Ekko, Recurrent.
- Résumé : Combo contrôle : piocher, removal pour ralentir l'adversaire, puis la boucle d'énergie et Time Warp.

## Master Yi, Wuju Bladesman
- Source : [riftbound.gg](https://riftbound.gg/master-yi-wuju-bladesman-guide/) (maj 2026-08-20) ; confiance : guide lu en entier.
- Deck par défaut : `yi-bladesman_swagalisk_rq-la_6th` ; champion Master Yi, Tempered ; courbe 1:14, 2:14, 3:5, 4:2, 5:3, 6:1.
- Plan codé : axe T, style tempo ; garde : Pit Rookie, Scuttle Crab, First Mate ; clés : Master Yi, Tempered, Rengar, Trophy Hunter.
- Résumé : Tenir ou conquérir avec des unités et des sorts pas chers ; la légende donne +2 might au défenseur seul. Tôt : peu de sorts, garder ses runes pour se développer, 1 rune ouverte au tour 3 si besoin ; pas de removal sur les petites unités adverses tôt.

## Master Yi, Wuju Master
- Source : [riftbound.gg](https://riftbound.gg/master-yi-wuju-master-guide/) (maj 2026-07-17) ; confiance : guide lu en entier.
- Deck par défaut : `yi-master_xiaoshenxian_wuhan-open_bestof` ; champion Master Yi, Tempered ; courbe 1:7, 2:17, 3:3, 4:3, 5:6, 7+:3.
- Plan codé : axe C, style midrange ; garde : Wuju Apprentice, Gemhand Hunter, Herald of Spring ; clés : Master Yi, Unstoppable, Voracious Gromp.
- Résumé : Hunt / XP : pas passif tôt, gagner de l'XP vers 6 puis 11 (conquêtes et tenues), puis submerger. Contre l'aggro, retirer les unités même au prix d'XP.

## Mel, Soul's Reflection
- Source : [riftbound.gg](https://riftbound.gg/mel-souls-reflection-guide/) (maj 2026-08-08) ; confiance : guide lu en entier.
- Deck par défaut : `mel_ctg-alanzq_rq-barcelona_19th` ; champion Mel, Defiant Soul ; courbe 1:9, 2:11, 3:7, 4:5, 6:2, 7+:5.
- Plan codé : axe T, style midrange ; garde : Ravenbloom Student, Traveling Merchant ; clés : Applied Researchers, Mel, Defiant Soul.
- Résumé : Midrange valeur : removal par la might, garder quelques runes pour protéger une unité importante sur un battlefield.

## Miss Fortune, Bounty Hunter
- Source : [riftbound.gg](https://riftbound.gg/miss-fortune-bounty-hunter-guide/) (maj 2026-02-14) ; confiance : guide lu en entier.
- Deck par défaut : `miss-fortune_battle-cattle_rq-sydney_35th` ; champion Miss Fortune, Captain ; courbe 1:13, 2:9, 3:7, 4:3, 7+:7.
- Plan codé : axe T, style ramp ; garde : Mobilize, Catalyst of Aeons, Dazzling Aurora ; clés : Dazzling Aurora, Elder Dragon, Miss Fortune, Captain.
- Résumé : Ramp (Dazzling Aurora) : défendre tôt et limiter les points adverses, attaquer tard avec les grosses unités, conquérir les deux battlefields le même tour grâce à [Ganking] (légende).

## Nasus, Curator of the Sands
- Source : [riftbound.gg](https://riftbound.gg/nasus-curator-of-the-sands-guide/) (maj 2026-08-20) ; confiance : guide lu en entier.
- Deck par défaut : `nasus_drinkmaker888_rq-barcelona_80th` ; champion Nasus, Ascended ; courbe 1:14, 2:13, 3:5, 7+:7.
- Plan codé : axe T, style ramp ; garde : Clockwork Keeper, Ravenbloom Student, Find Your Center ; clés : Thousand-Tailed Watcher, Sprite Queen, Nasus, Ascended.
- Résumé : Fin de partie : contester tôt (Ravenbloom Student), monter à 7 runes vers le tour 3, puis grosse unité + sort pas cher (la légende prépare 2 runes). N'a pas peur d'être derrière.

## Ornn, Fire Below the Mountain
- Source : [riftbound.gg](https://riftbound.gg/ornn-fire-below-the-mountain-guide/) (maj 2026-08-30) ; confiance : guide lu en entier.
- Deck par défaut : `ornn_mice-themanland_rq-barcelona_1st` ; champion Ornn, Blacksmith ; courbe 0:3, 1:9, 2:17, 3:9, 4:1.
- Plan codé : axe C, style tempo ; garde : Pit Crew, Seal of Focus, Clockwork Keeper ; clés : Ornn, Blacksmith, Pit Crew.
- Résumé : Équipements, très rapide : utiliser la légende chaque tour pour se développer en gardant des runes, ne pas seulement réagir ; faible contre les adversaires qui ferment le jeu tôt. Vainqueur du RQ de Barcelone.

## Poppy, Keeper of the Hammer
- Source : [mobalytics.gg](https://mobalytics.gg/riftbound/decks/poppy-keeper-of-the-hammer-sydney-regional-qualifier-best-of-rowanno1) (maj date inconnue) ; confiance : résumés de recherche et listes, pas de guide lisible.
- Deck par défaut : `poppy_rowanno1_rq-sydney_21st` ; champion Poppy, Paragon ; courbe 1:7, 2:12, 4:5, 7+:15.
- Plan codé : axe T, style ramp ; garde : Mobilize, Catalyst of Aeons, Dazzling Aurora ; clés : Dazzling Aurora, Elder Dragon, Vanguard Armory.
- Résumé : Pas de guide trouvé. Listes Body / Order Aurora : monter en runes, Dazzling Aurora, dragons, Vanguard Armory.

## Pyke, Bloodharbor Ripper
- Source : [riftbound.gg](https://riftbound.gg/pyke-bloodharbor-ripper-guide/) (maj 2026-07-08) ; confiance : guide lu en entier.
- Deck par défaut : `pyke_royy_rebound-la_2nd` ; champion Pyke, Dockside Butcher ; courbe 1:4, 2:13, 3:14, 4:6, 6:2.
- Plan codé : axe C, style midrange ; garde : Traveling Merchant, Treasure Hunter ; clés : Vex, Apathetic, Ezreal, Prodigy.
- Résumé : Perturbation Chaos + removal Fury + [Ambush] : pousser l'avance tôt ; en partie lente, gagner la guerre de valeur (défausse adverse, renvoi des grosses unités).

## Rek'sai, Void Burrower
- Source : [riftbound.gg](https://riftbound.gg/reksai-void-burrower-guide/) (maj 2026-09-14) ; confiance : guide lu en entier.
- Deck par défaut : `reksai_sashkoenaislabiq_rq-singapore_14th` ; champion Rek'Sai, Breacher ; courbe 1:3, 2:21, 3:1, 4:6, 5:3, 6:5.
- Plan codé : axe C, style aggro ; garde : Carrion Dredger, Mischievous Marai, Noxus Hopeful, Inferna ; clés : Kai'Sa, Survivor, Undertitan.
- Résumé : Conquête : pression tôt avec au moins 9 unités à 2, garder l'adversaire en réaction, removal pour réduire les cibles et conquérir ; peu d'outils pour revenir de derrière.

## Renata Glasc, Chem-Baroness
- Source : [riftbound.gg](https://riftbound.gg/renata-glasc-chem-baroness-guide/) (maj 2026-03-19) ; confiance : guide lu en entier.
- Deck par défaut : `renata-glasc_purp_rq-singapore_311th` ; champion Renata Glasc, Mastermind ; courbe 1:3, 2:20, 3:5, 4:1, 5:5, 6:1, 7+:4.
- Plan codé : axe C, style midrange ; garde : Pit Crew, Plundering Poro, Sprite Fountain, Patched Porobot ; clés : Renata Glasc, Mastermind.
- Résumé : Départ proactif pour atteindre 5 points vite (débloque l'énergie de la légende) ; Hostile Takeover en removal au milieu ; ensuite tours explosifs ou ressources gardées pour réagir.

## Renekton, Butcher of the Sands
- Source : [riftbound.gg](https://riftbound.gg/vendette-pre-rift-decks-for-each-legend/) (maj 2026-07-25) ; confiance : résumés de recherche et listes, pas de guide lisible.
- Deck par défaut : `renekton_maskofmomentum_rq-singapore_384th` ; champion Renekton, Rage Fueled ; courbe 1:6, 2:9, 3:13, 4:6, 5:4, 6:1.
- Plan codé : axe C, style aggro ; garde : First Mate, Inferna, Pit Rookie ; clés : Renekton, Rage Fueled, Rengar, Trophy Hunter.
- Résumé : Pas de guide de tournoi lisible. Légende [Reaction] : énergie pour les unités ; « nourrir le moteur, inonder le plateau » ; une unité au tour 1 puis les grosses.

## Rengar, Pridestalker
- Source : [riftbound.gg](https://riftbound.gg/rengar-pridestalker-guide/) (maj 2026-08-30) ; confiance : guide lu en entier.
- Deck par défaut : `rengar_dsg-prismaticism_rq-los-angeles_1st` ; champion Rengar, Trophy Hunter ; courbe 1:6, 2:11, 3:14, 4:6, 5:1, 6:1.
- Plan codé : axe C, style aggro ; garde : Inferna, Pit Rookie, Irresistible Faefolk, Kinkou Initiate ; clés : Kai'Sa, Survivor, Rengar, Trophy Hunter.
- Résumé : Aggro Body / Fury : prendre l'avance vite et rester devant, gagner les premiers showdowns ; peu de retour possible.

## Rumble, Mechanized Menace
- Source : [riftbound.gg](https://riftbound.gg/rumble-mechanized-menace-guide/) (maj 2026-03-19) ; confiance : guide lu en entier.
- Deck par défaut : `rumble_catmeat_rq-sydney_250th` ; champion Rumble, Hotheaded ; courbe 1:7, 2:12, 3:5, 4:4, 5:3, 6:4, 7+:4.
- Plan codé : axe C, style tempo ; garde : Forecaster, Gem Jammer, Plundering Poro ; clés : Rumble, Hotheaded, Thousand-Tailed Watcher.
- Résumé : Mechs : prendre un battlefield au tour 2 avec un Mech (surtout en commençant), accumuler l'avance de plateau ; plan simple et limité.

## Sett, The Boss
- Source : [riftbound.gg](https://riftbound.gg/sett-the-boss-guide/) (maj 2026-07-07) ; confiance : guide lu en entier.
- Deck par défaut : `sett_ctcg-collin-k_rq-utrecht_3rd` ; champion Sett, Kingpin ; courbe 1:10, 2:11, 3:9, 4:5, 5:4.
- Plan codé : axe C, style aggro ; garde : Fiora, Victorious, Pit Rookie, First Mate, Irresistible Faefolk ; clés : Fiora, Victorious, Sett, Kingpin.
- Résumé : « Marquer à tout prix » : buffs, garder les unités en vie (la légende les sauve du removal), marquer chaque tour ; pas de guerre de valeur.

## Shen, Eye of Twilight
- Source : [riftbound.gg](https://riftbound.gg/vendette-pre-rift-decks-for-each-legend/) (maj 2026-07-25) ; confiance : résumés de recherche et listes, pas de guide lisible.
- Deck par défaut : `shen_sgf-sevya_rq-singapore_450th` ; champion Shen, Scourge of Shadows ; courbe 1:7, 2:18, 3:6, 4:4, 5:2, 7+:2.
- Plan codé : axe T, style midrange ; garde : Lonely Poro, Loyal Poro, Unsung Hero ; clés : Shen, Scourge of Shadows, Fiora, Victorious.
- Résumé : Pas de guide de tournoi lisible. Légende : donner [Tank] à une unité ; contrôle défensif, protéger la meilleure unité, tenir ses battlefields ; le début de partie compte.

## Sivir, Battle Mistress
- Source : [riftbound.gg](https://riftbound.gg/sivir-battle-mistress-guide/) (maj 2026-05-18) ; confiance : guide d'un autre archétype, plan déduit du deck par défaut.
- Deck par défaut : `sivir_randyyy_rq-los-angeles_146th` ; champion Sivir, Mercenary ; courbe 1:7, 2:11, 3:18, 4:3.
- Plan codé : axe C, style tempo ; garde : Traveling Merchant, Shadow Order Disciple, Fizz, Trickster ; clés : Sivir, Mercenary, Ezreal, Prodigy.
- Résumé : Le guide décrit une version Aurora ; le deck par défaut est un tempo Body / Chaos (cartes cachées, Tornado Warrior, Tail-Cloaked Matriarch) : plan de conquérant déduit de la liste.

## Teemo, Swift Scout
- Source : [riftbound.gg](https://riftbound.gg/teemo-swift-scout-guide/) (maj 2026-07-17) ; confiance : guide lu en entier.
- Deck par défaut : `teemo_lunasky_rq-vancouver_64th` ; champion Teemo, Strategist ; courbe 1:6, 2:17, 3:9, 4:6, 6:1.
- Plan codé : axe T, style controle ; garde : Ravenbloom Student, Sprite Fountain, Evelynn, Entrancing ; clés : Teemo, Strategist, Nocturne, Horrifying.
- Résumé : Contrôle à cartes cachées : tenir les battlefields, cacher une carte chaque tour une fois devant, marquer chaque tour et retirer les unités qui menacent un retour ; ne pas s'épuiser trop tôt.

## Vex, Gloomist
- Source : [riftbound.gg](https://riftbound.gg/vex-gloomist-guide/) (maj 2026-09-14) ; confiance : guide lu en entier.
- Deck par défaut : `vex_eep-bonk-repeat_rq-sydney_4th` ; champion Vex, Apathetic ; courbe 1:6, 2:18, 3:9, 4:6.
- Plan codé : axe T, style controle ; garde : Mutated Mouser, Evelynn, Entrancing, Teemo, Scout ; clés : Vex, Apathetic, Evelynn, Entrancing.
- Résumé : Contrôle de tenue : plateau solide, contester un battlefield tôt, puis refuser les retours adverses (stuns, contres) ; Vex, Apathetic à poser tôt et protéger.

## Vi, Piltover Enforcer
- Source : [mobalytics.gg](https://mobalytics.gg/riftbound/decks/vi-piltover-enforcer-sydney-regional-qualifier-best-of-sirieous) (maj date inconnue) ; confiance : résumés de recherche et listes, pas de guide lisible.
- Deck par défaut : `vi_sirieous_rq-sydney_79th` ; champion Vi, Destructive ; courbe 2:17, 3:7, 4:15.
- Plan codé : axe C, style aggro ; garde : Honest Broker, Unsung Hero, Long Sword ; clés : Vi, Destructive.
- Résumé : Pas de guide lisible. Fury / Order agressif : conquérir vite ; la légende prépare une unité quand on tue une unité avec 3 dégâts de trop.

## Viktor, Herald of the Arcane
- Source : [riftbound.gg](https://riftbound.gg/viktor-herald-of-the-arcane-guide/) (maj 2026-07-04) ; confiance : guide lu en entier.
- Deck par défaut : `viktor_rednaxell_rq-utrecht_2nd` ; champion Viktor, Leader ; courbe 1:5, 2:18, 3:9, 5:5, 7+:2.
- Plan codé : axe C, style midrange ; garde : Carrion Dredger, Honest Broker, Xin Zhao, Vigilant ; clés : Viktor, Leader, Xin Zhao, Vigilant.
- Résumé : Hors tempo, jetons : le jeton de la légende conquiert les battlefields vides (pas de showdown), removal pour prendre l'avance, unités qui entrent prêtes pour finir.

## Volibear, Relentless Storm
- Source : [riftbound.gg](https://riftbound.gg/volibear-relentless-storm-guide/) (maj 2026-03-19) ; confiance : guide lu en entier.
- Deck par défaut : `volibear_ctcg-villionaire_rq-utrecht_14th` ; champion Volibear, Furious ; courbe 1:4, 2:8, 4:3, 5:3, 6:5, 7+:16.
- Plan codé : axe T, style ramp ; garde : Mobilize, Catalyst of Aeons, Dazzling Aurora ; clés : Elder Dragon, Kadregrin the Infernal, Gentle Gemdragon, Dazzling Aurora.
- Résumé : Ramp : unités de 5 might et plus (rune en plus avec la légende), garder le score adverse bas tôt, removal seulement si besoin, puis les grosses menaces.

## Yasuo, Unforgiven
- Source : [riftbound.gg](https://riftbound.gg/yasuo-unforgiven-guide/) (maj 2026-04-21) ; confiance : guide lu en entier.
- Deck par défaut : `yasuo_kiiro_rq-vancouver_148th` ; champion Yasuo, Remorseful ; courbe 1:10, 2:16, 3:6, 4:7.
- Plan codé : axe C, style tempo ; garde : Scuttle Crab, Stellacorn Herder, Treasure Hunter ; clés : Stellacorn Herder, Kha'Zix, Mutating Horror, Yasuo, Remorseful.
- Résumé : Déplacer des unités pour marquer chaque tour, protéger celles qui grandissent en bougeant ; la légende traite une unité comme prête (conquérir ou rentrer en base).

## Zed, Master of Shadows
- Source : [riftbound.cardsrealm.com](https://riftbound.cardsrealm.com/en-us/articles/riftbound-review-zeds-kit) (maj avant Vendetta) ; confiance : résumés de recherche et listes, pas de guide lisible.
- Deck par défaut : `zed_ghosterdriver_rq-barcelona_164th` ; champion Zed, Without a Sound ; courbe 0:2, 1:1, 2:13, 3:15, 4:5, 6:3.
- Plan codé : axe C, style tempo ; garde : Traveling Merchant, Death Mark, Noxus Hopeful ; clés : Zed, Without a Sound, Kharox.
- Résumé : Pas de guide de tournoi. Revue d'avant la sortie : jetons Shadow Clone (Death Mark) renforcés en bannissant des unités de la défausse ; Fury / Chaos.

