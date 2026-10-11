"""Plans de jeu par légende (2026-10-11) : un plan pour chacune des 49 légendes des decks par défaut (train.default_decks).

Hypothèses tirées de guides, PAS des vérités : rien n'est mesuré. Activation seulement à la demande :
    exp_search.py <paires> <graine0> "sh@plan=leg" sh defaut      (A = plans par légende, B = IA sans plan)
Sans cette option, rien ne change (train.py et la table n'utilisent pas ce module).

Sources (fiches : ../gameplans/legendes.md, générées par `python3 plans_legendes.py`) :
  - « lu » : guide de la légende sur riftbound.gg (auteur Den), page lue en entier le 2026-10-11 ; codes de cartes
    traduits en noms avec cards_unique.csv et gardés seulement s'ils sont dans le deck par défaut ;
  - « recherche » : pas de guide lisible (sites bloqués), seulement des résumés de recherche et des listes de tournoi ;
  - « deck » : le guide décrit un autre archétype que le deck par défaut ; plan déduit de la liste (cartes, courbe).
Principes communs : « The In-Depth Guide to Scoring in Riftbound » (riftbound.gg, Den, maj 2026-08-01) :
  - conquérant (C) : marquer 2-4-6 puis double conquête, ne pas surengager ;
  - tenant (T) : tenir un battlefield, les unités avant les points, ne pas s'épuiser en runes quand l'adversaire est
    en zone létale (6 points et plus).

Ce que le plan change (PlanAgent : mêmes simulations, seul le score change) :
  - mulligan : ne renvoie jamais une carte « garde » ; renvoie au plus 2 cartes, les plus chères d'abord, au-dessus d'un
    seuil qui dépend du style (aggro 5, tempo 5, midrange 6, controle / ramp / combo 7) ; aggro et tempo renvoient aussi
    les cartes de 4 et plus sans unité de coût 3 ou moins en main ;
  - shape (fin de simulation, unités d'évaluation, 7 ≈ 1 point) :
      C : W_CONQ × might de mes unités sur les battlefields ;
      T : W_HOLD par battlefield tenu (en plus de ai.EV["bf"]) ; si l'adversaire est à 2 points de la victoire ou moins,
          W_OPEN par rune prête (2 au plus) ;
      cartes « cles » en jeu sous mon contrôle : W_KEY chacune (les garder en vie, les poser) ;
      controle et combo : W_HAND par carte en main.
Poids choisis sans mesure (RB_LEG_WC, RB_LEG_WT, RB_LEG_WO, RB_LEG_WK, RB_LEG_WH). Akali et LeBlanc gardent leurs
plans (Gorica, Hook tempo), écrits et mesurés avant. Aucun ordre de set ne compte ici.
"""
import os
from plans import Plan, PLANS

W_CONQ = float(os.environ.get("RB_LEG_WC", "0.3"))
W_HOLD = float(os.environ.get("RB_LEG_WT", "1.5"))
W_OPEN = float(os.environ.get("RB_LEG_WO", "0.5"))
W_KEY = float(os.environ.get("RB_LEG_WK", "1.0"))
W_HAND = float(os.environ.get("RB_LEG_WH", "0.3"))

SEUIL = {"aggro": 5, "tempo": 5, "midrange": 6, "controle": 7, "ramp": 7, "combo": 7}
GG = "https://riftbound.gg/"
SCORE = GG + "the-in-depth-guide-to-scoring-in-riftbound/"
AURORA = ("Mobilize", "Catalyst of Aeons", "Dazzling Aurora")


def L(axe, style, garde, cles, conf, src, maj, note):
    return dict(axe=axe, style=style, garde=tuple(garde), cles=tuple(cles), conf=conf, src=src, maj=maj, note=note)


# légende : axe (C / T), style, garde (mulligan), cles (en jeu), confiance, source, mise à jour de la source, résumé
LEG = {
    "Ahri, Nine-Tailed Fox": L(
        "T", "controle", ["Ravenbloom Student", "Sprite Fountain"],
        ["Ravenbloom Student", "Thousand-Tailed Watcher", "Ahri, Inquisitive"], "lu",
        GG + "ahri-nine-tailed-fox-guide/", "2026-07-17",
        "Jeu de tenue : ancre tôt (Ravenbloom Student), garder une rune ouverte pour réagir aux conquêtes, Sprite pour "
        "tenir ; Thousand-Tailed Watcher conquiert en fin de partie. Le 8e point se prend plutôt en conquérant dans son "
        "tour qu'en tenant (une tenue rend souvent un tour à l'adversaire)."),
    "Akali, Rogue Assassin": L(
        "T", "controle", ["Stellacorn Herder", "Astral Heron"], ["Stellacorn Herder", "Astral Heron", "Kai'Sa, Survivor"],
        "lu", GG + "akali-rogue-assassin-guide/", "2026-09-14",
        "Plan existant Gorica (plans.py) gardé. Guide : removal efficace, contrôle du plateau, deux moteurs de pioche à "
        "protéger (Stellacorn Herder, Kai'Sa, Survivor), poser Akali (fragile) seulement sur un plateau sûr ; "
        "Astral Heron ancre la fin de partie."),
    "Ambessa, Matriarch of War": L(
        "C", "midrange", ["Legion Marauder", "Irresistible Faefolk"], ["Legion Marauder", "Ambessa, The Wolf"], "lu",
        GG + "ambessa-matriarch-of-war-guide/", "2026-08-14",
        "Plateau : prendre le plateau tôt et forcer l'adversaire à défendre ; la légende prépare une unité (Empower) pour "
        "marquer ; removal en base. Légende faible en partie longue : recycler les runes sans hésiter en fin de partie."),
    "Annie, Dark Child": L(
        "C", "aggro", ["Traveling Merchant", "Inferna", "Rengar, Pouncing"], ["Traveling Merchant", "Kai'Sa, Survivor"],
        "lu", GG + "annie-dark-child-guide/", "2026-07-03",
        "Aggro : conquérir chaque tour jusqu'à 8. La légende prépare 2 runes en fin de tour : tout dépenser dans son tour "
        "et garder ces 2 runes pour réagir. Pioche indispensable (Traveling Merchant, Kai'Sa) ; les attaquants [Assault] "
        "tiennent mal ; Vex et Star-Crossed contre une grosse unité adverse."),
    "Azir, Emperor of the Sands": L(
        "T", "controle", ["Doran's Shield", "Soul Sword", "Eye of the Herald"], ["Azir, Sovereign"], "lu",
        GG + "azir-emperor-of-the-sands-guide/", "2026-09-08",
        "Contrôle : 13 équipements bon marché pour créer des Sand Soldiers ; Arise! (sort signature) finit la partie. "
        "Contre un deck lent : poser chaque tour et marquer ; contre un deck rapide : valeur, removal."),
    "Darius, Hand of Noxus": L(
        "C", "aggro", ["Baited Hook", "Carrion Dredger", "Noxus Hopeful", "Honest Broker"], ["Baited Hook"], "lu",
        GG + "darius-hand-of-noxus-guide/", "2026-07-06",
        "Aggro / tempo pressé : développer au maximum tôt (Legion plus fort tôt), Baited Hook sacrifie une petite unité "
        "pour en poser une plus grosse en base ; removal Order. Peu de retour possible : ne pas prendre de retard."),
    "Diana, Scorn of the Moon": L(
        "C", "tempo", ["Ravenbloom Student", "Traveling Merchant"], ["Ravenbloom Student", "Hwei, Brooding Painter"],
        "lu", GG + "diana-scorn-of-the-moon-guide/", "2026-09-14",
        "Rapide : dominer les showdowns tôt (Ravenbloom Student, sorts pas chers payés par la légende), removal contre "
        "les grosses unités au milieu, Ride The Wind pour finir (marque aussi au tour adverse)."),
    "Draven, Glorious Executioner": L(
        "T", "midrange", ["Kai'Sa, Survivor", "Vex, Apathetic"], ["Vex, Apathetic", "Kai'Sa, Survivor", "Draven, Showboat"],
        "lu", GG + "draven-glorious-executioner-guide/", "2026-09-14",
        "Midrange réactif : Vex, Apathetic et Kai'Sa, Survivor gagnent la partie s'ils restent en vie, les protéger ; "
        "le champion est lent (4 points avant d'être une menace). Faible derrière et contre les decks rapides."),
    "Ezreal, Prodigal Explorer": L(
        "T", "controle", ["Bewitching Spirit", "Fizz, Trickster"], ["Vex, Apathetic", "Thousand-Tailed Watcher"], "lu",
        GG + "ezreal-prodigal-explorer-guide/", "2026-09-14",
        "Contrôle / perturbation : plateau faible, removal en continu, gagner à l'usure quand l'adversaire n'a plus de "
        "cartes ; la légende évite de manquer de cartes ; Thousand-Tailed Watcher conclut."),
    "Fiora, Grand Duelist": L(
        "T", "ramp", ["Mobilize", "Dazzling Aurora"], ["Dazzling Aurora", "Elder Dragon"], "deck",
        GG + "fiora-grand-duelist-guide/", "2026-09-11",
        "Le deck par défaut est la version Aurora du guide : monter en runes (Mobilize, Catalyst of Aeons), poser "
        "Dazzling Aurora qui met en jeu des dragons ; Rengar et Rift Herald entre-temps."),
    "Garen, Might of Demacia": L(
        "T", "ramp", list(AURORA), ["Dazzling Aurora", "Elder Dragon", "Vanguard Armory"], "deck",
        GG + "garen-might-of-demacia-guide/", "2026-03-19",
        "Le guide (ancien) parle de jetons ; les listes compétitives jouent Dazzling Aurora, comme le deck par défaut : "
        "monter en runes, Aurora, dragons, Vanguard Armory pour les jetons. Légende la plus faible selon le guide."),
    "Irelia, Blade Dancer": L(
        "T", "controle", ["Stellacorn Herder", "Astral Heron"], ["Stellacorn Herder", "Astral Heron", "Irelia, Fervent"],
        "lu", GG + "irelia-blade-dancer-guide/", "2026-09-08",
        "Contrôle : se développer sans pousser trop tôt, la légende pousse les points au milieu ; Astral Heron tient en "
        "fin de partie ; les autres unités clés rentrent en base en fin de tour ; pas de showdown contre une grosse unité "
        "Body / Fury sans réponse."),
    "Ivern, Green Father": L(
        "T", "midrange", ["Mutated Mouser", "Stalwart Poro", "Trusty Ramhound"], ["Ivern, Nurturer"], "recherche",
        "https://mobalytics.gg/riftbound/decks/ivern-green-father-barcelona-regional-qualifier-best-of-aserty", "2026-08",
        "Pas de guide trouvé. Listes Calm / Order : petites unités (Mutated Mouser, Stalwart Poro, Trusty Ramhound), "
        "Defy, Discipline, Friendship ; plan de tenue déduit de la liste."),
    "Jax, Grandmaster At Arms": L(
        "C", "midrange", ["Brutalizer", "Doran's Blade", "Guardian Angel"], ["Jax, Unmatched"], "lu",
        GG + "jax-grandmaster-at-arms-guide/", "2026-02-11",
        "Équipements : des unités dures à retirer qui provoquent des showdowns ; la légende demande un équipement attaché "
        "et deux unités ; protéger les cartes clés avec les sorts Calm."),
    "Jayce, Defender of Tomorrow": L(
        "T", "ramp", ["Mobilize", "Platewyrm Egg", "Catalyst of Aeons", "Dazzling Aurora"], ["Elder Dragon", "Dazzling Aurora"],
        "lu", GG + "jayce-defender-of-tomorrow-guide/", "2026-09-14",
        "Ramp / pioche : mettre une grande menace en jeu tôt pour tenir un battlefield (Dazzling Aurora, Promising Future) ; "
        "pas de combos d'équipement (le retrait d'équipement est partout)."),
    "Jhin, Virtuoso": L(
        "T", "combo", ["Seal of Insight", "Energy Conduit", "Promising Future", "Time Warp"], ["Energy Conduit"], "lu",
        GG + "jhin-virtuoso-guide/", "2026-07-17",
        "Combo : tours 1 à 4, tenir, gagner des runes (les sorts de 4 et plus en donnent avec la légende), piocher, "
        "retirer des unités, sans surengager ; concentrer le removal sur un battlefield ; Time Warp au bon tour. Ignorer "
        "le score adverse tant qu'il n'approche pas 8."),
    "Jinx, Loose Cannon": L(
        "C", "aggro", ["Traveling Merchant", "Chemtech Enforcer", "Overzealous Fan"], ["Traveling Merchant"], "lu",
        GG + "jinx-loose-cannon-guide/", "2026-02-20",
        "Aggro défausse : la légende pioche 2 quand la main a 1 carte ou moins, donc défausser sans retenue et vider sa "
        "main ; pression constante contre un deck lent ; contre un deck aussi rapide, dépenser moins de cartes."),
    "Kai'Sa, Daughter of the Void": L(
        "C", "midrange", ["Watchful Sentry", "Lecturing Yordle", "Plundering Poro"],
        ["Thousand-Tailed Watcher", "Kai'Sa, Survivor"], "lu", GG + "kaisa-daughter-of-the-void-guide/", "2026-07-13",
        "Midrange proactif : coller des unités au plateau, puis sorts et pioche pour prendre les battlefields ; la "
        "légende ignore un coût de puissance par tour, surtout utile une fois une unité posée (removal = conquête)."),
    "Kennen, Heart of the Tempest": L(
        "T", "controle", ["Traveling Merchant", "Shadow Order Disciple", "Lightning Rush"],
        ["Rhasa the Sunderer", "Kennen, Storm of Shuriken"], "lu",
        GG + "playing-building-kennen-in-the-post-ban-metagame/", "2026-10-07",
        "Midrange / contrôle : respecter le début adverse, contrôler la vitesse du score, remplir la défausse pour un "
        "gros tour (Rhasa the Sunderer) ; ne pas faire la course. Mauvais contre Rengar et Master Yi."),
    "Kha'Zix, Voidreaver": L(
        "C", "midrange", ["Vex, Apathetic", "Irresistible Faefolk", "Kinkou Initiate"],
        ["Vex, Apathetic", "Kha'Zix, Mutating Horror"], "lu", GG + "khazix-voidreaver-guide/", "2026-07-05",
        "Midrange Body / Chaos : Vex, Apathetic tôt sur un battlefield, menaces au milieu, [Ambush] pour soutenir les "
        "petites unités ; une fois devant, perturbation Chaos."),
    "LeBlanc, Deceiver": L(
        "C", "tempo", ["Soaring Scout", "Glasc Mixologist"], ["LeBlanc, Fragmented", "Karthus, Eternal"], "lu",
        GG + "leblanc-deceiver-guide/", "2026-07-03",
        "Plan existant Hook tempo (plans.py) gardé. Guide : [Deathknell] d'abord, reflets faits pour mourir, Baited Hook "
        "(version occidentale) pour poser plus gros en déclenchant les Deathknell."),
    "Lee Sin, Blind Monk": L(
        "T", "midrange", ["Lonely Poro", "Scuttle Crab"], ["Lee Sin, Centered", "Wizened Elder", "Irelia, Fervent"], "lu",
        GG + "lee-sin-blind-monk-guide/", "2026-03-19",
        "Midrange unités + buffs : empiler les unités sur un seul lieu et les buffer ; la position avant les points tôt ; "
        "buffer la plus faible de deux unités pour qu'elles échangent."),
    "Leona, Radiant Dawn": L(
        "T", "midrange", ["Nami, Headstrong", "Stalwart Poro"], ["Leona, Zealot", "Fiora, Victorious"], "lu",
        GG + "leona-radiant-dawn-guide/", "2026-02-20",
        "Showdowns : étourdir les unités adverses pour buffer les siennes (légende) ; se développer une fois un "
        "battlefield contrôlé ; protéger les unités clés. Faiblesse : les unités dans la base adverse."),
    "Lillia, Bashful Bloom": L(
        "T", "controle", ["Sprite Fountain", "Ravenbloom Student", "Plundering Poro"], ["Thousand-Tailed Watcher"], "lu",
        GG + "lillia-bashful-bloom-guide/", "2026-07-12",
        "Valeur : échanger les Sprites temporaires contre les vraies unités adverses, puis les grosses cartes de fin."),
    "Lucian, Purifier": L(
        "C", "midrange", ["Doran's Blade", "Long Sword", "Kai'Sa, Survivor"], ["Lucian, Merciless", "Kai'Sa, Survivor"],
        "lu", GG + "lucian-purifier-guide/", "2026-02-14",
        "Conquérant à équipements : conquérir chaque tour en gardant ses unités en vie pour reconquérir ; "
        "Relentless Pursuit marque aussi au tour adverse."),
    "Lux, Lady of Luminosity": L(
        "T", "combo", ["Seal of Insight", "Forge of the Future", "Ekko, Recurrent", "Time Warp"], ["Ekko, Recurrent"], "lu",
        GG + "lux-lady-of-luminosity-guide/", "2026-07-13",
        "Combo contrôle : piocher, removal pour ralentir l'adversaire, puis la boucle d'énergie et Time Warp."),
    "Master Yi, Wuju Bladesman": L(
        "T", "tempo", ["Pit Rookie", "Scuttle Crab", "First Mate"], ["Master Yi, Tempered", "Rengar, Trophy Hunter"], "lu",
        GG + "master-yi-wuju-bladesman-guide/", "2026-08-20",
        "Tenir ou conquérir avec des unités et des sorts pas chers ; la légende donne +2 might au défenseur seul. Tôt : "
        "peu de sorts, garder ses runes pour se développer, 1 rune ouverte au tour 3 si besoin ; pas de removal sur les "
        "petites unités adverses tôt."),
    "Master Yi, Wuju Master": L(
        "C", "midrange", ["Wuju Apprentice", "Gemhand Hunter", "Herald of Spring"],
        ["Master Yi, Unstoppable", "Voracious Gromp"], "lu", GG + "master-yi-wuju-master-guide/", "2026-07-17",
        "Hunt / XP : pas passif tôt, gagner de l'XP vers 6 puis 11 (conquêtes et tenues), puis submerger. Contre "
        "l'aggro, retirer les unités même au prix d'XP."),
    "Mel, Soul's Reflection": L(
        "T", "midrange", ["Ravenbloom Student", "Traveling Merchant"], ["Applied Researchers", "Mel, Defiant Soul"], "lu",
        GG + "mel-souls-reflection-guide/", "2026-08-08",
        "Midrange valeur : removal par la might, garder quelques runes pour protéger une unité importante sur un "
        "battlefield."),
    "Miss Fortune, Bounty Hunter": L(
        "T", "ramp", list(AURORA), ["Dazzling Aurora", "Elder Dragon", "Miss Fortune, Captain"], "lu",
        GG + "miss-fortune-bounty-hunter-guide/", "2026-02-14",
        "Ramp (Dazzling Aurora) : défendre tôt et limiter les points adverses, attaquer tard avec les grosses unités, "
        "conquérir les deux battlefields le même tour grâce à [Ganking] (légende)."),
    "Nasus, Curator of the Sands": L(
        "T", "ramp", ["Clockwork Keeper", "Ravenbloom Student", "Find Your Center"],
        ["Thousand-Tailed Watcher", "Sprite Queen", "Nasus, Ascended"], "lu",
        GG + "nasus-curator-of-the-sands-guide/", "2026-08-20",
        "Fin de partie : contester tôt (Ravenbloom Student), monter à 7 runes vers le tour 3, puis grosse unité + sort "
        "pas cher (la légende prépare 2 runes). N'a pas peur d'être derrière."),
    "Ornn, Fire Below the Mountain": L(
        "C", "tempo", ["Pit Crew", "Seal of Focus", "Clockwork Keeper"], ["Ornn, Blacksmith", "Pit Crew"], "lu",
        GG + "ornn-fire-below-the-mountain-guide/", "2026-08-30",
        "Équipements, très rapide : utiliser la légende chaque tour pour se développer en gardant des runes, ne pas "
        "seulement réagir ; faible contre les adversaires qui ferment le jeu tôt. Vainqueur du RQ de Barcelone."),
    "Poppy, Keeper of the Hammer": L(
        "T", "ramp", list(AURORA), ["Dazzling Aurora", "Elder Dragon", "Vanguard Armory"], "recherche",
        "https://mobalytics.gg/riftbound/decks/poppy-keeper-of-the-hammer-sydney-regional-qualifier-best-of-rowanno1",
        "date inconnue",
        "Pas de guide trouvé. Listes Body / Order Aurora : monter en runes, Dazzling Aurora, dragons, Vanguard Armory."),
    "Pyke, Bloodharbor Ripper": L(
        "C", "midrange", ["Traveling Merchant", "Treasure Hunter"], ["Vex, Apathetic", "Ezreal, Prodigy"], "lu",
        GG + "pyke-bloodharbor-ripper-guide/", "2026-07-08",
        "Perturbation Chaos + removal Fury + [Ambush] : pousser l'avance tôt ; en partie lente, gagner la guerre de "
        "valeur (défausse adverse, renvoi des grosses unités)."),
    "Rek'sai, Void Burrower": L(
        "C", "aggro", ["Carrion Dredger", "Mischievous Marai", "Noxus Hopeful", "Inferna"], ["Kai'Sa, Survivor", "Undertitan"],
        "lu", GG + "reksai-void-burrower-guide/", "2026-09-14",
        "Conquête : pression tôt avec au moins 9 unités à 2, garder l'adversaire en réaction, removal pour réduire les "
        "cibles et conquérir ; peu d'outils pour revenir de derrière."),
    "Renata Glasc, Chem-Baroness": L(
        "C", "midrange", ["Pit Crew", "Plundering Poro", "Sprite Fountain", "Patched Porobot"], ["Renata Glasc, Mastermind"],
        "lu", GG + "renata-glasc-chem-baroness-guide/", "2026-03-19",
        "Départ proactif pour atteindre 5 points vite (débloque l'énergie de la légende) ; Hostile Takeover en removal au "
        "milieu ; ensuite tours explosifs ou ressources gardées pour réagir."),
    "Renekton, Butcher of the Sands": L(
        "C", "aggro", ["First Mate", "Inferna", "Pit Rookie"], ["Renekton, Rage Fueled", "Rengar, Trophy Hunter"],
        "recherche", GG + "vendette-pre-rift-decks-for-each-legend/", "2026-07-25",
        "Pas de guide de tournoi lisible. Légende [Reaction] : énergie pour les unités ; « nourrir le moteur, inonder le "
        "plateau » ; une unité au tour 1 puis les grosses."),
    "Rengar, Pridestalker": L(
        "C", "aggro", ["Inferna", "Pit Rookie", "Irresistible Faefolk", "Kinkou Initiate"],
        ["Kai'Sa, Survivor", "Rengar, Trophy Hunter"], "lu", GG + "rengar-pridestalker-guide/", "2026-08-30",
        "Aggro Body / Fury : prendre l'avance vite et rester devant, gagner les premiers showdowns ; peu de retour "
        "possible."),
    "Rumble, Mechanized Menace": L(
        "C", "tempo", ["Forecaster", "Gem Jammer", "Plundering Poro"], ["Rumble, Hotheaded", "Thousand-Tailed Watcher"],
        "lu", GG + "rumble-mechanized-menace-guide/", "2026-03-19",
        "Mechs : prendre un battlefield au tour 2 avec un Mech (surtout en commençant), accumuler l'avance de plateau ; "
        "plan simple et limité."),
    "Sett, The Boss": L(
        "C", "aggro", ["Fiora, Victorious", "Pit Rookie", "First Mate", "Irresistible Faefolk"],
        ["Fiora, Victorious", "Sett, Kingpin"], "lu", GG + "sett-the-boss-guide/", "2026-07-07",
        "« Marquer à tout prix » : buffs, garder les unités en vie (la légende les sauve du removal), marquer chaque "
        "tour ; pas de guerre de valeur."),
    "Shen, Eye of Twilight": L(
        "T", "midrange", ["Lonely Poro", "Loyal Poro", "Unsung Hero"], ["Shen, Scourge of Shadows", "Fiora, Victorious"],
        "recherche", GG + "vendette-pre-rift-decks-for-each-legend/", "2026-07-25",
        "Pas de guide de tournoi lisible. Légende : donner [Tank] à une unité ; contrôle défensif, protéger la meilleure "
        "unité, tenir ses battlefields ; le début de partie compte."),
    "Sivir, Battle Mistress": L(
        "C", "tempo", ["Traveling Merchant", "Shadow Order Disciple", "Fizz, Trickster"], ["Sivir, Mercenary", "Ezreal, Prodigy"],
        "deck", GG + "sivir-battle-mistress-guide/", "2026-05-18",
        "Le guide décrit une version Aurora ; le deck par défaut est un tempo Body / Chaos (cartes cachées, Tornado "
        "Warrior, Tail-Cloaked Matriarch) : plan de conquérant déduit de la liste."),
    "Teemo, Swift Scout": L(
        "T", "controle", ["Ravenbloom Student", "Sprite Fountain", "Evelynn, Entrancing"],
        ["Teemo, Strategist", "Nocturne, Horrifying"], "lu", GG + "teemo-swift-scout-guide/", "2026-07-17",
        "Contrôle à cartes cachées : tenir les battlefields, cacher une carte chaque tour une fois devant, marquer chaque "
        "tour et retirer les unités qui menacent un retour ; ne pas s'épuiser trop tôt."),
    "Vex, Gloomist": L(
        "T", "controle", ["Mutated Mouser", "Evelynn, Entrancing", "Teemo, Scout"], ["Vex, Apathetic", "Evelynn, Entrancing"],
        "lu", GG + "vex-gloomist-guide/", "2026-09-14",
        "Contrôle de tenue : plateau solide, contester un battlefield tôt, puis refuser les retours adverses (stuns, "
        "contres) ; Vex, Apathetic à poser tôt et protéger."),
    "Vi, Piltover Enforcer": L(
        "C", "aggro", ["Honest Broker", "Unsung Hero", "Long Sword"], ["Vi, Destructive"], "recherche",
        "https://mobalytics.gg/riftbound/decks/vi-piltover-enforcer-sydney-regional-qualifier-best-of-sirieous", "date inconnue",
        "Pas de guide lisible. Fury / Order agressif : conquérir vite ; la légende prépare une unité quand on tue une "
        "unité avec 3 dégâts de trop."),
    "Viktor, Herald of the Arcane": L(
        "C", "midrange", ["Carrion Dredger", "Honest Broker", "Xin Zhao, Vigilant"], ["Viktor, Leader", "Xin Zhao, Vigilant"],
        "lu", GG + "viktor-herald-of-the-arcane-guide/", "2026-07-04",
        "Hors tempo, jetons : le jeton de la légende conquiert les battlefields vides (pas de showdown), removal pour "
        "prendre l'avance, unités qui entrent prêtes pour finir."),
    "Volibear, Relentless Storm": L(
        "T", "ramp", list(AURORA), ["Elder Dragon", "Kadregrin the Infernal", "Gentle Gemdragon", "Dazzling Aurora"], "lu",
        GG + "volibear-relentless-storm-guide/", "2026-03-19",
        "Ramp : unités de 5 might et plus (rune en plus avec la légende), garder le score adverse bas tôt, removal "
        "seulement si besoin, puis les grosses menaces."),
    "Yasuo, Unforgiven": L(
        "C", "tempo", ["Scuttle Crab", "Stellacorn Herder", "Treasure Hunter"],
        ["Stellacorn Herder", "Kha'Zix, Mutating Horror", "Yasuo, Remorseful"], "lu", GG + "yasuo-unforgiven-guide/",
        "2026-04-21",
        "Déplacer des unités pour marquer chaque tour, protéger celles qui grandissent en bougeant ; la légende traite une "
        "unité comme prête (conquérir ou rentrer en base)."),
    "Zed, Master of Shadows": L(
        "C", "tempo", ["Traveling Merchant", "Death Mark", "Noxus Hopeful"], ["Zed, Without a Sound", "Kharox"], "recherche",
        "https://riftbound.cardsrealm.com/en-us/articles/riftbound-review-zeds-kit", "avant Vendetta",
        "Pas de guide de tournoi. Revue d'avant la sortie : jetons Shadow Clone (Death Mark) renforcés en bannissant des "
        "unités de la défausse ; Fury / Chaos."),
}

EXISTING = {"Akali, Rogue Assassin": "Gorica", "LeBlanc, Deceiver": "Hook tempo"}


class LegendPlan(Plan):
    name = "legende"

    def __init__(s, legend):
        s.legend = legend
        s.d = LEG[legend]
        s.garde = frozenset(s.d["garde"])
        s.cles = frozenset(s.d["cles"])

    def mulligan(s, g, pid):
        h = g.p[pid].hand
        st = s.d["style"]
        lim = SEUIL[st]
        cand = sorted((c for c in h if c.cname not in s.garde), key=lambda c: (-(c.spec["e"] + c.spec["p"]), c.uid))
        out = [c for c in cand if c.spec["e"] >= lim]
        if st in ("aggro", "tempo") and not any(c.spec["type"] == "Unit" and c.spec["e"] <= 3 for c in h):
            out += [c for c in cand if c not in out and c.spec["e"] >= 4]
        return out[:2]

    def shape(s, g, me):
        if g.winner is not None:
            return 0.0
        d = s.d
        v = 0.0
        if d["axe"] == "C":
            v += W_CONQ * sum(max(0, g.might(u)) for u in g.units(me) if u.loc in (0, 1))
        else:
            v += W_HOLD * sum(1 for b in g.bfs if b.ctrl == me)
            if g.p[1 - me].points >= g.victory - 2:
                v += W_OPEN * min(2, sum(1 for r in g.p[me].runes if not r.exhausted))
        if s.cles:
            v += W_KEY * sum(1 for o in g.board if o.ctrl == me and o.cname in s.cles)
        if d["style"] in ("controle", "combo"):
            v += W_HAND * len(g.p[me].hand)
        return v


def plan_for(deck):
    """Plan de la légende du deck : plan existant (Akali, LeBlanc), plan par légende, sinon aucun plan."""
    lg = deck.get("legend") or ""
    if lg in EXISTING:
        return PLANS[EXISTING[lg]]()
    if lg in LEG:
        return LegendPlan(lg)
    return Plan()


def fiches():
    """Texte de ../gameplans/legendes.md (fiches générées depuis LEG : une seule source)."""
    import train as T
    from game import SPEC
    from collections import Counter
    dd = T.default_decks()
    conf = {"lu": "guide lu en entier", "recherche": "résumés de recherche et listes, pas de guide lisible",
            "deck": "guide d'un autre archétype, plan déduit du deck par défaut"}
    n = Counter(LEG[k]["conf"] for k in LEG)
    out = ["---", "titre: Plans de jeu par légende", "resume: un plan sourcé pour chaque légende des decks par défaut, "
           "hypothèses non mesurées, codées dans engine/plans_legendes.py", "maj: 2026-10-11",
           "sources: riftbound.gg (guides de Den), mobalytics, cardsrealm ; voir chaque fiche", "---",
           "# Plans de jeu par légende (49 decks par défaut)", "",
           "Fichier généré par `python3 riftbound/engine/plans_legendes.py` depuis la table `LEG` : ne pas l'éditer à la main.",
           "", "**Ce sont des hypothèses, pas des résultats.** Aucun plan n'est mesuré. Mesure proposée (en local) :",
           "`python3 exp_search.py 80 <graines neuves> \"sh@plan=leg\" sh defaut` (A = plans par légende, B = IA sans plan),",
           "puis confirmation sur d'autres graines neuves ; « on ne sait pas » sous 2 écarts-types.", "",
           f"Confiance : {n['lu']} guides lus en entier, {n['deck']} plans déduits du deck (guide d'un autre archétype), "
           f"{n['recherche']} sans guide lisible (sites autres que riftbound.gg bloqués le 2026-10-11).",
           "Principes communs : [guide du score]({}) (conquérant / tenant, zone létale à 6 points).".format(SCORE), "",
           "Ce que l'IA en fait : `axe` C (conquérant : valorise la might sur les battlefields) ou T (tenant : valorise "
           "les battlefields tenus et les runes prêtes quand l'adversaire approche de la victoire) ; `garde` = jamais "
           "renvoyées au mulligan ; `cles` = valorisées en jeu ; contrôle et combo valorisent la main.", ""]
    for lg in sorted(LEG):
        d = LEG[lg]
        deck = T.load(dd[lg]) if lg in dd else None
        out.append(f"## {lg}")
        out.append(f"- Source : [{d['src'].split('/')[2]}]({d['src']}) (maj {d['maj']}) ; confiance : {conf[d['conf']]}.")
        if deck:
            curve = Counter(min(SPEC[x]["e"], 7) for x in deck["main"])
            out.append(f"- Deck par défaut : `{dd[lg]}` ; champion {deck.get('champion')} ; courbe "
                       + ", ".join(f"{k}{'+' if k == 7 else ''}:{v}" for k, v in sorted(curve.items())) + ".")
        if lg in EXISTING:
            out.append(f"- Plan utilisé : plan existant « {EXISTING[lg]} » (`plans.py`).")
        else:
            out.append(f"- Plan codé : axe {d['axe']}, style {d['style']} ; garde : {', '.join(d['garde'])} ; "
                       f"clés : {', '.join(d['cles'])}.")
        out.append(f"- Résumé : {d['note']}")
        out.append("")
    return "\n".join(out)


if __name__ == "__main__":
    from pathlib import Path
    p = Path(__file__).resolve().parents[1] / "gameplans" / "legendes.md"
    p.write_text(fiches() + "\n", encoding="utf-8")
    print(f"{p} : {len(LEG)} légendes")
