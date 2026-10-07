---
name: retex-method
description: What the user means by "retex" (self-critical lessons learned) and the simulation-statistics rules learned from it
metadata:
  type: feedback
---
"Retex" for this user = a factual, self-critical review of what Claude learned/claimed in the project: what holds, what was overstated, which biases, what to change. Not a web-collection of tournament reports (that interpretation was wrong, 2026-10-02).

**Why:** the user wants to counter biases and be "très factuelle et juste"; the first retex found several overstated claims (G2 +8.7 -> +6.6 on fresh seeds; initiative 14 -> 5-10 pts; Block/NSF claim unsupported).

**How to apply:** in every simulation study ([[riftbound-engine]]): re-run any selected option on fresh seeds (exp.run_job offset) and only announce that number; never pool configs played on the same seeds as independent; give intervals and say "on ne sait pas" under 2 SE; cite a field number only if read on the page; ask a closed question when a request is ambiguous. Reports: riftbound/retex/retex-etude-akali-leblanc.md; riftbound/retex/retex-table-entrainement.md (2026-10-06, tool retex: say "testé" only for paths played end to end in a real game, never for injected states; sims before 2026-10-06 lack floating energy, re-measure before citing). Related: [[akali-gorica-heron]].
