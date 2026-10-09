"""Legal discretionary actions (rule 410) and the process of playing cards and abilities (rules 353-359, 398-406).

Actions are tuples that only reference objects by uid, so the same action can be applied to a cloned game:
  ('end',) ('pass',)
  ('play', uid, src, choice)      src in hand|champ|facedown|trash(flow)
  ('hide', uid, bf)
  ('act', src_uid, ability_index, choice)   src_uid = 'legend' for the player's legend
  ('move', (uids...), dest)
"""
from game import Item, ANY, SPEC, EQUIP_BONUS
from itertools import count

_ids = count(1)
MAX_CHOICES = 8


def card_obj(g, pid, uid, src):
    pl = g.p[pid]
    zone = {"hand": pl.hand, "champ": pl.champ, "trash": pl.trash}.get(src)
    if src == "facedown":
        for b in g.bfs:
            for c in b.facedowns:
                if c.uid == uid:
                    return c
        return None
    for c in zone or []:
        if c.uid == uid:
            return c
    return None


# ---------------------------------------------------------------- timing
def has_timing(g, pid, im, src, closed, loc_ok_ambush=False):
    """Can this card be played now? closed=True: Closed state (needs Reaction).
    closed=False and showdown: needs Action or Reaction. Neutral Open on own turn: anything."""
    if src == "facedown":
        return True                                  # facedown Hidden cards have Reaction (rule 811.6)
    t = im.timing
    if im.quickdraw:
        t = "reaction"
    if closed:
        return t == "reaction" or loc_ok_ambush
    return t in ("action", "reaction") or loc_ok_ambush


def state_neutral_open_main(g, pid):
    return not g.chain and g.sd is None and g.stage == "main" and g.tp == pid


# ---------------------------------------------------------------- cost
def flow_of(g, pid, card):
    """[Flow] cost of a card in the trash: printed (Impl.flow) or granted this turn (g.effects kind='grant_flow'
    with uid/oid: Kennen, Storm of Shuriken). (energy, power, domains) or None."""
    im = g.impl(card)
    if im is not None and im.flow:
        return im.flow
    for ef in g.effects:
        if ef.get("kind") == "grant_flow" and ef["uid"] == card.uid and ef["oid"] == card.oid:
            return ef["flow"]
    return None


def card_kw(g, pid, card, kw, src=None):
    """Keywords a card has outside the board, from its Impl or granted by other objects (Impl.card_kw(g, src_obj,
    pid, card, kw, zone) -> bool: Jax, Unmatched gives [Quick-Draw] to Equipment in hand, Rek'Sai, Breacher gives
    [Accelerate] to units played from anywhere other than a hand)."""
    im = g.impl(card)
    if im is None:
        return False
    own = {"Quick-Draw": im.quickdraw, "Accelerate": im.accelerate, "Hidden": im.hidden, "Ambush": im.ambush}.get(kw)
    if own:
        return True
    for so, ia in g.providers("card_kw"):
        if ia.card_kw(g, so, pid, card, kw, src):
            return True
    return False


def repeat_instances(g, pid, card):
    """Granted [Repeat] costs of a spell (rule 820.1.c.2: each instance can be paid once): g.effects kind=
    'grant_repeat' (pid, cost=fn(g, card) -> (e, reqs) or None, next=True for "your next spell") and auras
    (Impl.card_kw(..., 'Repeat') returning (e, reqs)). Returns [(key, e, reqs)]."""
    out = []
    for i, ef in enumerate(g.effects):
        if ef.get("kind") == "grant_repeat" and ef["pid"] == pid:
            c = ef["cost"](g, card)
            if c is not None:
                out.append((("eff", ef.get("key", i)), c[0], list(c[1])))
    for so, ia in g.providers("card_kw"):
        c = ia.card_kw(g, so, pid, card, "Repeat", None)
        if c:
            out.append((("src", getattr(so, "uid", 0), so.name if hasattr(so, "name") else ""), c[0], list(c[1])))
    return out


def _opt_parts(g, pid, card, im, choice, base_e, base_reqs):
    """Optional additional costs paid with this choice: [(tag, e, reqs)] (tags 'acc', 'rep', 'opt'; 'must' for a
    mandatory part that comes with an optional one, e.g. the Deflect of a repetition's target: never discounted).
    Impl.opt_parts(g, pid, card, choice) replaces the default reading (Accelerate, Impl.repeat, extra_cost_fn)."""
    if im.opt_parts is not None:
        return [(t, e, list(r)) for t, e, r in im.opt_parts(g, pid, card, choice)]
    parts = []
    if choice.get("acc"):
        parts.append(("acc", 1, [frozenset(card.spec["domains"]) or ANY]))
    if choice.get("rep") and im.repeat:
        parts.append(("rep", im.repeat[0], [frozenset(im.repeat[1]) or ANY] * im.repeat[2]))
    if im.extra_cost_fn is not None:
        x = im.extra_cost_fn(g, pid, card, choice)
        if x[0] or x[1]:
            parts.append(("opt", x[0], list(x[1])))
    return parts


def cost_mods(g, pid, what):
    """Cost modifiers of other objects and effects (rule 356.3-356.4): Impl.cost_aura(g, src, pid, what) and
    g.effects kind='cost_mod' fn(g, eff, pid, what), each returning a list of dicts:
      e: +n increase / -n discount of Energy; p: [reqs] Power increase; rm: n "1 rune of any type less";
      flex: n "1 energy or 1 rune of any type less"; min_e: "to a minimum of N energy" (that discount only);
      on: 'all' (default) | 'opt' (each optional additional cost) | 'rep' (each Repeat cost).
    what = dict(kind='spell'|'unit'|'gear'|'ability', card=, obj=, ab=, choice=, src=)."""
    out = []
    for so, ia in g.providers("cost_aura"):
        for m in ia.cost_aura(g, so, pid, what) or ():
            out.append(m)
    for ef in list(g.effects):
        if ef.get("kind") == "cost_mod" and ef.get("pid", pid) == pid:
            for m in ef["fn"](g, ef, pid, what) or ():
                out.append(dict(m, _eff=ef))
    return out


def _drop_reqs(reqs, n):
    """Remove n Power requirements, the most restrictive first ("costs 1 rune of any type less")."""
    reqs = sorted(reqs, key=lambda r: (len(r), sorted(r)))
    return reqs[n:]


def _flex(variants):
    """One "1 energy or 1 rune of any type less" discount applied to each variant (energy first)."""
    out = []
    for ve, vr in variants:
        nv = []
        if ve > 0:
            nv.append((ve - 1, vr))
        if vr:
            nv.append((ve, _drop_reqs(vr, 1)))
        for x in nv or [(ve, vr)]:
            if x not in out:
                out.append(x)
    return out


def _apply_mods(g, pid, e, reqs, parts, mods, ctx):
    """Apply cost increases then discounts (rule 356.3-356.4) to a base cost and its optional additional costs
    (parts). When a discount lets the player pick energy or rune, the first payable variant is used (energy first).
    Returns (e, reqs)."""
    reqs = list(reqs)
    whole = [m for m in mods if m.get("on", "all") == "all"]
    for m in whole:                                    # increases first (rule 356.3)
        if m.get("e", 0) > 0:
            e += m["e"]
        reqs += list(m.get("p", ()))
    variants = [(e, reqs)]
    for tag, pe, pr in parts:                          # discounts of each optional / Repeat cost
        pv = [(pe, list(pr))]
        for m in mods:
            on = m.get("on", "all")
            if on == "all" or (on == "rep" and tag != "rep") or tag == "must":
                continue
            if m.get("e", 0) < 0:
                pv = [(max(0, a + m["e"]), b) for a, b in pv]
            if m.get("rm"):
                pv = [(a, _drop_reqs(b, m["rm"])) for a, b in pv]
            for _ in range(m.get("flex", 0)):
                pv = _flex(pv)
        variants = [(ve + a, vr + b) for ve, vr in variants for a, b in pv]
    for m in whole:                                    # discounts of the whole cost (rule 356.4)
        if m.get("e", 0) < 0:
            floor = m.get("min_e")
            variants = [((max(ve + m["e"], min(ve, floor)) if floor is not None else max(0, ve + m["e"])), vr)
                        for ve, vr in variants]
        if m.get("rm"):
            variants = [(ve, _drop_reqs(vr, m["rm"])) for ve, vr in variants]
        for _ in range(m.get("flex", 0)):
            variants = _flex(variants)
    if len(variants) > 1:
        for ve, vr in variants:
            if g.can_pay(pid, ve, vr, ctx):
                return ve, list(vr)
    return variants[0][0], list(variants[0][1])


def total_cost(g, pid, card, choice, src):
    """Rule 356: base cost (modified), additional costs, increases, discounts. Returns (energy, reqs)."""
    im = g.impl(card)
    sp = card.spec
    e, p = sp["e"], sp["p"]
    doms = sp["domains"]
    alt = None
    if src == "facedown" or choice.get("free"):
        e, p = 0, 0                                    # Hidden: ignore its base cost
    elif choice.get("alt") is not None:
        alt = choice                                   # alternative cost (rule 356.1.b) instead of the base cost
        e, p = choice["alt_e"], 0
    elif choice.get("flow"):
        fl = flow_of(g, pid, card)
        e, p = fl[0], fl[1]
        doms = fl[2] if len(fl) > 2 else doms
    if choice.get("ignore_energy"):
        e = 0
    if im.cost_mod and src != "facedown" and not choice.get("free"):
        de, dp = im.cost_mod(g, pid, card, choice)
        e, p = max(0, e - de), max(0, p - dp)
    reqs = g.power_reqs(doms, p)
    if alt is not None:
        reqs = list(alt["alt_reqs"])
    # optional additional costs (Accelerate, Repeat, "you may pay ... as an additional cost")
    parts = _opt_parts(g, pid, card, im, choice, e, reqs)
    for r in choice.get("reps", ()):
        parts.append(("rep", r["_e"], list(r["_reqs"])))       # granted Repeat instances paid
    # mandatory additional costs of the location (Dragon Roost) and Deflect: enemy objects chosen (rule 809)
    if choice.get("loc_reqs"):
        reqs += list(choice["loc_reqs"])
    reqs += deflect_total(g, pid, card, choice)
    what = dict(kind=sp["type"].lower(), card=card, obj=None, ab=None, choice=choice, src=src)
    mods = cost_mods(g, pid, what) if (g.effects or cards_hooks().get("cost_aura")) else []
    e, reqs = _apply_mods(g, pid, e, reqs, parts, mods, pay_ctx(card))
    # discounts: Astral Heron "your next card costs [2] and [A][A] less"
    disc = [ef for ef in g.effects if ef.get("kind") == "heron" and ef["pid"] == pid]
    for ef in disc:
        e = max(0, e - 2)
        for _ in range(2):
            if reqs:
                # drop the most restrictive requirement first
                reqs.sort(key=len)
                reqs.pop(0)
    return e, reqs


def cards_hooks():
    import cards
    return cards.HOOKS


def deflect_reqs(g, pid, choice):
    """Deflect (rule 809): [A] per Deflect value for each enemy object chosen in choice['tg'] (ignored for objects
    at a battlefield whose players "ignore [Deflect] while paying for spells and abilities choosing something here":
    Heisho, Shell of the World)."""
    out = []
    for uid in choice.get("tg", ()):
        o = g.obj(uid) if isinstance(uid, int) else None
        if o is not None and o.ctrl != pid:
            if o.loc in (0, 1) and g.bfs[o.loc].name == "Heisho, Shell of the World":
                continue
            out += [ANY] * g.kw_value(o, "Deflect")
    return out


def deflect_total(g, pid, card, choice):
    """Tout le coût de Deflect d'un sort joué avec ce choix (809.1.c : « pour chaque fois qu'ils me choisissent ») :
    cibles du sort, puis cibles de chaque répétition payée (Repeat imprimé : tg2 ; Repeat accordé : reps), car une
    répétition choisit ses propres cibles (820)."""
    im = g.impl(card)
    if im is not None and im.ignore_deflect:
        return []
    out = deflect_reqs(g, pid, choice)
    if choice.get("rep"):
        out += deflect_reqs(g, pid, dict(tg=choice.get("tg2", ())))
    for r in choice.get("reps", ()):
        out += deflect_reqs(g, pid, dict(tg=r.get("tg", ())))
    return out


def pay_ctx(card):
    return dict(kind=card.spec["type"].lower(), card=card)


def affordable(g, pid, card, choice, src):
    e, reqs = total_cost(g, pid, card, choice, src)
    return g.can_pay(pid, e, reqs, pay_ctx(card))


def unit_timing_ok(im, closed):
    """A unit or gear with [Reaction] (or [Action] outside Closed states) can be played at that timing,
    still only to its normal locations (rules 806.3, 813.3.a)."""
    return im.timing == "reaction" or (im.timing == "action" and not closed)


# ---------------------------------------------------------------- enumeration
def loc_allowed(g, pid, card, loc):
    """A location where a card (or token) may be played: "Units can't be played here" (Rockfall Path), "opponents can
    only play units to their base" (Mageseeker Warden)... (Impl.loc_veto(g, src, pid, card, loc) of other objects,
    g.effects kind='loc_veto') and the card's own "Play me only to ..." (Impl.only_locs(g, pid, card))."""
    im = g.impl(card)
    if im is not None and im.only_locs is not None and loc not in im.only_locs(g, pid, card):
        return False
    for so, ia in g.providers("loc_veto"):
        if ia.loc_veto(g, so, pid, card, loc):
            return False
    for ef in g.effects:
        if ef.get("kind") == "loc_veto" and ef["fn"](g, ef, pid, card, loc):
            return False
    return True


def unit_locations(g, pid, card, src, closed_or_focus, with_extra=False):
    """Rule 355.2: base or a battlefield you control; Ambush adds battlefields where you have units; the card's own
    extra locations (Impl.extra_locs(g, pid, card, closed) -> [bf]: "You may play me to an occupied enemy
    battlefield", "to an open battlefield"...) and those other objects allow (Impl.aura_locs(g, src, pid, card,
    closed) -> [bf or (bf, extra choice)]: Arachnoid Horror, Miss Fortune, Buccaneer, Dragon Roost). Forbidden
    locations are removed (loc_allowed). with_extra=True returns [(loc, extra choice dict)]."""
    im = g.impl(card)
    if src == "facedown":
        locs = [(card.hidden_bf, {})]
    else:
        locs = []
        if not closed_or_focus:
            locs.append(("base", {}))
            locs += [(b.idx, {}) for b in g.bfs if b.ctrl == pid]
        if im.ambush or card_kw(g, pid, card, "Ambush", src):
            for b in g.bfs:
                if (b.idx, {}) not in locs and any(u.ctrl == pid for u in g.units(loc=b.idx)):
                    locs.append((b.idx, {}))
        if im.extra_locs is not None:
            for l in im.extra_locs(g, pid, card, closed_or_focus):
                if (l, {}) not in locs:
                    locs.append((l, {}))
        for so, ia in g.providers("aura_locs"):
            for l in ia.aura_locs(g, so, pid, card, closed_or_focus) or ():
                l, ex = (l if isinstance(l, tuple) else (l, {}))
                if (l, ex) not in locs:
                    locs.append((l, ex))
    locs = [(l, ex) for l, ex in locs if loc_allowed(g, pid, card, l)]
    return locs if with_extra else [l for l, ex in locs if not ex]


def open_bf(g, b):
    """An open battlefield: no units there and no controller (rule 355.2.b)."""
    return b.ctrl is None and not g.units(loc=b.idx)


def play_forbidden(g, pid, card, src):
    """"Can't play" restrictions (Impl.forbid_play(g, src_obj, pid, card, zone) of other objects: Fallen Feline,
    Noxus Saboteur...; g.effects kind='no_play' fn(g, eff, pid, card, zone): Lilting Lullaby, Brynhir)."""
    for so, ia in g.providers("forbid_play"):
        if ia.forbid_play(g, so, pid, card, src):
            return True
    for ef in g.effects:
        if ef.get("kind") == "no_play" and ef["fn"](g, ef, pid, card, src):
            return True
    return False


def card_choices(g, pid, card, src, timing_closed, timing_showdown, every=False):
    """All the play choices for a card (rule 355), already filtered for affordability.
    every=True: every legal choice (Impl.all_choices instead of the AI's capped list, cards.cap lifted), for a
    human player."""
    if not every:
        return _card_choices(g, pid, card, src, timing_closed, timing_showdown, full_choices(g, pid))
    prev, g.every_choice = getattr(g, "every_choice", False), True
    try:
        return _card_choices(g, pid, card, src, timing_closed, timing_showdown, True)
    finally:
        g.every_choice = prev


def _card_choices(g, pid, card, src, timing_closed, timing_showdown, every):
    im = g.impl(card)
    if im is None:
        return []
    if play_forbidden(g, pid, card, src):
        return []
    typ = card.spec["type"]
    restricted = timing_closed or timing_showdown
    out = []
    hidden_bf = card.hidden_bf if src == "facedown" else None
    qd = typ == "Gear" and card_kw(g, pid, card, "Quick-Draw", src)
    if typ == "Unit":
        timed = unit_timing_ok(im, timing_closed)
        amb = im.ambush or card_kw(g, pid, card, "Ambush", src)
        if restricted and src != "facedown" and not amb and not timed:
            return []
        locs = unit_locations(g, pid, card, src, restricted and src != "facedown" and not timed, with_extra=True)
        extra = im.as_played(g, pid, card) if im.as_played else [{}]
        acc = card_kw(g, pid, card, "Accelerate", src) and src != "facedown"
        for loc, lx in locs:
            for ex in extra:
                for a in ([False, True] if acc else [False]):
                    out.append(dict(ex, loc=loc, acc=a, **lx))
    elif typ == "Gear":
        if restricted and not (qd or src == "facedown" or unit_timing_ok(im, timing_closed)):
            return []
        extra = im.as_played(g, pid, card) if im.as_played else [{}]
        for ex in extra:
            out.append(dict(ex, loc=hidden_bf if hidden_bf is not None else "base"))
            if src == "trash" and flow_of(g, pid, card):
                out[-1]["flow"] = True
    elif typ == "Spell":
        if restricted and src != "facedown" and im.timing not in ("action", "reaction"):
            return []
        if timing_closed and src != "facedown" and im.timing != "reaction":
            return []
        ctx = dict(hidden_bf=hidden_bf, card=card, src=src)
        if every and im.all_choices is not None:
            base = im.all_choices(g, pid, ctx)
        else:
            base = im.choices(g, pid, ctx) if im.choices else [dict()]
            base = base if every else base[:MAX_CHOICES]
        if src == "trash" and flow_of(g, pid, card) is not None:
            base = [dict(ch, flow=True) for ch in base]
        out += spell_variants(g, pid, card, im, base, ctx, every)
    if g.effects:
        # play permissions created by effects (g.effects kind='play_perm', pid, key, fn(g, eff, card, src) -> bool,
        # choice=dict of choice flags, e.g. ignore_energy): Jayce, Man of Progress. Consumed by play_card.
        for ef in g.effects:
            if ef.get("kind") == "play_perm" and ef["pid"] == pid and ef["fn"](g, ef, card, src):
                out += [dict(ch, perm=ef["key"], **ef["choice"]) for ch in list(out) if ch.get("perm") is None]
    if im.alt_costs is not None and src != "facedown":
        # alternative costs (Impl.alt_costs(g, pid, card, src) -> [dict(key=, e=, reqs=)]): "you may play me from
        # your trash for ...", "you may play me for 1 mind rune"
        alts = im.alt_costs(g, pid, card, src) or []
        out = [ch for ch in out if src != "trash" or ch.get("flow") or g.effects_of("play_from_trash", pid)] + \
            [dict(ch, flow=False, alt=a["key"], alt_e=a["e"], alt_reqs=tuple(a["reqs"]))
             for a in alts for ch in out]
    elif src == "trash":
        out = [ch for ch in out if ch.get("flow") or g.effects_of("play_from_trash", pid)]
    res = []
    for ch in out:
        if affordable(g, pid, card, ch, src):
            res.append(ch)
    return res


def full_choices(g, pid=None):
    """True when every legal choice must be offered: a human player (agent with every_choice=True, train.Human) or
    card_choices / ability_options with every=True (Game.every_choice while they run). The AI keeps its capped,
    ordered lists (cards.cap)."""
    if getattr(g, "every_choice", False):
        return True
    ags = getattr(g, "agents", None)
    return pid is not None and bool(ags) and bool(getattr(ags[pid], "every_choice", False))


REPEAT_KEYS = ("rep", "reps", "rep2", "sq", "tg2")


def spell_variants(g, pid, card, im, base, ctx, every=False):
    """A spell's choices with its optional Repeat costs: the printed [Repeat] (choice rep=True, the repetition's
    targets in tg2) and each granted Repeat instance (rule 820.1.c.2: choice reps=(repetition choice,))."""
    out = []
    grep = repeat_instances(g, pid, card)
    every = every or full_choices(g, pid)
    second = None
    if im.repeat or grep:
        if every and im.all_choices is not None:
            second = im.all_choices(g, pid, ctx) or [{}]
        else:
            second = (im.choices(g, pid, ctx) if im.choices else None) or [{}]
        # A repetition uses the spell's base choices: a choice that already carries a repetition (printed Repeat
        # coded in the choices, e.g. Temptation rep2/tg2, Square Up sq/tg2) would repeat again without its cost
        # paid (rule 820.1.c.2).
        second = [c for c in second if not any(c.get(k) for k in REPEAT_KEYS)] or [{}]
    for ch in base:
        out.append(ch)
        if im.repeat:                              # Repeat can be paid with a Flow play too
            for ch2 in second[:(None if every else 3)]:
                out.append(dict(ch, rep=True, tg2=ch2.get("tg", ())))
        for k, re_, rr in grep:
            # a granted Repeat (rule 820): pay it to repeat the effect, with its own choices
            for ch2 in second[:(None if every else 2)]:
                rc = dict(ch2, _key=k, _e=re_, _reqs=tuple(rr))
                out.append(dict(ch, reps=(rc,)))
    return out


def playable_cards(g, pid, closed, showdown):
    pl = g.p[pid]
    cands = [(c, "hand") for c in pl.hand] + [(c, "champ") for c in pl.champ]
    for b in g.bfs:
        for c in b.facedowns:
            if c.owner == pid and c.hidden_turn is not None and c.hidden_turn < g.turn_no:
                cands.append((c, "facedown"))
    any_trash = bool(g.effects_of("play_from_trash", pid))      # Endless Riches: "You may play cards from your trash."
    for c in pl.trash:
        im = g.impl(c)
        if im is None:
            continue
        if any_trash or flow_of(g, pid, c) or im.alt_costs is not None:
            cands.append((c, "trash"))
    return cands


def play_options(g, pid, closed, showdown, every=False):
    opts = []
    seen = set()
    for c, src in playable_cards(g, pid, closed, showdown):
        key = (c.cname, src)
        if key in seen and src != "facedown":
            continue                                    # identical copies give identical options
        seen.add(key)
        for ch in card_choices(g, pid, c, src, closed, showdown, every=every):
            opts.append(("play", c.uid, src, ch))
    return opts


def abilities_of(g, obj):
    """Activated abilities of an object: its own (Impl.abilities), granted for a duration (obj.abs), granted by other
    objects (Impl.grant_abilities(g, src, obj)) and those of copied text (Svellsongur). Indices are stable for a
    given state (the 'act' action refers to an index)."""
    leg = getattr(obj, "zone", None) == "legend"
    im = g.impl(g.p[obj.ctrl].legend_name) if leg else g.impl(obj)
    out = list(im.abilities) if im is not None else []
    if not leg and getattr(obj, "attached_to", None) is not None:
        # 434.1.e : une carte attachée a son texte de règles inactif tant qu'elle reste attachée ; un équipement attaché ne
        # peut donc plus utiliser Equip (ni ses autres capacités imprimées) pour changer d'unité. Il reste attaché jusqu'à
        # ce qu'il quitte l'unité (mort, effet « detach »…). Les capacités données par un autre effect restent (ci-dessous).
        out = []
    out += [a for a, _ in obj.abs]
    if not leg and obj.attached:
        for n in g.copied_texts(obj):
            ic = g.impl(n)
            if ic is not None:
                out += list(ic.abilities)
    for so, ia in g.providers("grant_abilities"):
        out += list(ia.grant_abilities(g, so, obj) or ())
    return out


def ability_options(g, pid, closed, showdown, main, every=False):
    if not every:
        return _ability_options(g, pid, closed, showdown, main, full_choices(g, pid))
    prev, g.every_choice = getattr(g, "every_choice", False), True
    try:
        return _ability_options(g, pid, closed, showdown, main, True)
    finally:
        g.every_choice = prev


def _ability_options(g, pid, closed, showdown, main, every):
    opts = []
    srcs = [o for o in g.board if o.ctrl == pid] + ["legend"]
    for o in srcs:
        obj = g.p[pid].legend if o == "legend" else o
        for i, ab in enumerate(abilities_of(g, obj)):
            t = ab.get("timing", "main")
            if closed and t != "reaction":
                continue
            if showdown and not closed and t not in ("action", "reaction"):
                continue
            if not closed and not showdown and not main:
                continue
            if g.tp != pid and t == "main":
                continue
            if ab.get("can") and not ab["can"](g, pid, obj):
                continue
            if ab.get("exhaust") and obj.exhausted:
                continue
            if every and ab.get("all_choices"):
                chs = ab["all_choices"](g, pid, obj)
            else:
                chs = ab["choices"](g, pid, obj) if ab.get("choices") else [dict()]
                chs = chs if every else chs[:MAX_CHOICES]
            for ch in chs:
                e, reqs = ability_cost(g, pid, obj, ab, ch)
                if g.can_pay(pid, e, reqs, dict(kind="ability", obj=obj)):
                    opts.append(("act", ("legend", pid) if o == "legend" else o.uid, i, ch))
    return opts


def ability_cost(g, pid, obj, ab, ch):
    """Cost of an activated ability: its own cost plus Deflect for enemy objects it chooses (rule 809), then the
    cost modifiers of other objects and effects (cost_mods with kind='ability')."""
    e, reqs = ab["cost"](g, pid, obj, ch)
    if not ab.get("ignore_deflect"):
        reqs = list(reqs) + deflect_reqs(g, pid, ch)
    if g.effects or cards_hooks().get("cost_aura"):
        mods = cost_mods(g, pid, dict(kind="ability", card=None, obj=obj, ab=ab, choice=ch, src=None))
        if mods:
            e, reqs = _apply_mods(g, pid, e, reqs, [], mods, dict(kind="ability", obj=obj))
    return e, list(reqs)


def move_options(g, pid):
    """Standard Move (rule 144/420.3): exhaust ready units to move them to one destination.
    base -> battlefield, battlefield -> base, battlefield -> battlefield only with Ganking. Movement restrictions
    (Game.can_move) and move costs of several units (Game.move_tax) apply."""
    ready = [u for u in g.units(pid) if not u.exhausted]
    opts = []
    dests = ["base", 0, 1]
    for d in dests:
        movers = [u for u in ready if u.loc != d and (d == "base" or u.loc == "base" or g.has_kw(u, "Ganking"))
                  and g.can_move(u, d, pid, True)]
        if d == "base":
            movers = [u for u in movers if u.loc in (0, 1)]
        if not movers:
            continue
        groups = set()
        for u in movers:
            groups.add((u.uid,))
        by_loc = {}
        for u in movers:
            by_loc.setdefault(u.loc, []).append(u.uid)
        for l, us in by_loc.items():
            if len(us) > 1:
                groups.add(tuple(sorted(us)))
                if len(us) > 2:
                    # all but the weakest
                    us2 = sorted(us, key=lambda x: g.might(g.obj(x)))
                    groups.add(tuple(sorted(us2[1:])))
        if len(by_loc) > 1:
            groups.add(tuple(sorted(u.uid for u in movers)))
        for grp in sorted(groups):   # ordre fixe : un set de tuples n'itère pas pareil en 32 et 64 bits (Pyodide)
            if len(grp) > 1 and cards_hooks().get("move_tax"):
                reqs = g.move_tax(pid, [g.obj(u) for u in grp], d)
                if reqs and not g.can_pay(pid, 0, reqs, dict(kind="move")):
                    continue
            opts.append(("move", grp, d))
    return opts


def hide_costs(g, pid):
    """Costs to hide a card (rule 811.1.b: [A]), alternatives (Impl.hide_cost(g, src, pid) -> [(e, reqs)]: Teemo,
    Swift Scout) and "hide cards ignoring costs this turn" (g.effects kind='hide_free': Guerilla Warfare)."""
    out = [(0, (ANY,))]
    if g.effects_of("hide_free", pid):
        return [(0, ())]
    for so, ia in g.providers("hide_cost"):
        for e, reqs in ia.hide_cost(g, so, pid) or ():
            c = (e, tuple(reqs))
            if c not in out:
                out.append(c)
    return out


def hide_slots(g, b):
    """Number of facedown cards a battlefield can hold (Bandle Tree: "You may hide an additional card here")."""
    im = g.impl(b.name)
    return 1 + (im.hide_slots if im is not None and im.hide_slots else 0)


def hide_options(g, pid):
    pl = g.p[pid]
    opts = []
    costs = [c for c in hide_costs(g, pid) if g.can_pay(pid, c[0], list(c[1]), dict(kind="hide"))]
    if not costs:
        return opts
    seen = set()
    for c in pl.hand + pl.champ:
        im = g.impl(c)
        if im is None or not (im.hidden or card_kw(g, pid, c, "Hidden", "hand")) or c.cname in seen:
            continue
        seen.add(c.cname)
        for b in g.bfs:
            if b.ctrl == pid and len(b.facedowns) < hide_slots(g, b):
                for i, cost in enumerate(costs):
                    opts.append(("hide", c.uid, b.idx) if cost == (0, (ANY,)) else ("hide", c.uid, b.idx, cost))
    return opts


def main_options(g, pid, every=False):
    opts = [("end",)]
    opts += play_options(g, pid, closed=False, showdown=False, every=every)
    opts += ability_options(g, pid, closed=False, showdown=False, main=True, every=every)
    opts += hide_options(g, pid)
    opts += move_options(g, pid)
    return opts


def timed_options(g, pid, closed, every=False):
    showdown = g.sd is not None
    opts = play_options(g, pid, closed=closed, showdown=showdown or closed, every=every)
    opts += ability_options(g, pid, closed=closed, showdown=showdown, main=False, every=every)
    return opts


# ---------------------------------------------------------------- execution
def do_action(g, a):
    kind = a[0]
    pid = g.chain[-1].ctrl if False else None
    if kind == "play":
        _, uid, src, ch = a
        pid = owner_of(g, uid, src)
        card = card_obj(g, pid, uid, src)
        play_card(g, pid, card, src, dict(ch))
    elif kind == "hide":
        uid, bf = a[1], a[2]
        pid = owner_of(g, uid, "hand")
        hide(g, pid, uid, bf, a[3] if len(a) > 3 else (0, (ANY,)))
    elif kind == "act":
        _, src, i, ch = a
        activate(g, src, i, dict(ch))
    elif kind == "move":
        _, uids, dest = a
        units = [g.obj(u) for u in uids]
        pid = units[0].ctrl
        if len(units) > 1 and cards_hooks().get("move_tax"):
            reqs = g.move_tax(pid, units, dest)      # Mageseeker Investigator
            if reqs and not g.pay(pid, 0, reqs, dict(kind="move")):
                raise RuntimeError("cannot pay the move cost")
        for u in units:
            u.exhausted = True                       # cost of the Standard Move
        g.log(f"P{pid} moves {units} -> {dest}")
        g.move(units, dest, pid, standard=True)
    else:
        raise ValueError(a)


def owner_of(g, uid, src):
    for pl in g.p:
        for z in (pl.hand, pl.champ, pl.trash):
            for c in z:
                if c.uid == uid:
                    return pl.pid
    for b in g.bfs:
        for c in b.facedowns:
            if c.uid == uid:
                return c.owner
    raise ValueError(f"card {uid} not found")


def hide(g, pid, uid, bf, cost=(0, (ANY,))):
    """Hide (rule 421, 811): pay [A] (or an alternative cost), place facedown at a battlefield you control."""
    c = card_obj(g, pid, uid, "hand") or card_obj(g, pid, uid, "champ")
    assert g.pay(pid, cost[0], list(cost[1]), dict(kind="hide", card=c))
    g.remove_from_zone(c)
    c.reset()
    c.zone = "facedown"
    c.hidden_turn = g.turn_no
    c.hidden_bf = bf
    g.bfs[bf].facedowns.append(c)
    g.log(f"P{pid} hides a card at {g.bfs[bf].name}")
    g.stats[f"hide_P{pid}"] += 1
    g.need_cleanup = True
    g.emit("hide", pid=pid, card=c, bf=bf)


def play_card(g, pid, card, src, ch, limited=False):
    """The process of play (rule 353). limited=True: played by an effect (Baited Hook, Mixologist...)."""
    im = g.impl(card)
    sp = card.spec
    paid_e = 0
    if not limited:
        e, reqs = total_cost(g, pid, card, ch, src)
        what = dict(kind=sp["type"].lower(), card=card, obj=None, ab=None, choice=ch, src=src)
        used = [m["_eff"] for m in cost_mods(g, pid, what) if m.get("_eff") is not None and m["_eff"].get("once")] \
            if g.effects else []
        if not g.pay(pid, e, reqs, pay_ctx(card)):
            raise RuntimeError(f"cannot pay for {card} {ch}")
        nd = len(deflect_total(g, pid, card, ch))
        if nd:
            g.log(f"  P{pid} pays Deflect +{nd} power")
        paid_e = e
        # consume Heron discount and the one-shot cost effects that applied ("the next spell you play...")
        g.effects = [ef for ef in g.effects if not (ef.get("kind") == "heron" and ef["pid"] == pid)
                     and not any(ef is u for u in used)]
    elif ch.get("pay_power"):
        _, reqs = total_cost(g, pid, card, dict(ch, ignore_energy=True, free=False), "hand")
        if not g.pay(pid, 0, reqs):
            return None
    elif ch.get("pay_cost"):
        # limited play paying a given cost (e.g. "you may play me for 1 rune of any type"): (energy, reqs)
        e, reqs = ch["pay_cost"]
        if not g.pay(pid, e, list(reqs), pay_ctx(card)):
            return None
        paid_e = e
    if ch.get("perm") is not None:                 # a play permission used (card_choices)
        g.effects = [ef for ef in g.effects if not (ef.get("kind") == "play_perm" and ef.get("key") == ch["perm"])]
    if sp["type"] == "Spell":
        for ef in list(g.effects):
            if ef.get("kind") == "grant_repeat" and ef.get("next") and ef["pid"] == pid:
                g.effects.remove(ef)                   # "Give the next spell you play this turn [Repeat]..."
    # additional non-resource costs (paid as part of costs, rule 357.2)
    if ch.get("kill") is not None:
        k = g.obj(ch["kill"])
        if k is not None:
            ch["killed_info"] = dict(e=k.spec["e"], p=k.spec["p"], might=g.might(k), name=k.cname)
            g.kill([k], pid, cost=True)
    if ch.get("xp"):
        g.spend_xp(pid, ch["xp"] if not isinstance(ch["xp"], bool) else 3)
    if im.pay_extra is not None:
        im.pay_extra(g, pid, card, ch)                 # non-resource additional costs of the card (discard...)
    g.remove_from_zone(card)
    from_facedown = src == "facedown"
    hidden_bf = card.hidden_bf if from_facedown else None
    card.zone = "chain"
    if sp["type"] == "Spell":
        g.hist["spell_e"][pid].append(paid_e)
    elif sp["type"] == "Gear" and not card.token:
        g.hist["gear_played"][pid] += 1
    card.paid_e = paid_e
    g.finalized[pid].append(card.cname)
    g.log(f"P{pid} plays {card.cname} from {src} {fmt_choice(g, ch)}")
    g.stats[f"play_{card.cname}"] += 1
    if not g.chain and g.chain_origin is None:
        g.chain_origin = "play"
    if sp["type"] in ("Unit", "Gear"):
        loc = ch.get("loc", "base")
        if sp["type"] == "Gear" and not from_facedown:
            loc = "base"
        ready = (sp["type"] == "Gear" and not im.enter_exhausted) or bool(ch.get("acc")) or ch.get("ready", False)
        if not ready and (enters_ready(g, pid, card, ch) or others_enter_ready(g, pid, card)):
            ready = True
        g.enter_board(card, pid, loc, ready=ready)
        if ch.get("tag"):
            card.chosen_tag = ch["tag"]                # "As you play me, choose ... I gain that tag."
        card.played_from = src
        card.play_choice = ch
        g.played_event(pid, card)
        keyword_play_triggers(g, pid, card, hidden_bf)
        if im.on_play:
            im.on_play(g, card, dict(ch, hidden_bf=hidden_bf, src=src))
        if card_kw(g, pid, card, "Quick-Draw", src):
            # Quick-Draw: "When you play this, attach it to a unit you control."
            def qd_choose(g_, it):
                us = [u for u in g_.units(it.ctrl)]
                if hidden_bf is not None:
                    us = [u for u in us if u.loc == hidden_bf] or us
                if not us:
                    return False
                u = g_.ask(it.ctrl, "equip_target", us, gear=card)
                g_.add_target(it, u)
                return True

            def qd_resolve(g_, it):
                u = g_.legal(it, 0)
                gobj = g_.obj(it.src)
                if u is not None and gobj is not None:
                    attach(g_, gobj, u)
            g.queue_trigger(pid, f"Quick-Draw {card.cname}", qd_resolve, src=card.uid, choose=qd_choose)
        g.need_cleanup = True
        return card
    # spell
    g.spells_played[pid] += 1
    it = Item("spell", pid, card.cname, card=card, src=card.uid, data=dict(ch))
    it.data["from"] = src
    it.data["hidden_bf"] = hidden_bf
    it.data["played_by"] = pid
    it.data["paid_e"] = paid_e
    for ef in [e for e in g.effects if e.get("kind") == "next_spell" and e["pid"] == pid]:
        g.effects.remove(ef)                           # "The next spell you play this turn ..." (Ravenborn Tome)
        ef["fn"](g, ef, it)
    if ch.get("reps"):
        reps = []
        for rc in ch["reps"]:
            rc = dict(rc)
            rc["_oids"] = {u: g.obj(u).oid for u in rc.get("tg", ()) if isinstance(u, int) and g.obj(u) is not None}
            for u in sorted(rc["_oids"]):
                it.chosen.add(u)
            reps.append(rc)
        it.data["reps"] = reps
    if ch.get("flow") or im.banish_after:
        it.data["after"] = "banish"
    if im.uncounterable:
        it.uncounterable = True
    preds = im.preds or []
    tg = list(ch.get("tg", ()))
    for i, uid in enumerate(tg):
        if isinstance(uid, int):
            o = g.obj(uid)
            if o is not None:
                pred = preds[min(i, len(preds) - 1)] if preds else None
                g.add_target(it, o, pred)
            else:
                it.targets.append((uid, -1, None))
        else:
            it.targets.append((None, -1, None))
    for uid in ch.get("tg2", ()):
        if isinstance(uid, int) and g.obj(uid) is not None:
            o = g.obj(uid)
            it.data.setdefault("t2", []).append((o.uid, o.oid))
            it.chosen.add(o.uid)
    if ch.get("item") is not None:
        it.data["target_item"] = ch["item"]
    g.chain.append(it)
    g.priority = pid
    g.passes = 0
    g.on_finalize(it)
    g.need_cleanup = True
    return it


def others_enter_ready(g, pid, card):
    """"Your units enter ready" from other objects (Impl.units_enter_ready(g, src, pid, card): Magma Wurm, Master
    Yi, Wuju Master, Renata Glasc, Industrialist) and effects (g.effects kind='enter_ready' fn(g, eff, pid, card):
    Bushwhack...) (rule 369.3)."""
    for so, ia in g.providers("units_enter_ready"):
        if ia.units_enter_ready(g, so, pid, card):
            return True
    for ef in g.effects:
        if ef.get("kind") == "enter_ready" and ef["fn"](g, ef, pid, card):
            if ef.get("once"):
                g.effects.remove(ef)               # "The next unit you play this turn enters ready"
            return True
    return False


def enters_ready(g, pid, card, ch):
    """'I enter ready' (Impl.enter_ready: True or fn(g, pid, card, choice)) and [Level N] 'I enter ready'
    (Impl.levels ... ready=True). Checked on the card as it is played."""
    im = g.impl(card)
    er = im.enter_ready
    if er is True or (callable(er) and er(g, pid, card, ch)):
        return True
    if im.levels:
        xp = g.p[pid].xp
        if any(xp >= n and d.get("ready") for n, d in im.levels):
            return True
    return False


def keyword_play_triggers(g, pid, card, hidden_bf=None):
    """Play triggers of printed or granted keywords of a permanent that was just played (rule 803: keywords
    first): [Vision] (rule 817, one trigger per instance) and [Weaponmaster] (rule 821)."""
    for _ in range(g.kw_value(card, "Vision")):
        g.queue_trigger(pid, f"Vision {card.cname}", lambda g_, it: g_.predict(it.ctrl, 1), src=card.uid)
    if card.spec["type"] == "Unit":
        for _ in range(g.kw_value(card, "Weaponmaster")):
            weaponmaster_trigger(g, pid, card)


def equip_cost(g, pid, gear, unit, discount_any=0):
    """Cost of the Equip ability of gear for unit, as (energy, reqs, ability) or None if it has none.
    discount_any: number of [A] removed (Weaponmaster, rule 821.1.c.3: only [A] is reduced)."""
    im = g.impl(gear)
    if im is None:
        return None
    for ab in im.abilities:
        if ab.get("name") == "Equip":
            e, reqs = ab["cost"](g, pid, gear, dict(tg=(unit.uid,)))
            reqs = list(reqs)
            for _ in range(discount_any):
                if ANY in reqs:
                    reqs.remove(ANY)
            return e, reqs, ab
    return None


def weaponmaster_trigger(g, pid, unit):
    """[Weaponmaster] "When you play me, you may choose an Equipment you control and pay its Equip cost, reduced
    by [A], to attach it to me" (rule 821). The choice is made at finalization, the cost paid on resolution."""
    def options(g_, it):
        u = g_.obj(it.src)
        if u is None:
            return []
        out = []
        for x in g_.gear(it.ctrl):
            if "Equipment" not in x.spec["tags"] or x.attached_to == u.uid:
                continue
            c = equip_cost(g_, it.ctrl, x, u, 1)
            if c is not None and g_.can_pay(it.ctrl, c[0], c[1], dict(kind="ability", obj=x)) and \
                    (c[2].get("can_extra") is None or c[2]["can_extra"](g_, it.ctrl, x)):
                out.append(x)
        return sorted(out, key=lambda x: -(EQUIP_BONUS.get(x.cname, 0) * 10 + x.spec["e"]))

    def choose(g_, it):
        opts = options(g_, it)
        if not opts:
            return False
        x = g_.ask(it.ctrl, "weaponmaster_pick", opts, unit=g_.obj(it.src))
        if x is None:
            return False
        g_.add_target(it, x, lambda g2, it2, o: o.ctrl == it2.ctrl)
        return True

    def resolve(g_, it):
        u = g_.obj(it.src)
        x = g_.legal(it, 0)
        if u is None or x is None or u.oid != it.data["oid"]:
            return
        c = equip_cost(g_, it.ctrl, x, u, 1)
        if c is None or not g_.pay(it.ctrl, c[0], c[1], dict(kind="ability", obj=x)):
            return
        if c[2].get("extra_cost"):
            c[2]["extra_cost"](g_, it.ctrl, x, dict(tg=(u.uid,)))
        attach(g_, x, u)

    g.queue_trigger(pid, f"Weaponmaster {unit.cname}", resolve, dict(oid=unit.oid), src=unit.uid, may=True,
                    choose=choose)


def attach(g, gear, unit):
    """Attach (rule 434): detach from previous unit, location follows the unit (not a move)."""
    if gear.attached_to is not None:
        old = g.obj(gear.attached_to)
        if old is not None and gear.uid in old.attached:
            old.attached.remove(gear.uid)
    gear.attached_to = unit.uid
    unit.attached.append(gear.uid)
    gear.loc = unit.loc
    g.log(f"  {gear} attached to {unit}")
    g.emit("attached", gear=gear, unit=unit)


def activate(g, src, i, ch):
    """Activated abilities (rule 376). Add abilities and Equip resolve through the chain like others."""
    if isinstance(src, tuple):
        pid = src[1]
        obj = g.p[pid].legend
    else:
        obj = g.obj(src)
        pid = obj.ctrl
    ab = abilities_of(g, obj)[i]
    e, reqs = ability_cost(g, pid, obj, ab, ch)
    what = dict(kind="ability", card=None, obj=obj, ab=ab, choice=ch, src=None)
    used = [m["_eff"] for m in cost_mods(g, pid, what) if m.get("_eff") is not None and m["_eff"].get("once")] \
        if g.effects else []
    printed_e = ab["cost"](g, pid, obj, ch)[0]
    if not g.pay(pid, e, reqs, dict(kind="ability", obj=obj)):
        raise RuntimeError("cannot pay ability")
    g.effects = [ef for ef in g.effects if not any(ef is u for u in used)]
    if obj.spec["type"] == "Gear":
        g.hist["gear_abs"][pid] += 1
    if ab.get("exhaust"):
        obj.exhausted = True
    if ab.get("extra_cost"):
        ab["extra_cost"](g, pid, obj, ch)
    it = Item("ability", pid, f"{obj.cname}: {ab['name']}", fn=ab["resolve"], src=obj.uid, data=dict(ch))
    it.data["_ab"] = ab["name"]
    it.data["_cost_e"] = ab.get("printed_e", printed_e)       # printed Energy cost (rule 206.1)
    it.data["_src_obj"] = obj
    for j, uid in enumerate(ch.get("tg", ())):
        o = g.obj(uid)
        if o is not None:
            preds = ab.get("preds") or []
            g.add_target(it, o, preds[min(j, len(preds) - 1)] if preds else None)
    g.log(f"P{pid} activates {it.name} {fmt_choice(g, ch)}")
    g.stats[f"act_{obj.cname}:{ab['name']}"] += 1
    if not g.chain and g.chain_origin is None:
        g.chain_origin = "play"
    g.chain.append(it)
    g.priority = pid
    g.passes = 0
    g.on_finalize(it)
    g.need_cleanup = True


def fmt_choice(g, ch):
    parts = []
    for k, v in ch.items():
        if k in ("tg", "tg2"):
            parts.append(f"{k}=" + ",".join(str(g.obj(u) or u) for u in v))
        elif k in ("killed_info",):
            continue
        elif v is not False and v is not None and v != {} and v != ():
            parts.append(f"{k}={v}")
    return "(" + " ".join(parts) + ")" if parts else ""
