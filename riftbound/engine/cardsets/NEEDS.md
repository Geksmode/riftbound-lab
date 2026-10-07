# Engine hooks still missing

Card workers: add one line per card you could not model without changing `game.py` / `actions.py` / `cards.py`
(the card stays out of `IMPL`). Format: `- **Card name** (module) — what is missing — rule`.
The engine maintainer will add the hook and the card can then be modelled.

## Known limits of the foundation (2026-10-07)

- **Deflect on triggered abilities** — `trig_target(..., deflect=True)` makes a trigger pay Deflect (rule 809,
  383.3), but the triggers of the 88 cards modelled before the card modules (Akali, Deadly Weapon; Kennen; Vi;
  Harnessed Dragon; Ruined Rex; Blitzcrank...) keep `deflect=False` so that their behaviour is unchanged.
- **Decree of Insight** — "Ignore [Deflect] while paying this spell's cost" is not applied (the old entry has no
  `ignore_deflect=True`; left unchanged on purpose).
- **Empowered event** — `g.empower(o)` emits `empowered`; the old entries (Akali, Deadly Weapon; Mournful
  Witness; Akali, Rogue Assassin) still set `o.empowered = True` directly, without the event.
- **Restricted energy** — the rune pool has no per-resource restrictions. "Spend this Energy only during
  showdowns / only to play units" is modelled on the [Add] ability itself, which is only used while paying
  (`add_ability(..., can=...)`, `ctx = dict(kind="spell"|"unit"|"gear"|"ability"|"hide", card=, obj=)`):
  Diana, Scorn of the Moon; Renekton, Butcher of the Sands.
- **[Add] abilities** are used automatically while paying (rule 429.3), never activated alone to float
  resources. Costs: exhaust, "Kill this", conversions (`conv=`: runes to energy, energy to power, "pay any
  amount" with `var=True`, a kill as the cost: Renekton, Hextech Anomaly, Ancient Henge, Malzahar) and [Add]
  abilities granted by other objects (`grant_add`: Heimerdinger).
- **Becomes Mighty** (rule 709) is detected at cleanups, only when a card with `track_mighty=True` is
  registered; a unit that enters the board with 5+ Might does not "become" Mighty.
- **Equipment** Might Bonus and Effect Text are not in the card database: give `bonus=` (and the Effect Text)
  from the card image; `auto_keywords` never registers Equipment for that reason.
- **Tokens played by effects** (`make_token`) do not emit `played` and do not get Vision/Weaponmaster play
  triggers (same as the old Mech/Gold/Reflection code).
- **Hidden restrictions** of Weaponmaster's choice and of keyword triggers are not applied (rule 811.1.d.2).
- **Baron Nashor / Baron Pit** — need a third battlefield (battlefields are a pair everywhere in the engine):
  see `NEEDS_battlefields.md`.
- 3-4 player modes are not implemented.

## Requests from card modules

(none yet)
