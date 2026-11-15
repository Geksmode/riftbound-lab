---
name: akali-gorica-heron
description: User plays Gorica's Akali Heron (RQ Singapore); manager sim findings vs LeBlanc: plan ~+10, aggro/Void Gate cost, lists equal
metadata:
  type: user
---
2026-10-02: the user said "je joue Akali heron de gorica": their list is Gorica's RQ Singapore Akali (decks/raw/akali_gorica_rq-singapore.txt), not the Wuhan list of [[akali-vs-leblanc]]. Base Akali analyses on it. Retex 2026-10-02: keep the Gorica main deck; G2 (−2 Long Sword −3 Defy +Akali Silent +2 Ferrous Forerunner +Lonely Poro +Scuttle Crab) is at most a side idea. Early "G2 52.9% vs stock 44.2%" was selection-biased (+6.6 ± 2.8 unbiased, only when nobody follows a plan); "keep Block/NSF = 10 pts" and early battlefield rankings are NOT supported; "LeBlanc sidée" is invented.

Manager sessions ([[riftbound-manager]]; full tables in riftbound/manager/learnings.md). Never pool across engine/AI versions.

OLD engine version (s004-s015, tempo AI + LeBlanc "réel"), opponent leblanc_gyatarina_ccs-iq5_1st:
- Lists stock/G2/GL/GD within ±4 pts (G2 − stock +0.4 ± 1.9 on 1,200 paired); GL/"cut Long Sword not Defy" refuted. Default = stock list.
- Gorica plan vs no plan +10.3 ± 1.6 (both LeBlanc plans); at equal battlefield +10.7 ± 2.3 (gain is in-game decisions).
- Aggressive plan −5.1 ± 1.5.
- Vs Windswept present Sigil: Void Gate −7.9 ± 2.3, Forgotten Monument −5.6 ± 1.9. Vs Star Spring keep plan's Void Gate (Sigil −2.1 ± 3.3, piste).
- Windswept presented by LeBlanc costs Akali 4.6 ± 1.0; going first +9.0 ± 2.2.
- LeBlanc Hook tempo vs Deathknell no difference (−0.5 ± 2.4). REF level ~48%.

NEW engine (s016+, engine 014259caa8, rune rules of 2026-10-06: recycled rune exhausts first + floating energy):
- s016-s018 pooled (s018 on the refactored engine of 2026-10-07, outcomes verified identical): plan vs none +13.0 ± 2.4, aggro −7.3 ± 2.2, Void Gate −9.3 ± 2.2, Forgotten Monument −9.0 ± 3.2 (one block), Windswept +3.4 ± 1.3 — all old advice holds. Going first +4.1 ± 2.9 unsure. REF 47.6%.
- s019 planned: LeBlanc plan Hook tempo − Deathknell, G2 − stock.

2026-11-15 — ban du 2026-11-23 (Stellacorn Herder, Zhonya's Hourglass) : candidats par lecture des cartes, NON mesurés.
Rôles perdus : Stellacorn = pioche à chaque déplacement (légende Akali, Shuriken Flip) ; Zhonya = sauver une unité de la mort
(recall), caché pour 0 énergie. Pistes : à 4-5 → Akali, Silent (5/6 sideboards Akali), Pakaa Protector (pioche au déplacement
si unité, ~47 % ; sinon +2 might), Scuttle Crab (pioche en jouant, 5/6 mains Akali) ; protection → Unlicensed Armory (seul
« recall au lieu de mourir » non banni, coûte une défausse + 1 fury), Twilight Shroud, +1 Not So Fast, +1 Block, Spirit's Refuge.
Mesuré ensuite (2026-11-15), voir le bloc « Résultats » plus bas.
Radiance (lecture des cartes, non mesuré) : Akali, Brash (5e+1 Fury, 5 ; 1er déplacement du tour = +1 énergie +1 rune, 2e = carte
du dessus jouable : seul vrai « avantage au déplacement » ; 1 copie dans une Akali de Sacramento, 482e) ; Stoic Ferryman (4e Calm,
2/Shield 3 ; joué sur un battlefield, renvoie une autre de tes unités à la base = sauvetage + déplacement) ; Consult the Heavens,
Hunker Down, Life from Stone (cantrips Calm à 1). Rien en Radiance ne sauve de la mort comme Zhonya. riftbound.gg : seule Hush
(RAD-041) entre dans le sideboard Akali.

2026-11-15 — Résultats `exp_akali_ban.py` (`results_akali_ban.json`), Gorica contre LeBlanc IQ#5, IA sans plan des deux côtés.
Limites : un seul adversaire, qui garde Baited Hook / Star Spring (bannies) ; le plan Gorica tourne autour de Stellacorn.
- s1 (10 remplaçants de Stellacorn x3, Zhonya → +2 Not So Fast +1 Block, 300 parties, graines 710000+) : Lonely Poro 33,3 %
  ± 2,7, Ferrous Forerunner 33,0, Scuttle Crab 31,3, Jhin 30,7, Irelia 30,0, Ahri 28,3, Pakaa 28,0, Akali Silent 27,7,
  Akali Brash 27,3 : tous sous 2 écarts-types du 1er (apparié) ; seul Adaptatron est nettement pire (−10,7 ± 3,0).
- s2 (Poro + 10 remplaçants de Zhonya x3, 300 parties, graines 720000+) : Scuttle Crab 42,0 % ± 2,9, Stoic Ferryman 37,0
  (−5,0 ± 3,0, on ne sait pas), Hush 36,0 (on ne sait pas) ; nettement pires que Crab : En Garde, Twilight Shroud, Spirit's
  Refuge, +2 NSF +1 Block, Unlicensed Armory, Charm, Consult the Heavens (28,0).
- s3 (graines NEUVES 730000+, 500 parties) : Forerunner+Ferryman 41,0 % ± 2,2, Poro+Ferryman 40,8, Forerunner+Crab 40,8,
  Poro+Crab 40,6 : indiscernables (écarts ≤ 0,4 ± 2,6). Témoins sur les mêmes graines (Zhonya → +2 NSF +1 Block) :
  Forerunner 37,4 %, Poro 33,8 %. Apparié : avec Poro, Crab +6,8 ± 2,3 et Ferryman +7,0 ± 2,4 (confirmé) ; avec Forerunner,
  Crab +3,4 ± 2,6 et Ferryman +3,6 ± 2,5 (on ne sait pas).
- Conclusion : 3 Lonely Poro ou 3 Ferrous Forerunner pour Stellacorn (on ne sait pas lequel), 3 Scuttle Crab ou 3 Stoic
  Ferryman pour Zhonya (on ne sait pas lequel) ; ne pas remplacer Zhonya par des contres (Not So Fast, Block), ni par
  Armory, Charm ou Consult. Le deck reste vers 41 % contre cette LeBlanc avec l'IA sans plan.
- Les 2 témoins portent engine_md5 f274c70e7b au lieu de 93e9f7586d : seule différence, l'ajout de l'option `temoin` dans
  `exp_akali_ban.py` (vérifié : le hachage recalculé avec l'ancien script redonne 93e9f7586d) ; le moteur de jeu est identique.

