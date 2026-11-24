//! Plans de jeu par légende (`plans_legendes.py`, 2026-10-11). Hypothèses tirées de guides, PAS des vérités : rien n'est
//! mesuré. Activation seulement à la demande (`exp_search.py <paires> <graine0> "sh@plan=leg" sh defaut`) ; la table
//! n'utilise pas ce module. La table `LEG` est écrite dans le code Python (aucun fichier lu) : elle est recopiée ici
//! à l'identique (générée depuis `plans_legendes.LEG`). `fiches()` (génération de `gameplans/legendes.md`, outil de
//! documentation) n'est pas portée.
//!
//! Ce que le plan change : mulligan (jamais une carte « garde », au plus 2 cartes, les plus chères d'abord, au-dessus
//! d'un seuil selon le style) et shape (C : might sur les battlefields ; T : battlefields tenus, runes prêtes quand
//! l'adversaire approche de la victoire ; cartes « cles » en jeu ; contrôle et combo : la main).
use std::sync::{Arc, OnceLock};

use crate::ai::env_or;
use crate::game::{DeckList, Game, Res, O};
use crate::plans::{plans, NoPlan, Plan};
use crate::value::is_bf;

fn w(name: &str, default: &str) -> f64 {
    env_or(name, default).trim().parse().unwrap_or_else(|_| panic!("{name}"))
}

/// `W_CONQ` (RB_LEG_WC)
pub fn w_conq() -> f64 {
    static V: OnceLock<f64> = OnceLock::new();
    *V.get_or_init(|| w("RB_LEG_WC", "0.3"))
}
/// `W_HOLD` (RB_LEG_WT)
pub fn w_hold() -> f64 {
    static V: OnceLock<f64> = OnceLock::new();
    *V.get_or_init(|| w("RB_LEG_WT", "1.5"))
}
/// `W_OPEN` (RB_LEG_WO)
pub fn w_open() -> f64 {
    static V: OnceLock<f64> = OnceLock::new();
    *V.get_or_init(|| w("RB_LEG_WO", "0.5"))
}
/// `W_KEY` (RB_LEG_WK)
pub fn w_key() -> f64 {
    static V: OnceLock<f64> = OnceLock::new();
    *V.get_or_init(|| w("RB_LEG_WK", "1.0"))
}
/// `W_HAND` (RB_LEG_WH)
pub fn w_hand() -> f64 {
    static V: OnceLock<f64> = OnceLock::new();
    *V.get_or_init(|| w("RB_LEG_WH", "0.3"))
}

/// `SEUIL[style]`
pub fn seuil(style: &str) -> i64 {
    match style {
        "aggro" | "tempo" => 5,
        "midrange" => 6,
        "controle" | "ramp" | "combo" => 7,
        _ => panic!("KeyError: {style}"),
    }
}

/// Fiche d'une légende (`L(axe, style, garde, cles, conf, src, maj, note)`).
#[derive(Clone, Debug)]
pub struct Leg {
    pub axe: &'static str,
    pub style: &'static str,
    pub garde: &'static [&'static str],
    pub cles: &'static [&'static str],
    pub conf: &'static str,
    pub src: &'static str,
    pub maj: &'static str,
    pub note: &'static str,
}

const LEG_TABLE: &[(&str, Leg)] = &[
    ("Ahri, Nine-Tailed Fox", Leg { axe: "T", style: "controle", garde: &["Ravenbloom Student", "Sprite Fountain"], cles: &["Ravenbloom Student", "Thousand-Tailed Watcher", "Ahri, Inquisitive"], conf: "lu", src: "https://riftbound.gg/ahri-nine-tailed-fox-guide/", maj: "2026-07-17", note: "Jeu de tenue : ancre tôt (Ravenbloom Student), garder une rune ouverte pour réagir aux conquêtes, Sprite pour tenir ; Thousand-Tailed Watcher conquiert en fin de partie. Le 8e point se prend plutôt en conquérant dans son tour qu'en tenant (une tenue rend souvent un tour à l'adversaire)." }),
    ("Akali, Rogue Assassin", Leg { axe: "T", style: "controle", garde: &["Stellacorn Herder", "Astral Heron"], cles: &["Stellacorn Herder", "Astral Heron", "Kai'Sa, Survivor"], conf: "lu", src: "https://riftbound.gg/akali-rogue-assassin-guide/", maj: "2026-09-14", note: "Plan existant Gorica (plans.py) gardé. Guide : removal efficace, contrôle du plateau, deux moteurs de pioche à protéger (Stellacorn Herder, Kai'Sa, Survivor), poser Akali (fragile) seulement sur un plateau sûr ; Astral Heron ancre la fin de partie." }),
    ("Ambessa, Matriarch of War", Leg { axe: "C", style: "midrange", garde: &["Legion Marauder", "Irresistible Faefolk"], cles: &["Legion Marauder", "Ambessa, The Wolf"], conf: "lu", src: "https://riftbound.gg/ambessa-matriarch-of-war-guide/", maj: "2026-08-14", note: "Plateau : prendre le plateau tôt et forcer l'adversaire à défendre ; la légende prépare une unité (Empower) pour marquer ; removal en base. Légende faible en partie longue : recycler les runes sans hésiter en fin de partie." }),
    ("Annie, Dark Child", Leg { axe: "C", style: "aggro", garde: &["Traveling Merchant", "Inferna", "Rengar, Pouncing"], cles: &["Traveling Merchant", "Kai'Sa, Survivor"], conf: "lu", src: "https://riftbound.gg/annie-dark-child-guide/", maj: "2026-07-03", note: "Aggro : conquérir chaque tour jusqu'à 8. La légende prépare 2 runes en fin de tour : tout dépenser dans son tour et garder ces 2 runes pour réagir. Pioche indispensable (Traveling Merchant, Kai'Sa) ; les attaquants [Assault] tiennent mal ; Vex et Star-Crossed contre une grosse unité adverse." }),
    ("Azir, Emperor of the Sands", Leg { axe: "T", style: "controle", garde: &["Doran's Shield", "Soul Sword", "Eye of the Herald"], cles: &["Azir, Sovereign"], conf: "lu", src: "https://riftbound.gg/azir-emperor-of-the-sands-guide/", maj: "2026-09-08", note: "Contrôle : 13 équipements bon marché pour créer des Sand Soldiers ; Arise! (sort signature) finit la partie. Contre un deck lent : poser chaque tour et marquer ; contre un deck rapide : valeur, removal." }),
    ("Darius, Hand of Noxus", Leg { axe: "C", style: "aggro", garde: &["Baited Hook", "Carrion Dredger", "Noxus Hopeful", "Honest Broker"], cles: &["Baited Hook"], conf: "lu", src: "https://riftbound.gg/darius-hand-of-noxus-guide/", maj: "2026-07-06", note: "Aggro / tempo pressé : développer au maximum tôt (Legion plus fort tôt), Baited Hook sacrifie une petite unité pour en poser une plus grosse en base ; removal Order. Peu de retour possible : ne pas prendre de retard." }),
    ("Diana, Scorn of the Moon", Leg { axe: "C", style: "tempo", garde: &["Ravenbloom Student", "Traveling Merchant"], cles: &["Ravenbloom Student", "Hwei, Brooding Painter"], conf: "lu", src: "https://riftbound.gg/diana-scorn-of-the-moon-guide/", maj: "2026-09-14", note: "Rapide : dominer les showdowns tôt (Ravenbloom Student, sorts pas chers payés par la légende), removal contre les grosses unités au milieu, Ride The Wind pour finir (marque aussi au tour adverse)." }),
    ("Draven, Glorious Executioner", Leg { axe: "T", style: "midrange", garde: &["Kai'Sa, Survivor", "Vex, Apathetic"], cles: &["Vex, Apathetic", "Kai'Sa, Survivor", "Draven, Showboat"], conf: "lu", src: "https://riftbound.gg/draven-glorious-executioner-guide/", maj: "2026-09-14", note: "Midrange réactif : Vex, Apathetic et Kai'Sa, Survivor gagnent la partie s'ils restent en vie, les protéger ; le champion est lent (4 points avant d'être une menace). Faible derrière et contre les decks rapides." }),
    ("Ezreal, Prodigal Explorer", Leg { axe: "T", style: "controle", garde: &["Bewitching Spirit", "Fizz, Trickster"], cles: &["Vex, Apathetic", "Thousand-Tailed Watcher"], conf: "lu", src: "https://riftbound.gg/ezreal-prodigal-explorer-guide/", maj: "2026-09-14", note: "Contrôle / perturbation : plateau faible, removal en continu, gagner à l'usure quand l'adversaire n'a plus de cartes ; la légende évite de manquer de cartes ; Thousand-Tailed Watcher conclut." }),
    ("Fiora, Grand Duelist", Leg { axe: "T", style: "ramp", garde: &["Mobilize", "Dazzling Aurora"], cles: &["Dazzling Aurora", "Elder Dragon"], conf: "deck", src: "https://riftbound.gg/fiora-grand-duelist-guide/", maj: "2026-09-11", note: "Le deck par défaut est la version Aurora du guide : monter en runes (Mobilize, Catalyst of Aeons), poser Dazzling Aurora qui met en jeu des dragons ; Rengar et Rift Herald entre-temps." }),
    ("Garen, Might of Demacia", Leg { axe: "T", style: "ramp", garde: &["Mobilize", "Catalyst of Aeons", "Dazzling Aurora"], cles: &["Dazzling Aurora", "Elder Dragon", "Vanguard Armory"], conf: "deck", src: "https://riftbound.gg/garen-might-of-demacia-guide/", maj: "2026-03-19", note: "Le guide (ancien) parle de jetons ; les listes compétitives jouent Dazzling Aurora, comme le deck par défaut : monter en runes, Aurora, dragons, Vanguard Armory pour les jetons. Légende la plus faible selon le guide." }),
    ("Irelia, Blade Dancer", Leg { axe: "T", style: "controle", garde: &["Stellacorn Herder", "Astral Heron"], cles: &["Stellacorn Herder", "Astral Heron", "Irelia, Fervent"], conf: "lu", src: "https://riftbound.gg/irelia-blade-dancer-guide/", maj: "2026-09-08", note: "Contrôle : se développer sans pousser trop tôt, la légende pousse les points au milieu ; Astral Heron tient en fin de partie ; les autres unités clés rentrent en base en fin de tour ; pas de showdown contre une grosse unité Body / Fury sans réponse." }),
    ("Ivern, Green Father", Leg { axe: "T", style: "midrange", garde: &["Mutated Mouser", "Stalwart Poro", "Trusty Ramhound"], cles: &["Ivern, Nurturer"], conf: "recherche", src: "https://mobalytics.gg/riftbound/decks/ivern-green-father-barcelona-regional-qualifier-best-of-aserty", maj: "2026-08", note: "Pas de guide trouvé. Listes Calm / Order : petites unités (Mutated Mouser, Stalwart Poro, Trusty Ramhound), Defy, Discipline, Friendship ; plan de tenue déduit de la liste." }),
    ("Jax, Grandmaster At Arms", Leg { axe: "C", style: "midrange", garde: &["Brutalizer", "Doran's Blade", "Guardian Angel"], cles: &["Jax, Unmatched"], conf: "lu", src: "https://riftbound.gg/jax-grandmaster-at-arms-guide/", maj: "2026-02-11", note: "Équipements : des unités dures à retirer qui provoquent des showdowns ; la légende demande un équipement attaché et deux unités ; protéger les cartes clés avec les sorts Calm." }),
    ("Jayce, Defender of Tomorrow", Leg { axe: "T", style: "ramp", garde: &["Mobilize", "Platewyrm Egg", "Catalyst of Aeons", "Dazzling Aurora"], cles: &["Elder Dragon", "Dazzling Aurora"], conf: "lu", src: "https://riftbound.gg/jayce-defender-of-tomorrow-guide/", maj: "2026-09-14", note: "Ramp / pioche : mettre une grande menace en jeu tôt pour tenir un battlefield (Dazzling Aurora, Promising Future) ; pas de combos d'équipement (le retrait d'équipement est partout)." }),
    ("Jhin, Virtuoso", Leg { axe: "T", style: "combo", garde: &["Seal of Insight", "Energy Conduit", "Promising Future", "Time Warp"], cles: &["Energy Conduit"], conf: "lu", src: "https://riftbound.gg/jhin-virtuoso-guide/", maj: "2026-07-17", note: "Combo : tours 1 à 4, tenir, gagner des runes (les sorts de 4 et plus en donnent avec la légende), piocher, retirer des unités, sans surengager ; concentrer le removal sur un battlefield ; Time Warp au bon tour. Ignorer le score adverse tant qu'il n'approche pas 8." }),
    ("Jinx, Loose Cannon", Leg { axe: "C", style: "aggro", garde: &["Traveling Merchant", "Chemtech Enforcer", "Overzealous Fan"], cles: &["Traveling Merchant"], conf: "lu", src: "https://riftbound.gg/jinx-loose-cannon-guide/", maj: "2026-02-20", note: "Aggro défausse : la légende pioche 2 quand la main a 1 carte ou moins, donc défausser sans retenue et vider sa main ; pression constante contre un deck lent ; contre un deck aussi rapide, dépenser moins de cartes." }),
    ("Kai'Sa, Daughter of the Void", Leg { axe: "C", style: "midrange", garde: &["Watchful Sentry", "Lecturing Yordle", "Plundering Poro"], cles: &["Thousand-Tailed Watcher", "Kai'Sa, Survivor"], conf: "lu", src: "https://riftbound.gg/kaisa-daughter-of-the-void-guide/", maj: "2026-07-13", note: "Midrange proactif : coller des unités au plateau, puis sorts et pioche pour prendre les battlefields ; la légende ignore un coût de puissance par tour, surtout utile une fois une unité posée (removal = conquête)." }),
    ("Kennen, Heart of the Tempest", Leg { axe: "T", style: "controle", garde: &["Traveling Merchant", "Shadow Order Disciple", "Lightning Rush"], cles: &["Rhasa the Sunderer", "Kennen, Storm of Shuriken"], conf: "lu", src: "https://riftbound.gg/playing-building-kennen-in-the-post-ban-metagame/", maj: "2026-10-07", note: "Midrange / contrôle : respecter le début adverse, contrôler la vitesse du score, remplir la défausse pour un gros tour (Rhasa the Sunderer) ; ne pas faire la course. Mauvais contre Rengar et Master Yi." }),
    ("Kha'Zix, Voidreaver", Leg { axe: "C", style: "midrange", garde: &["Vex, Apathetic", "Irresistible Faefolk", "Kinkou Initiate"], cles: &["Vex, Apathetic", "Kha'Zix, Mutating Horror"], conf: "lu", src: "https://riftbound.gg/khazix-voidreaver-guide/", maj: "2026-07-05", note: "Midrange Body / Chaos : Vex, Apathetic tôt sur un battlefield, menaces au milieu, [Ambush] pour soutenir les petites unités ; une fois devant, perturbation Chaos." }),
    ("LeBlanc, Deceiver", Leg { axe: "C", style: "tempo", garde: &["Soaring Scout", "Glasc Mixologist"], cles: &["LeBlanc, Fragmented", "Karthus, Eternal"], conf: "lu", src: "https://riftbound.gg/leblanc-deceiver-guide/", maj: "2026-07-03", note: "Plan existant Hook tempo (plans.py) gardé. Guide : [Deathknell] d'abord, reflets faits pour mourir, Baited Hook (version occidentale) pour poser plus gros en déclenchant les Deathknell." }),
    ("Lee Sin, Blind Monk", Leg { axe: "T", style: "midrange", garde: &["Lonely Poro", "Scuttle Crab"], cles: &["Lee Sin, Centered", "Wizened Elder", "Irelia, Fervent"], conf: "lu", src: "https://riftbound.gg/lee-sin-blind-monk-guide/", maj: "2026-03-19", note: "Midrange unités + buffs : empiler les unités sur un seul lieu et les buffer ; la position avant les points tôt ; buffer la plus faible de deux unités pour qu'elles échangent." }),
    ("Leona, Radiant Dawn", Leg { axe: "T", style: "midrange", garde: &["Nami, Headstrong", "Stalwart Poro"], cles: &["Leona, Zealot", "Fiora, Victorious"], conf: "lu", src: "https://riftbound.gg/leona-radiant-dawn-guide/", maj: "2026-02-20", note: "Showdowns : étourdir les unités adverses pour buffer les siennes (légende) ; se développer une fois un battlefield contrôlé ; protéger les unités clés. Faiblesse : les unités dans la base adverse." }),
    ("Lillia, Bashful Bloom", Leg { axe: "T", style: "controle", garde: &["Sprite Fountain", "Ravenbloom Student", "Plundering Poro"], cles: &["Thousand-Tailed Watcher"], conf: "lu", src: "https://riftbound.gg/lillia-bashful-bloom-guide/", maj: "2026-07-12", note: "Valeur : échanger les Sprites temporaires contre les vraies unités adverses, puis les grosses cartes de fin." }),
    ("Lucian, Purifier", Leg { axe: "C", style: "midrange", garde: &["Doran's Blade", "Long Sword", "Kai'Sa, Survivor"], cles: &["Lucian, Merciless", "Kai'Sa, Survivor"], conf: "lu", src: "https://riftbound.gg/lucian-purifier-guide/", maj: "2026-02-14", note: "Conquérant à équipements : conquérir chaque tour en gardant ses unités en vie pour reconquérir ; Relentless Pursuit marque aussi au tour adverse." }),
    ("Lux, Lady of Luminosity", Leg { axe: "T", style: "combo", garde: &["Seal of Insight", "Forge of the Future", "Ekko, Recurrent", "Time Warp"], cles: &["Ekko, Recurrent"], conf: "lu", src: "https://riftbound.gg/lux-lady-of-luminosity-guide/", maj: "2026-07-13", note: "Combo contrôle : piocher, removal pour ralentir l'adversaire, puis la boucle d'énergie et Time Warp." }),
    ("Master Yi, Wuju Bladesman", Leg { axe: "T", style: "tempo", garde: &["Pit Rookie", "Scuttle Crab", "First Mate"], cles: &["Master Yi, Tempered", "Rengar, Trophy Hunter"], conf: "lu", src: "https://riftbound.gg/master-yi-wuju-bladesman-guide/", maj: "2026-08-20", note: "Tenir ou conquérir avec des unités et des sorts pas chers ; la légende donne +2 might au défenseur seul. Tôt : peu de sorts, garder ses runes pour se développer, 1 rune ouverte au tour 3 si besoin ; pas de removal sur les petites unités adverses tôt." }),
    ("Master Yi, Wuju Master", Leg { axe: "C", style: "midrange", garde: &["Wuju Apprentice", "Gemhand Hunter", "Herald of Spring"], cles: &["Master Yi, Unstoppable", "Voracious Gromp"], conf: "lu", src: "https://riftbound.gg/master-yi-wuju-master-guide/", maj: "2026-07-17", note: "Hunt / XP : pas passif tôt, gagner de l'XP vers 6 puis 11 (conquêtes et tenues), puis submerger. Contre l'aggro, retirer les unités même au prix d'XP." }),
    ("Mel, Soul's Reflection", Leg { axe: "T", style: "midrange", garde: &["Ravenbloom Student", "Traveling Merchant"], cles: &["Applied Researchers", "Mel, Defiant Soul"], conf: "lu", src: "https://riftbound.gg/mel-souls-reflection-guide/", maj: "2026-08-08", note: "Midrange valeur : removal par la might, garder quelques runes pour protéger une unité importante sur un battlefield." }),
    ("Miss Fortune, Bounty Hunter", Leg { axe: "T", style: "ramp", garde: &["Mobilize", "Catalyst of Aeons", "Dazzling Aurora"], cles: &["Dazzling Aurora", "Elder Dragon", "Miss Fortune, Captain"], conf: "lu", src: "https://riftbound.gg/miss-fortune-bounty-hunter-guide/", maj: "2026-02-14", note: "Ramp (Dazzling Aurora) : défendre tôt et limiter les points adverses, attaquer tard avec les grosses unités, conquérir les deux battlefields le même tour grâce à [Ganking] (légende)." }),
    ("Nasus, Curator of the Sands", Leg { axe: "T", style: "ramp", garde: &["Clockwork Keeper", "Ravenbloom Student", "Find Your Center"], cles: &["Thousand-Tailed Watcher", "Sprite Queen", "Nasus, Ascended"], conf: "lu", src: "https://riftbound.gg/nasus-curator-of-the-sands-guide/", maj: "2026-08-20", note: "Fin de partie : contester tôt (Ravenbloom Student), monter à 7 runes vers le tour 3, puis grosse unité + sort pas cher (la légende prépare 2 runes). N'a pas peur d'être derrière." }),
    ("Ornn, Fire Below the Mountain", Leg { axe: "C", style: "tempo", garde: &["Pit Crew", "Seal of Focus", "Clockwork Keeper"], cles: &["Ornn, Blacksmith", "Pit Crew"], conf: "lu", src: "https://riftbound.gg/ornn-fire-below-the-mountain-guide/", maj: "2026-08-30", note: "Équipements, très rapide : utiliser la légende chaque tour pour se développer en gardant des runes, ne pas seulement réagir ; faible contre les adversaires qui ferment le jeu tôt. Vainqueur du RQ de Barcelone." }),
    ("Poppy, Keeper of the Hammer", Leg { axe: "T", style: "ramp", garde: &["Mobilize", "Catalyst of Aeons", "Dazzling Aurora"], cles: &["Dazzling Aurora", "Elder Dragon", "Vanguard Armory"], conf: "recherche", src: "https://mobalytics.gg/riftbound/decks/poppy-keeper-of-the-hammer-sydney-regional-qualifier-best-of-rowanno1", maj: "date inconnue", note: "Pas de guide trouvé. Listes Body / Order Aurora : monter en runes, Dazzling Aurora, dragons, Vanguard Armory." }),
    ("Pyke, Bloodharbor Ripper", Leg { axe: "C", style: "midrange", garde: &["Traveling Merchant", "Treasure Hunter"], cles: &["Vex, Apathetic", "Ezreal, Prodigy"], conf: "lu", src: "https://riftbound.gg/pyke-bloodharbor-ripper-guide/", maj: "2026-07-08", note: "Perturbation Chaos + removal Fury + [Ambush] : pousser l'avance tôt ; en partie lente, gagner la guerre de valeur (défausse adverse, renvoi des grosses unités)." }),
    ("Rek'sai, Void Burrower", Leg { axe: "C", style: "aggro", garde: &["Carrion Dredger", "Mischievous Marai", "Noxus Hopeful", "Inferna"], cles: &["Kai'Sa, Survivor", "Undertitan"], conf: "lu", src: "https://riftbound.gg/reksai-void-burrower-guide/", maj: "2026-09-14", note: "Conquête : pression tôt avec au moins 9 unités à 2, garder l'adversaire en réaction, removal pour réduire les cibles et conquérir ; peu d'outils pour revenir de derrière." }),
    ("Renata Glasc, Chem-Baroness", Leg { axe: "C", style: "midrange", garde: &["Pit Crew", "Plundering Poro", "Sprite Fountain", "Patched Porobot"], cles: &["Renata Glasc, Mastermind"], conf: "lu", src: "https://riftbound.gg/renata-glasc-chem-baroness-guide/", maj: "2026-03-19", note: "Départ proactif pour atteindre 5 points vite (débloque l'énergie de la légende) ; Hostile Takeover en removal au milieu ; ensuite tours explosifs ou ressources gardées pour réagir." }),
    ("Renekton, Butcher of the Sands", Leg { axe: "C", style: "aggro", garde: &["First Mate", "Inferna", "Pit Rookie"], cles: &["Renekton, Rage Fueled", "Rengar, Trophy Hunter"], conf: "recherche", src: "https://riftbound.gg/vendette-pre-rift-decks-for-each-legend/", maj: "2026-07-25", note: "Pas de guide de tournoi lisible. Légende [Reaction] : énergie pour les unités ; « nourrir le moteur, inonder le plateau » ; une unité au tour 1 puis les grosses." }),
    ("Rengar, Pridestalker", Leg { axe: "C", style: "aggro", garde: &["Inferna", "Pit Rookie", "Irresistible Faefolk", "Kinkou Initiate"], cles: &["Kai'Sa, Survivor", "Rengar, Trophy Hunter"], conf: "lu", src: "https://riftbound.gg/rengar-pridestalker-guide/", maj: "2026-08-30", note: "Aggro Body / Fury : prendre l'avance vite et rester devant, gagner les premiers showdowns ; peu de retour possible." }),
    ("Rumble, Mechanized Menace", Leg { axe: "C", style: "tempo", garde: &["Forecaster", "Gem Jammer", "Plundering Poro"], cles: &["Rumble, Hotheaded", "Thousand-Tailed Watcher"], conf: "lu", src: "https://riftbound.gg/rumble-mechanized-menace-guide/", maj: "2026-03-19", note: "Mechs : prendre un battlefield au tour 2 avec un Mech (surtout en commençant), accumuler l'avance de plateau ; plan simple et limité." }),
    ("Sett, The Boss", Leg { axe: "C", style: "aggro", garde: &["Fiora, Victorious", "Pit Rookie", "First Mate", "Irresistible Faefolk"], cles: &["Fiora, Victorious", "Sett, Kingpin"], conf: "lu", src: "https://riftbound.gg/sett-the-boss-guide/", maj: "2026-07-07", note: "« Marquer à tout prix » : buffs, garder les unités en vie (la légende les sauve du removal), marquer chaque tour ; pas de guerre de valeur." }),
    ("Shen, Eye of Twilight", Leg { axe: "T", style: "midrange", garde: &["Lonely Poro", "Loyal Poro", "Unsung Hero"], cles: &["Shen, Scourge of Shadows", "Fiora, Victorious"], conf: "recherche", src: "https://riftbound.gg/vendette-pre-rift-decks-for-each-legend/", maj: "2026-07-25", note: "Pas de guide de tournoi lisible. Légende : donner [Tank] à une unité ; contrôle défensif, protéger la meilleure unité, tenir ses battlefields ; le début de partie compte." }),
    ("Sivir, Battle Mistress", Leg { axe: "C", style: "tempo", garde: &["Traveling Merchant", "Shadow Order Disciple", "Fizz, Trickster"], cles: &["Sivir, Mercenary", "Ezreal, Prodigy"], conf: "deck", src: "https://riftbound.gg/sivir-battle-mistress-guide/", maj: "2026-05-18", note: "Le guide décrit une version Aurora ; le deck par défaut est un tempo Body / Chaos (cartes cachées, Tornado Warrior, Tail-Cloaked Matriarch) : plan de conquérant déduit de la liste." }),
    ("Teemo, Swift Scout", Leg { axe: "T", style: "controle", garde: &["Ravenbloom Student", "Sprite Fountain", "Evelynn, Entrancing"], cles: &["Teemo, Strategist", "Nocturne, Horrifying"], conf: "lu", src: "https://riftbound.gg/teemo-swift-scout-guide/", maj: "2026-07-17", note: "Contrôle à cartes cachées : tenir les battlefields, cacher une carte chaque tour une fois devant, marquer chaque tour et retirer les unités qui menacent un retour ; ne pas s'épuiser trop tôt." }),
    ("Vex, Gloomist", Leg { axe: "T", style: "controle", garde: &["Mutated Mouser", "Evelynn, Entrancing", "Teemo, Scout"], cles: &["Vex, Apathetic", "Evelynn, Entrancing"], conf: "lu", src: "https://riftbound.gg/vex-gloomist-guide/", maj: "2026-09-14", note: "Contrôle de tenue : plateau solide, contester un battlefield tôt, puis refuser les retours adverses (stuns, contres) ; Vex, Apathetic à poser tôt et protéger." }),
    ("Vi, Piltover Enforcer", Leg { axe: "C", style: "aggro", garde: &["Honest Broker", "Unsung Hero", "Long Sword"], cles: &["Vi, Destructive"], conf: "recherche", src: "https://mobalytics.gg/riftbound/decks/vi-piltover-enforcer-sydney-regional-qualifier-best-of-sirieous", maj: "date inconnue", note: "Pas de guide lisible. Fury / Order agressif : conquérir vite ; la légende prépare une unité quand on tue une unité avec 3 dégâts de trop." }),
    ("Viktor, Herald of the Arcane", Leg { axe: "C", style: "midrange", garde: &["Carrion Dredger", "Honest Broker", "Xin Zhao, Vigilant"], cles: &["Viktor, Leader", "Xin Zhao, Vigilant"], conf: "lu", src: "https://riftbound.gg/viktor-herald-of-the-arcane-guide/", maj: "2026-07-04", note: "Hors tempo, jetons : le jeton de la légende conquiert les battlefields vides (pas de showdown), removal pour prendre l'avance, unités qui entrent prêtes pour finir." }),
    ("Volibear, Relentless Storm", Leg { axe: "T", style: "ramp", garde: &["Mobilize", "Catalyst of Aeons", "Dazzling Aurora"], cles: &["Elder Dragon", "Kadregrin the Infernal", "Gentle Gemdragon", "Dazzling Aurora"], conf: "lu", src: "https://riftbound.gg/volibear-relentless-storm-guide/", maj: "2026-03-19", note: "Ramp : unités de 5 might et plus (rune en plus avec la légende), garder le score adverse bas tôt, removal seulement si besoin, puis les grosses menaces." }),
    ("Yasuo, Unforgiven", Leg { axe: "C", style: "tempo", garde: &["Scuttle Crab", "Stellacorn Herder", "Treasure Hunter"], cles: &["Stellacorn Herder", "Kha'Zix, Mutating Horror", "Yasuo, Remorseful"], conf: "lu", src: "https://riftbound.gg/yasuo-unforgiven-guide/", maj: "2026-04-21", note: "Déplacer des unités pour marquer chaque tour, protéger celles qui grandissent en bougeant ; la légende traite une unité comme prête (conquérir ou rentrer en base)." }),
    ("Zed, Master of Shadows", Leg { axe: "C", style: "tempo", garde: &["Traveling Merchant", "Death Mark", "Noxus Hopeful"], cles: &["Zed, Without a Sound", "Kharox"], conf: "recherche", src: "https://riftbound.cardsrealm.com/en-us/articles/riftbound-review-zeds-kit", maj: "avant Vendetta", note: "Pas de guide de tournoi. Revue d'avant la sortie : jetons Shadow Clone (Death Mark) renforcés en bannissant des unités de la défausse ; Fury / Chaos." }),
];

/// `LEG` : (légende, fiche), ordre du dict Python.
pub fn leg() -> &'static [(&'static str, Leg)] {
    LEG_TABLE
}

/// `SANS_PLAN`
pub const SANS_PLAN: [&str; 9] = [
    "Ekko, Boy Who Shattered Time",
    "Evelynn, Agony's Embrace",
    "Jarvan IV, Exemplar of Demacia",
    "K'Sante, Pride of Nazumah",
    "Mordekaiser, Iron Revenant",
    "Orianna, Lady of Clockwork",
    "Riven, The Exile",
    "Seraphine, Starry-Eyed Songstress",
    "Ziggs, Hexplosives Expert",
];

/// `EXISTING`
pub const EXISTING: [(&str, &str); 2] = [("Akali, Rogue Assassin", "Gorica"), ("LeBlanc, Deceiver", "Hook tempo")];

/// `LegendPlan(legend)`
#[derive(Clone, Debug)]
pub struct LegendPlan {
    pub legend: String,
    pub d: Leg,
}

impl LegendPlan {
    pub fn new(legend: &str) -> LegendPlan {
        let d = leg().iter().find(|x| x.0 == legend).unwrap_or_else(|| panic!("KeyError: {legend}")).1.clone();
        LegendPlan { legend: legend.to_string(), d }
    }
}

impl Plan for LegendPlan {
    fn name(&self) -> &str {
        "legende"
    }
    fn mulligan(&self, g: &mut Game, pid: usize) -> Res<Option<Vec<O>>> {
        let h = g.p[pid].hand.clone();
        let st = self.d.style;
        let lim = seuil(st);
        let mut cand: Vec<O> = h.iter().copied().filter(|&c| !self.d.garde.contains(&g.cname(c))).collect();
        cand.sort_by_key(|&c| (-((g.spec(c).e + g.spec(c).p) as i64), g.o(c).uid));
        let mut out: Vec<O> = cand.iter().copied().filter(|&c| g.spec(c).e as i64 >= lim).collect();
        if (st == "aggro" || st == "tempo") && !h.iter().any(|&c| g.spec(c).type_ == "Unit" && g.spec(c).e <= 3) {
            let extra: Vec<O> = cand.iter().copied().filter(|c| !out.contains(c) && g.spec(*c).e >= 4).collect();
            out.extend(extra);
        }
        out.truncate(2);
        Ok(Some(out))
    }
    fn shape(&self, g: &mut Game, me: usize) -> Res<f64> {
        if g.winner.is_some() {
            return Ok(0.0);
        }
        let d = &self.d;
        let mut v = 0.0;
        if d.axe == "C" {
            let mut t = 0;
            for u in g.units(Some(me), None) {
                if is_bf(g.o(u).loc) {
                    t += 0.max(g.might(u)?);
                }
            }
            v += w_conq() * t as f64;
        } else {
            v += w_hold() * g.bfs.iter().filter(|b| b.ctrl == Some(me)).count() as f64;
            if g.p[1 - me].points >= g.victory - 2 {
                let ready = g.p[me].runes.iter().filter(|&&r| !g.r(r).exhausted).count();
                v += w_open() * ready.min(2) as f64;
            }
        }
        if !d.cles.is_empty() {
            let n = g.board.iter().filter(|&&o| g.o(o).ctrl == me && d.cles.contains(&g.cname(o))).count();
            v += w_key() * n as f64;
        }
        if d.style == "controle" || d.style == "combo" {
            v += w_hand() * g.p[me].hand.len() as f64;
        }
        Ok(v)
    }
}

/// `plan_for(deck)` : plan existant (Akali, LeBlanc), plan par légende, sinon aucun plan.
pub fn plan_for(deck: &DeckList) -> Arc<dyn Plan> {
    let lg = deck.legend.as_str();
    if let Some((_, p)) = EXISTING.iter().find(|x| x.0 == lg) {
        return plans(p).unwrap();
    }
    if leg().iter().any(|x| x.0 == lg) {
        return Arc::new(LegendPlan::new(lg));
    }
    Arc::new(NoPlan)
}
