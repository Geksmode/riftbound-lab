# Modelling cards: guide for card-module workers

The engine (`../game.py`, `../actions.py`, `../cards.py`) follows the Core Rules of 2026-07-16
(`../../rules/source/core_rules_2026-07-16.txt`, summary in `../../rules/core-rules-resume.md`). A card is
**modelled** iff its name is a key of `cards.IMPL`; the deck builder refuses every other card. So:

- one `card(name, ...)` per card name, written from the card's text (`game.SPEC[name]["text"]`, from
  `../../cards/cards_unique.csv`), with the rule numbers you rely on in comments;
- never register a placeholder or a partial behaviour: a card that you cannot model faithfully stays out of
  `IMPL` and goes into `NEEDS.md` with the reason;
- registering a name twice raises `ValueError` (no silent override), an unknown card name raises `KeyError`
  and an unknown `Impl` field raises `TypeError`.

## 1. Your module

```
engine/cardsets/<batch>.py         your cards
engine/cardsets/test_<batch>.py    their scenario tests (runnable on its own)
engine/cardsets/__init__.py        add "<batch>" to MODULES (keep the list sorted)
```

`cards.py` imports every module of `MODULES` at its end, in order, then registers the keyword-only cards that no
module modelled (`cards.AUTO`). The list is explicit (no directory scan) because the browser build (Pyodide)
fetches files by name: tell the orchestrator the new file names so the browser file list can be updated.

```python
"""Batch <batch>: <what is in it>."""
from cards import *          # card, value, enemies, friends, P_unit..., ability, parse_cost, when_level, ...

# Pouty Poro... — "<card text without reminder text>"
card("...", ...)
```

`from cards import *` gives the names in `cards.__all__` (helpers, predicates, `SPEC`, `ANY`, `DOMAINS`,
`EQUIP_BONUS`, `Item`, `Obj`, `play_card`, `attach`...). Anything else: `import cards` / `from game import ...`.
Keep helpers private to your module (`_name`) so that two modules never clash.

**Keyword-only cards are already done**: a card whose text is only keywords and reminder text (vanilla units,
`[Shield] [Tank]`, `[Vision]`, `[Weaponmaster]`, `[Hunt 3]`, tokens...) is built by `cards.auto_keywords(name)`
(see `cards.AUTO`). If you register one of them yourself, your entry wins (modules load first).

## 2. Impl fields

`card(name, **fields)` builds an `Impl`. Everything defaults to "nothing".

| field | meaning |
|---|---|
| `timing` | `None` / `"action"` / `"reaction"`: spells, and units or gear with [Action]/[Reaction] (played at that timing to their normal locations, rules 806, 813) |
| `kw` | printed keywords: `{"Tank": 1, "Assault": 2, "Shield": 1, "Deflect": 2, "Ganking": 1, "Backline": 1, "Temporary": 1, "Hunt": 2, "Vision": 1, "Weaponmaster": 1, "Unique": 1}` |
| `hidden`, `ambush`, `accelerate`, `quickdraw` | booleans for those keywords |
| `flow` | `(energy, power, domains)`; `flow_cost("4 energy and 1 fury rune")` builds it |
| `repeat` | `(energy, domains, power)`; `repeat_cost("2 energy")` builds it; the resolver must handle the repetition (`repeatable(fn)`) |
| `choices(g, pid, ctx)` | spells: list of choice dicts, best first for the AI (the AI gets the first 8, `max_choices`; a human gets them all, and `all_choices(g, pid, ctx)` when given). Targets are uids in `choice["tg"]`; other keys are free (`dest`, `item`, `kill`...). `ctx["hidden_bf"]`: battlefield index when played from facedown (targets must be there, rule 811.1.d), `ctx["card"]` |
| `preds` | one predicate per target, `pred(g, item, obj) -> bool`, checked again on resolution (rule 359.3.e); `g.legal(item, i)` returns the i-th target or `None` |
| `resolve(g, item)` | spell resolution. `item.ctrl`, `item.data` (the choice + `from`, `hidden_bf`), `item.card` |
| `on_play(g, obj, ctx)` | "When you play me/this": queue a trigger (`g.queue_trigger`). `ctx` = choice + `hidden_bf`, `src` (`hand`/`champ`/`facedown`/...) |
| `deathknell(g, item)` | [Deathknell] effect; `item.data["info"]` = look-back info: `name, ctrl, loc, might, alone, token, spec, cost, empowered, in_combat, attached`. `dk_choose(g, item)` makes its targets at finalization |
| `on_event(g, obj, ev, info)` | triggered abilities of a permanent on the board (events in section 4) |
| `abilities` | activated abilities (list of dicts, section 3) |
| `might_mod(g, obj) -> int` | passive Might modifier (any condition) |
| `might_if` | `[(cond(g, o), amount)]` dependent Might: `might_if=[(when_empowered, 2)]` |
| `kw_if` | `[(cond(g, o), {kw: value})]` dependent keywords: `kw_if=[(when_mighty, {"Deflect": 1, "Shield": 1})]` |
| `levels` | `[(N, dict(might=1, kw={"Ganking": 1}, ready=True))]` for "[Level N] I have +1 might and [Ganking]" / "I enter ready". For each field the highest Level reached wins ("instead") |
| `aura_kw(g, src, o) -> dict` | keywords this object gives to object `o` ("Other friendly units have [Vision]"). `src` is the permanent, the legend `Obj` or the `Battlefield` |
| `aura_might(g, src, o) -> int` | Might this object gives to `o` ("Your units have +1 might") |
| `enter_ready` | `True` or `fn(g, pid, card, choice)`: "I enter ready" |
| `enter_exhausted` | gear: "This enters exhausted" (gear enters ready by default) |
| `equip` | Equip cost text or tuple: `equip="1 fury rune"` adds the Equip ability (rule 818) |
| `bonus` | Equipment Might Bonus (not in the card database: read it on the card image) — stored in `EQUIP_BONUS` |
| `equip_kw` | keywords given to the equipped unit by the Effect Text: `equip_kw={"Assault": 1}` |
| `effect_event(g, gear, unit, ev, info)` | triggered Effect Text of an attached Equipment (Pendulum Blade) |
| `empower` | Empower cost text: adds the "Empower" ability (rule 827); "When I become Empowered" = event `empowered` |
| `add` | [Add] abilities usable while paying: `add=[add_ability("1 fury rune")]`, `add_ability("2 energy", can=lambda g, pid, o, ctx: ctx is not None and ctx["kind"] == "spell")`, `kill=True` for "Kill this, exhaust: Add..." |
| `cost_mod(g, pid, card, choice) -> (de, dp)` | cost reduction (energy, power) |
| `as_played(g, pid, card) -> [dict]` | optional additional costs of a unit: each dict is merged into the play choice (`kill=uid` kills a unit as a cost, `xp=n` spends XP) |
| `extra_cost_fn(g, pid, card, choice) -> (e, reqs)` | additional resource costs (e.g. "you may pay [C] as an additional cost" put in `as_played` as a flag, priced here) |
| `pay_extra(g, pid, card, choice)` | pays non-resource additional costs when the card is played (discard...) |
| `untargetable(g, obj, by_pid) -> bool` | "can't be chosen by enemy spells and abilities" |
| `uncounterable`, `banish_after`, `ignore_deflect` | "This can't be countered", "Banish this", "Ignore [Deflect] while paying this spell's cost" |
| `track_mighty` | set `True` on a card that listens to `becomes_mighty` (tracking costs time, so it is off otherwise) |
| `legend_event(g, pid, ev, info)` | legends (they are not on the board) |
| `bf_event(g, b, ev, info)` | battlefields: `b.idx`, `b.name`, `b.ctrl`, `b.owner` |
| `no_score(g, pid, b) -> bool` | battlefield: the player can't score here |

Costs are `(energy, reqs)` where `reqs` is a list of allowed-domain sets, one per Power (`[frozenset({"Fury"}), ANY]`).
`parse_cost("2 energy and 1 fury rune")` turns printed text into that.

## 3. Activated abilities

```python
ability(name, cost="1 energy and 1 calm rune", timing="main"|"action"|"reaction", exhaust=False, xp=0,
        kill_self=False, resolve=fn(g, item), choices=fn(g, pid, obj) -> [choice], preds=[...],
        can=fn(g, pid, obj) -> bool, extra_cost=fn(g, pid, obj, choice))
```
- `cost` may also be `fn(g, pid, obj, choice) -> (e, reqs)`. Deflect of enemy targets is added automatically.
- `exhaust=True` = "[E]:" ; `xp=2` = "Spend 2 XP:" ; `kill_self=True` = "Kill this:".
- On resolution `item.src` is the uid of the source (`g.obj(item.src)`; for a legend it is `g.p[pid].legend.uid`).
- Legends use the same `abilities` list (see Akali, Rogue Assassin in `cards.py`).

## 4. Events (`g.emit(ev, **info)`)

Delivered to `on_event` of every permanent on the board, `effect_event` of attached gear, `bf_event` of both
battlefields, `legend_event` of both legends, then the `g.effects` entries with `on == ev`.

| event | info |
|---|---|
| `played` | `pid, card, item, n` (n-th card whose play completed this turn; spells: after resolution) |
| `move` | `obj, frm, to, by` (`to`/`frm`: `"base"` or battlefield index 0/1) |
| `attack` / `defend` | `obj, bf` (once per unit per combat) |
| `combat_start` | `bf, sd` ; `showdown_start`: `bf, combat, sd` |
| `combat_won` | `pid, bf` ; `combat_end`: `bf, members` |
| `conquer` / `hold` | `pid, bf, units` (units of pid there) |
| `die` | `info` (same look-back dict as Deathknell) |
| `damaged` | `obj, amount, kind ("spell"/"ability"/"combat"), by` |
| `chosen` | `obj, item` (a chain item chose obj) |
| `ready` | `obj` ; `stun`: `obj, by` ; `buff`: `obj` ; `empowered`: `obj` ; `becomes_mighty`: `obj` |
| `attached` | `gear, unit` |
| `draw` | `pid, card` ; `discard`: `pid, card` ; `burn`: `pid, cards` ; `recycle`: `pid, cards` |
| `channel` | `pid, runes` ; `hide`: `pid, card, bf` ; `point`: `pid, why` |
| `xp_gain` / `xp_spend` | `pid, n` |
| `beginning_start`, `main_start`, `end_turn` | `pid` (turn player) |

Inside an event handler never change the game directly for a triggered ability: queue it.
```python
g.queue_trigger(pid, "Card name", resolve_fn, data=dict(...), src=obj.uid, may=False,
                choose=trig_target(options_fn, pred, deflect=True), cost=may_pay(2))
```
`may=True` asks "you may" at finalization (rule 383.3.a); `choose` makes targets at finalization (return False to
remove the trigger); `cost` pays a cost within instructions (rule 383.3.b). Use `deflect=True` in `trig_target`
when the trigger can choose enemy units (rule 809). Static "this turn" effects: `g.effects.append(dict(on=ev,
fn=fn(g, eff, info), dur="turn"))` (removed at end of turn).

## 5. The `g` API most cards need

| call | |
|---|---|
| `g.units(pid=None, loc="any")`, `g.gear(pid)`, `g.obj(uid)`, `g.board` | permanents |
| `g.p[pid]`: `.hand .deck .trash .banish .champ .runes .rune_deck .legend .legend_name .xp .points .pool_e .pool_p` | player |
| `g.tp`, `g.turn_no`, `g.stage`, `g.sd` (showdown: `.bf .combat .attacker .defender .assigned`), `g.bfs[i]` | state |
| `g.might(o)`, `g.mighty(o)`, `g.has_kw(o, kw)`, `g.kw_value(o, kw)`, `g.alone(o)`, `g.in_combat(o)`, `g.targetable(o, pid)` | queries |
| `g.deal(o, n, "spell"/"ability", pid)`, `g.kill([objs], pid)`, `g.stun(o, pid)`, `g.buff(o)`, `g.mod(o, n, "turn", minimum=None)`, `g.grant(o, kw, n, "turn"/None)` | effects |
| `g.move([objs], dest, pid)`, `g.recall(o)`, `g.ready_obj(o)`, `o.exhausted = True`, `g.empower(o)`, `g.disempower(o)` | |
| `g.draw(pid, n)`, `g.discard(pid, card)`, `g.burn(pid, n)`, `g.predict(pid, n)`, `g.channel(pid, n, exhausted=)`, `g.recycle_cards(pid, cards)`, `g.recycle_rune(pid, r)`, `g.to_zone(card, "hand"/"trash"/"banish"/"deck")` | cards |
| `g.gain_xp(pid, n)`, `g.spend_xp(pid, n)`, `g.level(pid, n)`, `g.legion(pid, card)`, `g.gain_point(pid, why)` | XP, Level, Legion |
| `g.add_pool(pid, e, ["Fury", "A"])`, `g.can_pay(pid, e, reqs)`, `g.pay(pid, e, reqs)` | resources ([Add] from a trigger) |
| `make_token(g, "Recruit", pid, loc, ready=False)`, `g.new_token(name, pid)` + `g.enter_board(t, pid, loc, ready)` | tokens: Recruit, Sprite, Mech, Sand Soldier, Bird, Gold, Tentacle, Shadow Clone, Reflection (`reflection_token`) |
| `g.counter(item)`, `item_by_id(g, id)`, `g.chain` | chain |
| `g.ask(pid, kind, options, **ctx)` | a choice by a player; `None` if no option, the option itself if only one (except kinds `may`, `may_target`, `order`). Order options best first: the AI (`ai.py`) takes `options[0]` for unknown kinds |
| `play_card(g, pid, card, src, choice, limited=True)`, `play_unit_free(g, pid, card, src, loc_choices, ignore_energy, alt_cost=None)` | playing a card from an effect; `alt_cost=(e, reqs)` is added to the cost (Flame Chompers: `(0, [FURY])`), with `ignore_energy` the remaining costs (Accelerate...) are still paid |
| `attach(g, gear, unit)`, `equip_cost(g, pid, gear, unit)` | equipment |

Helpers in `cards.py`: `value(g, o)` (AI ordering), `enemies(g, pid, at_bf, hidden_bf)`, `friends(...)`,
`all_units(...)`, `tg_choices(units)`, predicates `P_unit P_unit_bf P_enemy P_enemy_bf P_friend P_friend_bf P_gear`,
conditions `when_level(n) when_empowered when_mighty when_legion when_at_bf`, `repeatable(fn)`, `pay_deflect`,
`keywords_of(name)`, `deck_problems(deck)`, `is_token(name)`.

## 6. Worked examples (all in `cards.py`)

- **Spell with targets** — Back Off: `timing="action", hidden=True, preds=[P_unit], resolve=_back_off,
  choices=lambda g, pid, ctx: tg_choices([... enemies(g, pid, False, ctx["hidden_bf"]) ...])`. The resolver
  re-checks the target with `g.legal(it, 0)` and uses `it.data.get("from") == "hand"`. Two targets: Falling Star
  (`preds=[P_unit, P_unit]`, `g.legal(it, 1)`).
- **Unit with a play trigger** — Harnessed Dragon: `on_play` queues a trigger whose target is chosen at
  finalization with `trig_target(lambda g_, it: enemies(g_, it.ctrl, False, hb), P_enemy)`.
- **Deathknell** — Ruined Rex: `deathknell=_rex_dk, dk_choose=trig_target(...)`; Lonely Poro reads
  `it.data["info"]["alone"]`.
- **Triggered ability on an event** — Kai'Sa, Survivor (`ev == "conquer" and o in info["units"]`), Akali,
  Deadly Weapon (`move`, optional target), Vi, Peacekeeper (`attack`).
- **Legend** — LeBlanc, Deceiver: `legend_event=fn(g, pid, ev, info)` with a cost paid at finalization;
  Akali, Rogue Assassin: `abilities=[Empower, Retreat]`.
- **Battlefield** — Sigil of the Storm: `bf_event` on `conquer` with `info["bf"] == b.idx`; Forgotten Monument:
  `no_score`.
- **Gear with Equip** — Long Sword: `quickdraw=True, abilities=[equip_ability("Fury")]` + `EQUIP_BONUS`;
  Pendulum Blade: `effect_event`. New style: `card("Serrated Dirk", equip="1 fury rune", bonus=N)`.
- **Generic keywords** (tests in `test_keywords.py`): Gustwalker
  `card("Gustwalker", kw={"Hunt": 2}, levels=[(3, dict(might=1, kw={"Ganking": 1}))])`; Fiora, Victorious
  `kw_if=[(when_mighty, {"Deflect": 1, "Ganking": 1, "Shield": 1})]`; Apprentice Mage
  `empower="2 energy", might_if=[(when_empowered, 1)], on_event=<empowered -> predict 2>`; Seal of Rage
  `add=[add_ability("1 fury rune")]`; Desert's Call `repeat=repeat_cost("2 energy"), resolve=repeatable(fn)`.

## 7. Scenario tests

Copy the style of `../test_cards.py`, with the helpers of `../testkit.py`:

```python
"""Tests of batch <batch>. Run: python3 cardsets/test_<batch>.py"""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from testkit import *

T = Suite("<batch>")

@T.test
def gustwalker_ganking_at_level_3():
    g, ag = new()                      # Akali (P0) vs LeBlanc (P1), P0's Main Phase, empty hands
    u = put(g, 0, "Gustwalker", 0)     # on the board at battlefield 0 (P0 takes control of it)
    g.gain_xp(0, 3)
    assert g.has_kw(u, "Ganking") and g.might(u) == 4

@T.test
def my_spell():
    g, ag = new(answers0={"predict_recycle": True})   # scripted answers to g.ask, by kind
    runes(g, 0, ["Mind"] * 3)
    e = put(g, 1, "Mountain Drake")
    hand(g, 0, "Eclipse")
    g.apply(opt(g, 0, "Eclipse", lambda ch: ch.get("tg") == (e.uid,)))   # play it
    settle(g)                           # both players pass until the next Main decision
    assert g.might(e) == ...

if __name__ == "__main__":
    T.main()
```
Other helpers: `deck_top(g, pid, names)`, `champ(g, pid, name)`, `options_of(g, kind)`, `act_options(g, pid, name)`,
`temp_card(name, **fields)` (temporary Impl inside a `with`), `total_cost`, `card_choices`. At least one test per
card, plus one per rule interaction you had to decide. Then:

```bash
python3 cardsets/test_<batch>.py      # your tests
python3 test_all.py                   # everything (test_cards.py + every cardsets/test_*.py)
python3 fuzz_cards.py 100 --module <batch>   # random games with your cards: 0 exceptions, deterministic
```

## 8. Rules for every module

- **Determinism / Pyodide**: the engine runs in the browser (Pyodide, 32-bit wasm, Python 3.12): no new
  dependency, standard library only. Never let the order of a `set` (or `dict` built from one) of tuples or
  objects decide anything: iterate `sorted(...)` or lists. Use `g.rng` for randomness, never `random`.
  The same seed must give the same game.
- **Don't edit `game.py`, `actions.py` or `cards.py`.** If a needed engine hook is missing, implement it inside
  your module if you can (an `on_event`, a `g.effects` entry, a wrapper around a helper); otherwise add a line to
  `NEEDS.md` with the card name, what is missing and the rule number, and leave the card out of `IMPL`.
- Don't touch `train.py`, `train_games.py` or `../train/`.
- Shared folder: re-read a file just before changing it, keep edits small, read it back a few seconds later.
- **Human players** (`train.py`): every `g.ask` kind needs a French title, `ask_text(my_kind="Quelle carte
  prendre ?")` (or an entry in `cards.py`); `test_cards.py` checks it. Options that are not cards, runes,
  locations or booleans should be `Opt(label, value)` (the answer is the `Opt`; use `.value`). Never cap a list of
  legal choices with a bare slice: `cap(g, seq, n, pid)` keeps the AI's first n and gives a human all of them
  (`full_choices(g, pid)`); a combinatorial choice (subsets, damage splits) adds the full set when
  `full_choices(g, pid)` is true, or gives `Impl.all_choices`.

## 9. Shared hooks

Impl fields read on *other* objects (`cards.HOOK_FIELDS`; `g.providers(field)` yields `(src, impl)` for the board,
legends and battlefields; `fx_` fields are read on attached Equipment). The signature is in the docstring of the
engine function that reads it.

| field | use |
|---|---|
| `death_rep`, `fx_death_rep` | "would die ... instead" (`Game.kill`): returns dicts `name, apply, may, cost(g), key, multi` |
| `dmg_bonus`, `fx_dmg_bonus`, `dmg_prevent`, `aura_dmg_prevent`, `lethal_any` | damage (`Game.deal`, `Game.lethal`) |
| `no_combat_damage`, `aura_no_combat`, `ignore_tank` | combat damage |
| `cost_aura(g, src, pid, what)` | cost changes of other cards / abilities (`actions.cost_mods`; `min_e` floor) |
| `extra_locs`, `aura_locs`, `loc_veto`, `only_locs`, `forbid_play`, `card_kw` | where / whether a card can be played, keywords outside the board |
| `units_enter_ready`, `cant_move`, `fx_cant_move`, `aura_cant_move`, `move_tax` | entering, moving |
| `grant_abilities(g, src, obj)`, `grant_add(g, src)` | activated / [Add] abilities given to other objects (Heimerdinger) |
| `aura_untargetable`, `aura_tags`, `no_ready`, `aura_no_ready`, `channel_mod`, `score_veto` | targeting, tags, readying, channelling, scoring vetoes |
| `point_rep`, `scoring_extra`, `fx_swap_scoring` | scoring: replace a point, conquer/hold triggers an extra time |
| `choice_rep`, `minus_extra`, `no_counter`, `gold_extra`, `hide_cost`, `hide_slots`, `token_rep` | see `Game` / `actions` docstrings |
| `zone_event`, `on_seen`, `on_reveal`, `reveal_rep` | cards outside the board (`g.zone_listen` is set while a listener can be affected) |

Other shared tools:
- `g.effects` kind `play_perm` (`pid, key, fn(g, eff, card, src) -> bool, choice`): a permission to play a card in
  another way (Jayce, Man of Progress); the play choice carries `perm=key` and the permission is used up.
- `g.effects` kind `cost_mod` with `once=True` is consumed by the next play.
- `g.replace_battlefield(b, name)` (rule 438: the Battlefield object, its control and statuses stay; `b.replaced`
  is the original name; event `bf_replaced`) and `g.swap_back(b)`: Ivern, Green Father makes the "Brush" token
  battlefield (in `TOKEN_NAMES`, never in a deck).
- `alt_costs(g, pid, card, src) -> [dict(key=, e=, reqs=)]`: alternative costs (rule 356.1.b).
- Keep `cardsets/__init__.py` MODULES sorted, one name per line.
