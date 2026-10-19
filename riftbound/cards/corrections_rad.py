"""Corrections de la source pour Radiance (RAD) : la source (riftcodex, 2026-10-15, avant la sortie du 2026-10-23) donne
des domaines et des tags faux. Relevés sur les images officielles des cartes le 2026-10-19 (domaine = couleur et
symbole du cadre, tags = bandeau du type). Le tag interne « Device » des équipements n'est imprimé sur aucune carte.
Appliqué par build_cards.py (id -> champs remplacés)."""

FIX = {
    'rad-167': {'name': 'Neeko, Blending In'},  # nom imprimé (« Bleeding In » dans la source)
    'rad-001': {'domain': 'Fury', 'tags': 'Bandle City|Yordle'},  # Amateur Demolitionist
    'rad-002': {'tags': ''},  # Banner of Conquest
    'rad-003': {'domain': 'Fury', 'tags': 'Bilgewater'},  # Bilgewater Tracker
    'rad-004': {'domain': 'Fury'},  # Brawl
    'rad-005': {'tags': 'Shadow Isles|Spirit'},  # Lingering Animus
    'rad-006': {'domain': 'Fury'},  # Scorched Earth
    'rad-008': {'tags': 'Demacia|Yordle'},  # War Scout
    'rad-009': {'domain': 'Fury', 'tags': 'Bandle City|Yordle|Mech'},  # Antagonistic Apparatus
    'rad-011': {'tags': 'Piltover|Mech'},  # Incorrigible Construct
    'rad-012': {'domain': 'Fury'},  # Runeblade Anvil
    'rad-014': {'tags': 'Bilgewater'},  # Shimmering Swimmer
    'rad-015': {'tags': 'Akali|Ionia'},  # Akali, Brash
    'rad-016': {'domain': 'Fury', 'tags': 'Shurima'},  # Coalesced Energy
    'rad-017': {'domain': 'Fury', 'tags': ''},  # Detonation Amplifier
    'rad-018': {'tags': 'Noxus'},  # Drakehound Packmaster
    'rad-019': {'tags': 'Freljord'},  # Firebrand Raider
    'rad-020': {'tags': 'Mordekaiser|Noxus|Spirit'},  # Mordekaiser, Twice Slain
    'rad-022': {'domain': 'Fury'},  # Whirling Twirligig
    'rad-023': {'tags': 'Ziggs|Zaun|Yordle'},  # Ziggs, Short-Fused
    'rad-024': {'domain': 'Calm', 'tags': ''},  # Blossoming Dreamroot
    'rad-026': {'domain': 'Calm', 'tags': 'Freljord'},  # Entrenched Boar
    'rad-027': {'tags': 'Ixtal|Bird'},  # Iridescent Avian
    'rad-030': {'tags': 'Shurima'},  # Sanctum Conservator
    'rad-031': {'tags': 'Bilgewater|Sentinel'},  # Stoic Ferryman
    'rad-036': {'tags': 'Freljord|Bird'},  # Storm Raptor
    'rad-037': {'tags': 'Piltover'},  # Watchful Curator
    'rad-040': {'tags': 'Shadow Isles|Spirit'},  # Heartening Soul
    'rad-041': {'domain': 'Calm'},  # Hush
    'rad-042': {'tags': 'Orianna|Piltover'},  # Orianna, Exquisite
    'rad-043': {'tags': 'Riven|Noxus'},  # Riven, Broken Blade
    'rad-044': {'tags': ''},  # Hunting Blind
    'rad-045': {'tags': "K'Sante|Shurima"},  # K'Sante, Dauntless
    'rad-047': {'domain': 'Mind'},  # Fanfare
    'rad-049': {'domain': 'Mind', 'tags': 'Zaun'},  # Industrious Mechsmith
    'rad-050': {'tags': 'Shurima'},  # Mysterious Chronomancer
    'rad-052': {'tags': ''},  # Resource Extractor
    'rad-053': {'tags': 'Freljord|Bird'},  # Strigian Oracle
    'rad-054': {'tags': 'Bandle City|Yordle'},  # Trinket Peddler
    'rad-055': {'tags': 'Piltover|Mech'},  # Clanging Sentry
    'rad-057': {'tags': 'Piltover|Mech'},  # Piltovan Guidebot
    'rad-058': {'tags': 'Bilgewater'},  # Riptide Mage
    'rad-059': {'domain': 'Mind'},  # Supernova
    'rad-060': {'tags': 'Ionia|Dragon'},  # The Empyrean
    'rad-062': {'domain': 'Mind', 'tags': ''},  # Gravity Field
    'rad-065': {'tags': 'Seraphine|Piltover'},  # Seraphine, Inspiring
    'rad-066': {'tags': 'Freljord|Spirit'},  # Spirit of the Forge
    'rad-067': {'domain': 'Mind', 'tags': 'Shurima'},  # Clocktower Guardian
    'rad-068': {'tags': 'Orianna|Piltover'},  # Orianna, Mechanical Marvel
    'rad-071': {'domain': 'Body', 'tags': 'Equipment'},  # Nazuman Ntofo
    'rad-072': {'tags': 'Bandle City|Yordle'},  # Overloaded Traveler
    'rad-073': {'domain': 'Body', 'tags': ''},  # Pillaged Armory
    'rad-074': {'tags': 'Freljord'},  # Raving Witness
    'rad-075': {'tags': 'Demacia|Elite|Bird'},  # Skystrike Squadron
    'rad-076': {'tags': 'Mount Targon'},  # Sneaky Zeebles
    'rad-077': {'tags': 'Freljord|Spirit'},  # Wrath of the Freljord
    'rad-078': {'tags': 'Noxus'},  # Armored Tusker
    'rad-080': {'tags': 'Shurima'},  # Bloodscent Swarm
    'rad-081': {'domain': 'Body', 'tags': 'Freljord'},  # Elnuk Herd
    'rad-083': {'tags': 'Mount Targon'},  # Stalwart Goatherd
    'rad-085': {'tags': 'Graves|Bilgewater'},  # Graves, Blasting Through
    'rad-086': {'tags': "K'Sante|Shurima"},  # K'Sante, Courageous
    'rad-087': {'tags': ''},  # Portable Portal
    'rad-088': {'tags': 'Freljord'},  # Staunch Survivor
    'rad-089': {'tags': ''},  # Titanic Shrine
    'rad-090': {'tags': 'Evelynn|Demon'},  # Evelynn, Consuming
    'rad-091': {'tags': 'Jarvan IV|Demacia'},  # Jarvan IV, Martial Paragon
    'rad-092': {'tags': 'Mount Targon|Dragon'},  # Seraphic Wyvern
    'rad-093': {'domain': 'Chaos', 'tags': 'Bandle City|Yordle'},  # Disposal Expert
    'rad-094': {'tags': 'Zaun'},  # Explosimian
    'rad-095': {'tags': 'Shadow Isles|Demon'},  # Momentary Companion
    'rad-097': {'tags': 'Bilgewater'},  # Siren of the Shallows
    'rad-099': {'tags': 'Bilgewater|Demon'},  # Suave Negotiator
    'rad-102': {'domain': 'Chaos'},  # Demon Shade
    'rad-103': {'tags': 'Zaun'},  # Gutter Guardian
    'rad-104': {'domain': 'Chaos', 'tags': 'Shadow Isles|Cat'},  # Pitiless Stalker
    'rad-105': {'tags': 'Piltover'},  # Reckless Innovator
    'rad-106': {'domain': 'Chaos', 'tags': ''},  # The Good Stuff
    'rad-107': {'tags': 'Shadow Isles|Demon'},  # Bedeviling Stranger
    'rad-108': {'domain': 'Chaos', 'tags': ''},  # Bushwhack Trap
    'rad-109': {'tags': 'Evelynn|Demon'},  # Evelynn, In Control
    'rad-110': {'tags': 'Lulu|Bandle City|Yordle'},  # Lulu, Whimsical
    'rad-111': {'domain': 'Chaos', 'tags': ''},  # Mechanical Bandit
    'rad-112': {'tags': 'Ziggs|Zaun|Yordle', 'supertype': 'Champion'},  # Ziggs, Explosive
    'rad-113': {'tags': 'Bilgewater|Dragon'},  # Angler Wyrm
    'rad-114': {'tags': 'Riven|Noxus'},  # Riven, Deserter
    'rad-115': {'tags': 'Bilgewater'},  # Transpositionist
    'rad-116': {'tags': ''},  # Concert Poster
    'rad-118': {'tags': 'Shurima'},  # Herald of the Emperor
    'rad-119': {'tags': 'Demacia|Dog'},  # Hunting Dog
    'rad-120': {'tags': 'Piltover'},  # Meticulous Restorationist
    'rad-121': {'domain': 'Order'},  # One-Two Punch
    'rad-122': {'tags': 'Zaun'},  # Party Crasher
    'rad-123': {'domain': 'Order', 'tags': 'Equipment'},  # Protector's Shield
    'rad-124': {'domain': 'Order'},  # Burn Away
    'rad-125': {'tags': 'The Void'},  # Dune Devourer
    'rad-126': {'tags': 'Shadow Isles|Spirit'},  # Mournful Spirit
    'rad-128': {'tags': 'Demacia'},  # Steadfast Outrider
    'rad-129': {'tags': 'Noxus|Trifarian'},  # Trifarian Captain
    'rad-130': {'tags': 'Demacia'},  # Battlefield Armorer
    'rad-131': {'tags': 'Demacia'},  # Demacian Illuminators
    'rad-132': {'tags': 'Heimerdinger|Piltover|Yordle'},  # Heimerdinger, Academy Instructor
    'rad-133': {'tags': 'Jarvan IV|Demacia'},  # Jarvan IV, Royal Heir
    'rad-134': {'domain': 'Order'},  # Raucous Anthem
    'rad-136': {'tags': ''},  # Dockside Lock-Up
    'rad-137': {'tags': 'Mordekaiser|Noxus|Spirit'},  # Mordekaiser, Dominating
    'rad-138': {'tags': 'Seraphine|Piltover'},  # Seraphine, Not Alone
    'rad-146': {'tags': 'Orianna'},  # The Ball
    'rad-156': {'tags': 'Jarvan IV'},  # Demacian Standard
}

# textes : coût imprimé de la capacité de Runeblade Anvil (rune Fury, pas Chaos)
TEXT = [("Runeblade Anvil", "2 energy and 1 chaos rune, exhaust: Kill this.", "2 energy and 1 fury rune, exhaust: Kill this.")]
