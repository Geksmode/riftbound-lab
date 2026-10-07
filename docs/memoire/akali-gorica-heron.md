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
