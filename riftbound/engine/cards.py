"""Card behaviours: one Impl per card of the Akali / LeBlanc pool (85 cards: the 68 main/sideboard cards of the
12 tournament lists, legends, runes, battlefields), plus tokens. Each entry follows the card text quoted above it.

Hooks
  timing        None | 'action' | 'reaction'      (spells and abilities)
  hidden, ambush, accelerate, quickdraw, flow=(e, p, domains), repeat=(e, domains, p)
  kw            static keywords {'Tank':1, 'Assault':1, ...}
  choices(g, pid, ctx) -> [choice]   spells: targets as uids in choice['tg'] (ordered best first for the AI)
  preds         target predicates checked again on resolution (rule 359.3.e)
  resolve(g, item)                   spells
  on_play(g, obj, ctx)               "When you play me/this"
  deathknell(g, item)                item.data['info'] holds the look-back information
  on_event(g, obj, ev, info)         triggered abilities of a permanent
  might_mod(g, obj) -> int           passive Might modifiers
  untargetable(g, obj, by_pid)
  abilities                          activated abilities
  cost_mod(g, pid, card, choice) -> (energy reduction, power reduction)
  as_played(g, pid, card) -> [extra choice dicts]   optional additional costs
  bf_event / legend_event / effect_event / no_score
Generic keyword fields (see cardsets/GUIDE.md): kw_if, might_if, levels, aura_kw, aura_might, equip_kw, equip, bonus,
  empower, add, enter_ready, enter_exhausted, track_mighty, ignore_deflect, extra_cost_fn, pay_extra.
Card modules live in cardsets/ (listed in cardsets/__init__.py) and are imported at the end of this file; cards whose
text is only keywords and reminder text are registered by auto_keywords() after them.
"""
import re
from game import ANY, SPEC, DOMAINS, EQUIP_BONUS, Item, Obj, Opt
from actions import full_choices  # noqa: E402
from actions import (play_card, attach, equip_cost, deflect_reqs, loc_allowed, open_bf, card_kw, flow_of,
                     abilities_of, others_enter_ready, keyword_play_triggers, total_cost, card_choices, pay_ctx)


class Impl:
    def __init__(s, name, **kw):
        s.name = name
        s.timing = None
        s.hidden = s.ambush = s.accelerate = s.quickdraw = False
        s.flow = s.repeat = None
        s.kw = {}
        s.choices = s.resolve = s.on_play = s.deathknell = s.on_event = s.might_mod = None
        s.untargetable = s.cost_mod = s.as_played = None
        s.bf_event = s.legend_event = s.effect_event = s.no_score = None
        s.preds = None
        s.dk_choose = None
        s.abilities = []
        s.uncounterable = False
        s.banish_after = False
        s.max_choices = 8
        # generic keyword support
        s.kw_if = []            # [(cond(g, o), {kw: value})]  dependent keywords ([Empowered], "While I'm Mighty"...)
        s.might_if = []         # [(cond(g, o), amount)]       dependent Might modifiers
        s.levels = []           # [(N, dict(might=, kw={}, ready=True))]  [Level N] (highest reached wins per field)
        s.aura_kw = None        # fn(g, src, o) -> {kw: value}  keywords this object gives to other objects
        s.aura_might = None     # fn(g, src, o) -> int          Might this object gives to other objects
        s.equip_kw = None       # {kw: value} given to the unit this Equipment is attached to (Effect Text)
        s.enter_ready = False   # True | fn(g, pid, card, choice): "I enter ready"
        s.enter_exhausted = False   # gear: "This enters exhausted"
        s.add = []              # [dict(e=, p=[domain|'A'], exhaust=True, kill=False, can=fn(g, pid, o, ctx))] Add
        s.track_mighty = False  # this card listens to 'becomes_mighty'
        s.ignore_deflect = False    # "Ignore [Deflect] while paying this spell's cost"
        s.extra_cost_fn = None  # fn(g, pid, card, choice) -> (energy, reqs): additional resource costs of a card
        s.pay_extra = None      # fn(g, pid, card, choice): pays non-resource additional costs (discard...)
        s.auto = False          # registered by auto_keywords()
        s.module = None         # name of the module that registered it
        # ---- hooks for effects of other objects (scanned through Game.providers; see cardsets/GUIDE.md)
        for f in HOOK_FIELDS:
            setattr(s, f, None)
        s.multi_buff = False    # "I can have any number of buffs"
        s.assign_last = False   # "I must be assigned combat damage last" (without [Backline])
        s.tie_recall_all = False    # Symbol of the Solari
        s.equip_tags = None     # tags given to the equipped unit (Effect Text: "I am a Mech.")
        s.might_watch = None    # N: emits 'might_reached' when my Might becomes N or more
        s.fx_deathknell = None  # Equipment: "[Deathknell] — ..." in its Effect Text, fn(g, item) like deathknell
        s.on_discard = None     # fn(g, card, pid): "When you discard me" (Game.discard)
        s.on_leave = None       # fn(g, obj, info): "When this leaves the board" (info: 'leave' event, look-back
        #                         info['info'], dest, killed=True for a kill)
        s.opt_parts = None      # fn(g, pid, card, choice) -> [(tag, e, reqs)]: optional additional costs paid
        s.repeat_all = None     # fn(g, pid, card, ctx) -> [choice]: choices of a repetition (default: choices)
        s.all_choices = None    # fn(g, pid, ctx) -> [choice]: every legal choice, for a human player (train.py)
        s.alt_costs = None      # fn(g, pid, card, src) -> [dict(key=, e=, reqs=)]: alternative costs (rule 356.1.b)
        equip = kw.pop("equip", None)
        bonus = kw.pop("bonus", None)
        empower = kw.pop("empower", None)
        for k, v in kw.items():
            if not hasattr(s, k):
                raise TypeError(f"{name}: unknown Impl field {k!r}")
            setattr(s, k, v)
        s.abilities = list(s.abilities)
        if equip is not None and not any(a.get("name") == "Equip" for a in s.abilities):
            s.abilities.append(equip_ability_cost(equip))
        if bonus is not None:
            EQUIP_BONUS[name] = bonus
        if empower is not None:
            s.abilities.insert(0, empower_ability(empower))


# Impl fields that are hooks read by the engine on other objects (each registers the card's name in HOOKS[field]):
HOOK_FIELDS = (
    "death_rep", "fx_death_rep",                 # "would die ... instead" (Game.kill)
    "dmg_bonus", "fx_dmg_bonus", "dmg_prevent", "aura_dmg_prevent", "lethal_any",   # damage (Game.deal)
    "no_combat_damage", "aura_no_combat", "ignore_tank",                            # combat damage
    "cost_aura",                                 # cost modifiers of other cards / abilities (actions.total_cost)
    "extra_locs", "aura_locs", "loc_veto", "only_locs", "forbid_play", "card_kw",   # playing cards
    "units_enter_ready", "cant_move", "fx_cant_move", "aura_cant_move", "move_tax",  # entering, moving
    "grant_abilities", "aura_untargetable", "aura_tags",
    "no_ready", "aura_no_ready", "channel_mod", "score_veto", "point_rep", "scoring_extra", "fx_swap_scoring",
    "choice_rep", "minus_extra", "no_counter", "gold_extra", "hide_cost", "hide_slots", "token_rep",
    "grant_add",
    "zone_event", "on_seen", "on_reveal", "reveal_rep",
)
IMPL = {}
# French question titles of the g.ask kinds, for a human player (train.py). Modules declare theirs with ask_text().
ASK_TEXT = {}


def ask_text(**kinds):
    """Declare the French title of g.ask kinds: ask_text(stacked_pick="Stacked Deck : quelle carte prendre ?")."""
    for k, v in kinds.items():
        if k in ASK_TEXT and ASK_TEXT[k] != v:
            raise ValueError(f"ask kind {k!r} already has the title {ASK_TEXT[k]!r}")
        ASK_TEXT[k] = v


# French titles of the shared and per-card g.ask kinds (train.py prefixes the asking card's name when it is known).
ask_text(
    choose="Choisis une cible :", add_kill="Quelle unité détruire pour produire la ressource ?",
    altar_card="Quelle carte de ta main placer ?", altar_where="Sur le dessus ou sous ton deck ?",
    aphelios_mode="Quel mode ?", apothecary_pick="Quelle unité soigner ?", ava_play="Quelle carte jouer ?",
    azir_attach="Quel équipement attacher ?", azir_move="Déplacer cette unité ?",
    banish_from_trash="Quelle carte de la défausse bannir ?", bard_move="Quelles unités déplacer, et où ?",
    buff_target="Quelle unité renforcer (buff) ?", buhru_mode="Piocher 1 ou te renforcer ?",
    bullet_time_x="Combien de runes payer (X) ?", burn_player="Quel joueur subit l'effet ?",
    call_to_battle="Quelle unité déplacer vers ce champ de bataille ?", choose_player="Quel joueur défausse ?",
    clone_banish="Quelle unité bannir ?", constellation_kill="Quelle unité détruire pour payer ?",
    corrupted_dragon="Quelles unités ennemies renvoyer à leur base ?", dais_return="Quelle unité renvoyer en main ?",
    dame_unit="Quelle unité de ce lieu ?", death_replace="Remplacer la mort de l'unité ?",
    decree_strength="Quelle carte révélée prendre ?", detach="Quel équipement détacher ?",
    discord_subset="Quelles unités ennemies renvoyer en main ?", double_pick="Quelle unité prendre parmi les cartes vues ?",
    draw_pick="Quelle carte piocher ?", duel_keep="Quelle unité garder ?", empower_target="Quelle unité de ce lieu ?",
    entourage="Quelle légende, et la préparer ou l'épuiser ?", excited_discard="Défausser quelle carte ?",
    faefolk_target="Quelle unité ennemie déplacer ici ?", fate_pick="Quel sort jouer ?", fizz_pick="Quel sort lancer ?",
    forge_recycle="Quelle carte recycler ?", fortified_shield="Quelle unité protéger ?",
    foxfire_subset="Quelles unités cibler ?", friend_target="Quelle unité choisir ?",
    friendly_target="Quelle unité cibler ?", gear_target="Quel équipement cibler ?",
    harrowing_pick="Quelle unité de ta défausse jouer ?", hatchling_recycle="Recycler la carte du dessus du deck ?",
    here_to_help="Quelle unité jouer ?", investigator_pick="Quelle carte de sa main choisir ?",
    ivern_pick="Quelle unité prendre parmi les cartes vues ?", jayce_mode="Quel mot-clé gagner ?",
    judgment_gear="Quels équipements garder ?", judgment_hand="Quelles cartes de ta main garder ?",
    judgment_rune="Quelles runes garder ?", judgment_unit="Quelles unités garder ?",
    kato_target="Quelle unité alliée cibler ?", kharox_pick="Quelle unité de sa défausse jouer ?", matriarch_pick="Quelle unité jouer à ta base ?",
    mf_ready="Que préparer : une unité ou une rune ?", minah_mode="Piocher ou défausser ?",
    mindsplitter_pick="Quelle carte de la main adverse défausser ?", morgana_target="Quelle unité cibler ?",
    move_enemy="Quelle unité ennemie déplacer, et où ?", name_tag="Quel nom de champion choisir ?",
    ornn_pick="Quel équipement prendre parmi les cartes vues ?", overt_spend="Dépenser le buff de cette unité ?",
    party_favors="Que choisis-tu : des cartes ou des runes ?", pay_or_countered="Payer 2 pour ne pas être contré ?",
    play_choice="Comment jouer cette carte ?", play_extra="Payer un coût supplémentaire ?",
    predict_recycle="Recycler les cartes regardées ?", predict_top="Quelle carte remettre sur le dessus ?",
    profiteer="Quelle unité déplacer, et vers laquelle ?", promising_pick="Quelle carte jouer ?",
    promising_play="Comment jouer cette carte ?", pursuit_attach="Quel équipement attacher ?",
    qiyana_mode="Piocher 1 ou canaliser 1 rune ?", ready_gear="Quel équipement préparer ?",
    ready_pick="Que préparer ?", ready_rune="Quelle rune préparer ?", ready_target="Quelle unité préparer ?",
    reaver_move="Quelle unité déplacer ?", recycle_from_trash="Quelle carte de la défausse recycler ?",
    recycle_pick="Quelle carte de la défausse recycler ?", recycle_trash="Quelle carte de la défausse recycler ?",
    reinforce_pick="Quelle unité jouer ?", reksai_choice="Où jouer cette carte ?",
    reksai_pick="Quelle carte révélée jouer ?", reksai_play="Quelle carte révélée jouer ?",
    rell_pick="Quel équipement de ta main jouer et attacher ?", rengar_target="Quelle unité cibler ?",
    resurrect_pick="Quelle unité de ta défausse ramener ?", return_gear="Quel équipement renvoyer en main ?",
    return_pick="Quelle carte renvoyer en main ?", rites_pick="Quelle unité de ta défausse jouer ?",
    rumble_pick="Quelles cartes choisir (Mech à recycler, Mech à reprendre) ?", rush_pick="Quelle carte prendre en main ?",
    sabotage="Quelle carte révélée recycler ?", sacrifice_gear="Quel équipement sacrifier ?",
    shakedown_let_draw="Laisser l'adversaire piocher ?", skewer_pick="Quelle unité jouer sur ce champ de bataille ?",
    soulgorger_pick="Quelle carte de ta défausse jouer ?", spend_buff="Quelle unité dépense son buff ?",
    split_damage="Comment répartir les dégâts ?", split_keep="Entre quelles unités répartir les dégâts ?",
    stacked_pick="Quelle carte prendre en main ?", starhound_pick="Quelle carte de ta défausse reprendre en main ?",
    token_location="Où créer le jeton ?", trash_gear="Quel équipement mettre à la défausse ?",
    trash_spell="Quel sort de ta défausse reprendre ?", trash_unit="Quelle unité de ta défausse reprendre ?",
    tricksy_subset="Quelles unités choisir ?", veiled_detach="Détacher cet équipement ?",
    verdict_place="Sur le dessus ou sous le deck ?", void_rush_pick="Quelle carte jouer ?",
    weaponmaster_pick="Quel équipement attacher ?", whirlwind_pick="Quelle unité renvoyer dans la main de son propriétaire ?",
    wild_claw="Quelle unité jouer ?")


HOOKS = {}            # hook field -> names of the cards defining it (Game.providers)
CONVERTERS = []       # non-empty when a card has an [Add] ability with a resource/kill cost (Game.plan_convert)
AURA = set()          # names of cards with aura_kw / aura_might (scanned by Game.kw_value / Game.might)
ADDERS = set()        # names of cards with [Add] abilities usable while paying (Game.plan_payment)
TRACK_MIGHTY = []     # non-empty when a modelled card listens to 'becomes_mighty'
_LOADING = [None]     # module currently registering cards


def card(name, **kw):
    """Register the behaviour of a card (one entry per card name). Registering a name twice is an error."""
    if name in IMPL:
        prev = IMPL[name].module or "cards.py"
        raise ValueError(f"card {name!r} is already registered (by {prev}); a card has one Impl")
    if name not in SPEC:
        raise KeyError(f"card {name!r} is not in the card database (game.SPEC)")
    im = Impl(name, **kw)
    im.module = _LOADING[0]
    IMPL[name] = im
    if im.aura_kw is not None or im.aura_might is not None:
        AURA.add(name)
    if im.add:
        ADDERS.add(name)
    if im.track_mighty and not TRACK_MIGHTY:
        TRACK_MIGHTY.append(True)
    for f in HOOK_FIELDS + ("tie_recall_all", "might_watch"):
        if getattr(im, f):
            HOOKS.setdefault(f, set()).add(name)
    if any(ad.get("conv") for ad in im.add) and not CONVERTERS:
        CONVERTERS.append(True)
    return im


# ====================================================================== helpers
def value(g, o):
    """Rough value of a unit, used only to order choices for the AI."""
    if o.spec["type"] != "Unit":
        return 2 + o.spec["e"] / 2
    v = g.might(o) + (o.spec["e"] + 2 * o.spec["p"]) / 3
    if o.cname in ("Karthus, Eternal",):
        v += 4
    if o.token and o.cname == "Reflection":
        v -= 1
    return v


def enemies(g, pid, at_bf=False, hidden_bf=None, targetable=True):
    us = [u for u in g.units(1 - pid) if (not at_bf or u.loc in (0, 1))]
    if hidden_bf is not None:
        us = [u for u in us if u.loc == hidden_bf]
    if targetable:
        us = [u for u in us if g.targetable(u, pid)]
    return sorted(us, key=lambda u: -value(g, u))


def friends(g, pid, at_bf=False, hidden_bf=None):
    us = [u for u in g.units(pid) if (not at_bf or u.loc in (0, 1))]
    if hidden_bf is not None:
        us = [u for u in us if u.loc == hidden_bf]
    return sorted(us, key=lambda u: -value(g, u))


def all_units(g, pid, at_bf=False, hidden_bf=None):
    us = [u for u in g.units() if (not at_bf or u.loc in (0, 1))]
    if hidden_bf is not None:
        us = [u for u in us if u.loc == hidden_bf]
    us = [u for u in us if g.targetable(u, pid)]
    return sorted(us, key=lambda u: (u.ctrl == pid, -value(g, u)))


def hb_ok(it, o):
    hb = it.data.get("hidden_bf")
    return hb is None or o.loc == hb


def P_unit(g, it, o):
    return o.spec["type"] == "Unit" and hb_ok(it, o)


def P_unit_bf(g, it, o):
    return o.spec["type"] == "Unit" and o.loc in (0, 1) and hb_ok(it, o)


def P_enemy(g, it, o):
    return o.spec["type"] == "Unit" and o.ctrl != it.ctrl and hb_ok(it, o)


def P_enemy_bf(g, it, o):
    return P_enemy(g, it, o) and o.loc in (0, 1)


def P_friend(g, it, o):
    return o.spec["type"] == "Unit" and o.ctrl == it.ctrl and hb_ok(it, o)


def P_friend_bf(g, it, o):
    return P_friend(g, it, o) and o.loc in (0, 1)


def P_gear(g, it, o):
    return o.spec["type"] == "Gear"


def tg_choices(units, n=1):
    return [dict(tg=(u.uid,)) for u in units]


def cap(g, seq, n, pid=None):
    """The first n items for the AI (a cap on choice combinations), all of them for a human (full_choices)."""
    seq = list(seq)
    return seq if full_choices(g, pid) else seq[:n]


def item_by_id(g, iid):
    for it in g.chain:
        if it.id == iid:
            return it
    return None


def trig_target(options_fn, pred=None, kind="target", optional=False, deflect=True):
    """Build a finalization-time chooser for a triggered ability with one target.
    Enemy targets with [Deflect] must be paid for (rule 809.1.c : « Spells and abilities an opponent controls that
    target [me] cost ... more ») ; unpayable ones are not offered. Friendly targets never pay (deflect_reqs).
    deflect=False was the default before 2026-10-09: Akali, Deadly Weapon targeted Master Yi, Tempered (Deflect at
    Level 6) for free (retour utilisateur)."""
    def opts_of(g, it):
        opts = options_fn(g, it)
        if pred is not None and full_choices(g, it.ctrl):      # a human may choose any legal target
            opts = list(opts) + sorted([o for o in g.board if o not in opts and g.targetable(o, it.ctrl)
                                        and pred(g, it, o)], key=lambda o: (o.ctrl == it.ctrl, o.uid))
        if deflect:
            opts = [o for o in opts if g.can_pay(it.ctrl, 0, deflect_reqs(g, it.ctrl, dict(tg=(o.uid,))))]
        return opts

    def choose(g, it):
        opts = opts_of(g, it)
        if not opts:
            return False
        o = g.ask(it.ctrl, kind, opts, item=it)
        if o is None:
            return False
        if deflect and not pay_deflect(g, it, o):
            return False
        g.add_target(it, o, pred)
        return True
    choose.options = opts_of          # flush_triggers : pas de question « may » quand il n'y a aucune cible
    return choose


def may_pay(e, reqs_fn=None):
    def can(g, it):
        return g.can_pay(it.ctrl, e, reqs_fn(g, it) if reqs_fn else [])

    def cost(g, it):
        reqs = reqs_fn(g, it) if reqs_fn else []
        if not g.can_pay(it.ctrl, e, reqs):
            return False
        return g.pay(it.ctrl, e, reqs)
    cost.can = can            # flush_triggers : pas de question « utiliser l'effet ? » quand on ne peut pas payer
    cost.label = f"{e} énergie" + (" et des runes" if reqs_fn else "")
    return cost


def extra_label(g, ex):
    """French label of an 'as you play me' / additional cost choice dict (Impl.as_played), for a human player."""
    if not ex:
        return "sans coût additionnel"
    parts = []
    for k in sorted(ex):
        v = ex[k]
        o = g.obj(v) if isinstance(v, int) and not isinstance(v, bool) else None
        if k == "acc":
            parts.append("payer [Accelerate]" if v else "sans [Accelerate]")
        elif k == "kill" and o is not None:
            parts.append(f"tuer {o.cname}")
        elif k == "kills":
            parts.append("tuer " + ", ".join(g.obj(u).cname for u in v if g.obj(u) is not None) if v else "ne rien tuer")
        elif k == "ret_gear" and o is not None:
            parts.append(f"renvoyer {o.cname} en main")
        elif k == "tag":
            parts.append(f"tag {v}")
        elif k == "xp":
            parts.append("dépenser de l'XP")
        elif o is not None:
            parts.append(f"{k} : {o.cname}")
        elif v is True:
            parts.append("payer le coût additionnel")
        else:
            parts.append(f"{k} = {v}")
    return ", ".join(parts)


def play_unit_free(g, pid, c, src, loc_choices=None, ignore_energy=False, alt_cost=None):
    """Play a unit as a limited action, ignoring its cost (or only its Energy cost). Play restrictions apply: "can't
    play" effects (actions.play_forbidden), a card that can't be played now (Impl.as_played returning no choice:
    Ol' Poro, a mandatory additional cost that can't be paid) and locations where it can't be played
    (actions.loc_allowed). Additional costs (Impl.as_played choices) are chosen and paid (their resource part too).
    alt_cost=(e, reqs): played for that cost instead of its own ("you may pay [Fury] to play me", rule 356.1.a),
    added to the additional costs paid. Returns the played card or None."""
    from actions import loc_allowed, play_forbidden
    if play_forbidden(g, pid, c, src):
        return None
    im = g.impl(c)
    extras = im.as_played(g, pid, c) if im is not None and im.as_played else [{}]
    if card_kw(g, pid, c, "Accelerate", src):          # optional additional costs can still be paid (356.1.b.3)
        extras = [dict(ex, acc=True) for ex in extras] + list(extras)
    payable = []
    for ex in extras:
        e, reqs = total_cost(g, pid, c, dict(ex, free=not ignore_energy, ignore_energy=ignore_energy), src)
        if alt_cost is not None:
            e, reqs = e + alt_cost[0], list(reqs) + list(alt_cost[1])
        if g.can_pay(pid, e, reqs, pay_ctx(c)):
            payable.append((ex, e, reqs))
    if not payable:
        return None
    locs = loc_choices if loc_choices is not None else (["base"] + [b.idx for b in g.bfs if b.ctrl == pid])
    locs = [l for l in locs if loc_allowed(g, pid, c, l)]
    if not locs:
        return None
    if len(payable) > 1:
        pick = g.ask(pid, "play_extra", [Opt(extra_label(g, ex), (ex, e, reqs)) for ex, e, reqs in payable], card=c)
        ex, e, reqs = pick.value
    else:
        ex, e, reqs = payable[0]
    loc = g.ask(pid, "play_location", locs, card=c)
    ch = dict(ex, loc=loc, free=not ignore_energy, ignore_energy=ignore_energy)
    if e or reqs:
        ch["pay_cost"] = (e, tuple(reqs))           # Power cost (ignore_energy) and the additional costs
    return play_card(g, pid, c, src, ch, limited=True)


def unit_play_choices(g, pid, c, src, locs=None):
    """Play choices of a unit played by an effect (rule 419.3: limited play): its locations (locs=None: the usual
    ones with the card's own and granted permissions, actions.unit_locations; else exactly locs) without the
    forbidden ones, x its 'as you play me' / additional cost choices (Impl.as_played; none: it can't be played),
    x [Accelerate] when it has it (also granted: Rek'Sai, Breacher). Costs are not checked here."""
    from actions import loc_allowed, play_forbidden, unit_locations
    im = g.impl(c)
    if im is None or c.spec["type"] != "Unit" or play_forbidden(g, pid, c, src):
        return []
    if locs is None:
        pairs = unit_locations(g, pid, c, src, False, with_extra=True)
    else:
        pairs = [(l, {}) for l in locs if loc_allowed(g, pid, c, l)]
    extras = im.as_played(g, pid, c) if im.as_played else [{}]
    acc = card_kw(g, pid, c, "Accelerate", src)
    return [dict(ex, loc=l, acc=a, **lx) for l, lx in pairs for ex in extras for a in ([False, True] if acc else [False])]


def remake_choices(g, it, pid):
    """"You may make new choices for it" (rules 750-755) for a spell on the chain, by pid (its new controller):
    targets, modes, locations and destinations of the spell and of each repetition (choices made "as you play" and
    optional additional costs are kept, rule 752.2; costs of new choices are ignored, rule 755). Newly chosen
    objects get their targeting effects (rule 754)."""
    if it.kind != "spell":
        return
    im = g.impl(it.card)
    if im is None or im.choices is None:
        return
    ctx = dict(hidden_bf=it.data.get("hidden_bf"), card=it.card, src=it.data.get("from"))
    cands = im.all_choices(g, pid, ctx) if im.all_choices is not None else im.choices(g, pid, ctx)
    cost_keys = ("acc", "rep", "reps", "flow", "alt", "alt_e", "alt_reqs", "kill", "xp", "free", "ignore_energy")
    cands = [{k: v for k, v in ch.items() if k not in cost_keys} for ch in cands]
    if not cands:
        return
    before = set(it.chosen)
    keep = Opt("garder les choix actuels", None)
    pick = g.ask(pid, "new_choices", [Opt(_choice_label(g, ch), ch) for ch in cands] + [keep], item=it)
    preds = im.preds or []
    if pick is not None and pick.value is not None:
        ch = pick.value
        it.data.update(ch)
        it.targets = []
        it.chosen = set()
        for i, uid in enumerate(ch.get("tg", ())):
            o = g.obj(uid) if isinstance(uid, int) else None
            if o is not None:
                g.add_target(it, o, preds[min(i, len(preds) - 1)] if preds else None)
            else:
                it.targets.append((uid if isinstance(uid, int) else None, -1, None))
        for uid, oid in it.data.get("t2", ()):
            it.chosen.add(uid)
    if it.data.get("t2") is not None:                  # printed Repeat: the repetition's targets
        pick = g.ask(pid, "new_choices", [Opt(_choice_label(g, ch), ch) for ch in cands if ch.get("tg")] + [keep],
                     item=it, repeat=True)
        if pick is not None and pick.value is not None:
            it.data["t2"] = [(u, g.obj(u).oid) for u in pick.value.get("tg", ()) if g.obj(u) is not None]
    for rc in it.data.get("reps") or ():               # granted Repeat instances
        pick = g.ask(pid, "new_choices", [Opt(_choice_label(g, ch), ch) for ch in cands] + [keep], item=it,
                     repeat=True)
        if pick is not None and pick.value is not None:
            for k in [k for k in rc if not k.startswith("_")]:
                del rc[k]
            rc.update(pick.value)
            rc["_oids"] = {u: g.obj(u).oid for u in rc.get("tg", ()) if isinstance(u, int) and g.obj(u) is not None}
    for t in it.data.get("t2", ()) or ():
        it.chosen.add(t[0])
    for rc in it.data.get("reps") or ():
        it.chosen.update(rc.get("_oids", {}))
    for uid in sorted(it.chosen - before):           # rule 754: targeting effects of newly chosen objects
        o = g.obj(uid)
        if o is not None:
            if o.ctrl != it.ctrl and o.spec["type"] == "Unit":
                g.hist["chose_enemy"][it.ctrl] = True
            g.emit("chosen", obj=o, item=it)


_CHOICE_WORDS = dict(acc="avec Accelerate", free="gratuitement", ignore_energy="sans payer l'énergie",
                     pay_power="en payant la puissance", rep="répété", flow="avec Flow")


_CHOICE_UIDS = dict(kill="en détruisant", spend="en dépensant le buff de", disc="en défaussant",
                    exh="en épuisant", rec="en recyclant", cards="cartes :", cs="cartes :")


def _uid_name(g, x):
    o = g.obj(x) if isinstance(x, int) else None
    if o is None and isinstance(x, int):
        o = next((c for p in g.p for z in (p.hand, p.trash, p.deck, p.banish) for c in z if c.uid == x), None)
    return g.label(o) if o is not None else str(x)


def _tg_name(g, u):
    """Cible d'un choix : « Shipyard Skulker #2 » quand elle a des homonymes en jeu (Game.label)."""
    o = g.obj(u) if isinstance(u, int) else None
    return g.label(o) if o is not None else str(u)


def _choice_label(g, ch):
    parts = []
    for k in sorted(ch):
        v = ch[k]
        if k in ("tg", "tg2"):
            names = ", ".join(_tg_name(g, u) for u in v)
            parts.append(("cible " if k == "tg" else "cible répétée ") + names if names else "aucune cible")
        elif k == "item":
            x = item_by_id(g, v)
            parts.append(f"sort {x.name}" if x is not None else "un sort")
        elif k in ("loc", "dest"):
            if v == "base" or v in (0, 1):
                parts.append("à la base" if v == "base" else g.bfs[v].name)
        elif k == "mover" and g.obj(v) is not None:
            parts.append(f"déplace {g.label(g.obj(v))}")
        elif k in _CHOICE_WORDS:
            if v:
                parts.append(_CHOICE_WORDS[k])
        elif k in ("mover_oid", "oid"):
            continue
        elif k in _CHOICE_UIDS and v is not None:
            vs = v if isinstance(v, (tuple, list)) else (v,)
            names = [_uid_name(g, x) for x in vs]
            if names:
                parts.append(_CHOICE_UIDS[k] + " " + ", ".join(names))
        elif v is not None and v is not False:
            parts.append(f"{k} = {v}")
    return " ; ".join(parts) or "sans choix"


def gain_control_item(g, it, pid):
    """"Gain control of a spell" (rule 359.3.f): the item's controller changes (the player who played the card keeps
    having played it: it.data['played_by'])."""
    it.data.setdefault("played_by", it.ctrl)
    it.ctrl = pid
    g.log(f"  P{pid} gains control of {it.name}")


# ====================================================================== runes, legends
card("Fury Rune"); card("Calm Rune"); card("Mind Rune"); card("Order Rune")


# Akali, Rogue Assassin — "[Empower] [3][A]. [Action][>] [E]: If it's your turn, move a friendly unit in a
# showdown to base and if I'm [Empowered], ready it."
def _akali_retreat(g, it):
    u = g.legal(it, 0)
    if u is None or g.tp != it.ctrl:
        return
    g.move([u], "base", it.ctrl)
    me = g.obj(it.src)                     # "if I'm [Empowered]": the ability's source (Heimerdinger may have it)
    leg = me if me is not None else g.p[it.ctrl].legend
    if leg.empowered:
        g.ready_obj(u)


def _akali_retreat_choices(g, pid, leg):
    if g.sd is None or g.tp != pid:
        return []
    return [dict(tg=(u.uid,)) for u in friends(g, pid) if u.loc == g.sd.bf]


card("Akali, Rogue Assassin", abilities=[
    dict(name="Empower", timing="main", can=lambda g, pid, o: not o.empowered,
         cost=lambda g, pid, o, ch: (3, [ANY]), resolve=lambda g, it: g.empower(g.p[it.ctrl].legend)),
    dict(name="Retreat", timing="action", exhaust=True, can=lambda g, pid, o: g.sd is not None and g.tp == pid,
         choices=_akali_retreat_choices, cost=lambda g, pid, o, ch: (0, []),
         preds=[lambda g, it, o: o.ctrl == it.ctrl and g.sd is not None and o.loc == g.sd.bf],
         resolve=_akali_retreat),
])


# LeBlanc, Deceiver — "When you conquer or hold, you may discard 1 and exhaust me to play a ready Reflection unit
# token there. It becomes a copy of another unit there. Give it [Temporary]."
def _reflection(g, pid, loc, model):
    t = make_token(g, "Reflection", pid, loc, ready=True)
    if t is None:
        return None
    if model is not None:
        t.copy = model.cname                     # copyable traits incl. stats (Reflection ruling)
    g.grant(t, "Temporary", 1, None)
    g.log(f"  Reflection of {model.cname if model else 'nothing'} at {loc}")
    g.stats[f"reflection_P{pid}"] += 1
    return t


def _leblanc_legend(g, pid, ev, info):
    if ev not in ("conquer", "hold") or info["pid"] != pid:
        return
    bf = info["bf"]

    def cost(g_, it):
        pl = g_.p[it.ctrl]
        if pl.legend.exhausted or not pl.hand:
            return False
        c = g_.ask(it.ctrl, "discard", list(pl.hand), reason="leblanc")
        g_.to_zone(c, "trash")
        pl.legend.exhausted = True
        return True

    def choose(g_, it):
        us = [u for u in g_.units(loc=bf)]
        if not us:
            return False
        o = g_.ask(it.ctrl, "copy_target", sorted(us, key=lambda u: -value(g_, u)), item=it)
        g_.add_target(it, o, lambda g2, it2, o2: o2.loc == bf)
        return True

    def res(g_, it):
        model = g_.legal(it, 0)
        _reflection(g_, it.ctrl, bf, model)

    g.queue_trigger(pid, "LeBlanc, Deceiver", res, dict(bf=bf), may=True, choose=choose, cost=cost)


card("LeBlanc, Deceiver", legend_event=_leblanc_legend)


# ====================================================================== battlefields
def _bar(g, b, ev, info):
    # "When a unit moves from here, give it +1 might this turn."
    if ev == "move" and info["frm"] == b.idx:
        o = info["obj"]
        g.queue_trigger(o.ctrl, "Back-Alley Bar", lambda g_, it: (g_.obj(it.data["u"]) and
                        g_.mod(g_.obj(it.data["u"]), 1)), dict(u=o.uid))


card("Back-Alley Bar", bf_event=_bar)


def _dusk(g, b, ev, info):
    # "At the start of your Beginning Phase, you may kill a unit you control here to draw 1."
    if ev == "beginning_start":
        pid = info["pid"]
        if not g.units(pid, b.idx):
            return

        def res(g_, it):
            us = g_.units(it.ctrl, b.idx)
            if not us:
                return
            u = g_.ask(it.ctrl, "dusk_kill", [None] + us)
            if u is None:
                return
            g_.kill([u], it.ctrl, cost=True)
            g_.draw(it.ctrl, 1)
        g.queue_trigger(pid, "Dusk Rose Lab", res, may=True)


card("Dusk Rose Lab", bf_event=_dusk)
card("Forbidding Waste")                                # passive in Game.might
card("Forgotten Monument", no_score=lambda g, pid, b: g.p[pid].turns < 3)
card("Void Gate")                                       # Bonus Damage in Game.deal
card("Windswept Hillock")                               # Ganking in Game.kw_value
card("Aspirant's Climb")                                # victory score +1 in Game.__init__


def _sigil(g, b, ev, info):
    # "When you conquer here, recycle one of your runes."
    if ev == "conquer" and info["bf"] == b.idx:
        def res(g_, it):
            rs = g_.p[it.ctrl].runes
            if rs:
                r = g_.ask(it.ctrl, "recycle_rune", list(rs))
                if not r.exhausted:
                    # a ready rune is first exhausted for its [Add] 1 Energy (rule 429.3.a: Add Reactions resolve
                    # even during an ability's resolution); the energy floats in the Rune Pool, as in Game.pay
                    g_.p[it.ctrl].pool_e += 1
                g_.recycle_rune(it.ctrl, r)
        g.queue_trigger(info["pid"], "Sigil of the Storm", res)


card("Sigil of the Storm", bf_event=_sigil)


def _star_spring(g, b, ev, info):
    # "The first time a player plays a non-token unit here each turn, they may move another unit they control
    # here to its base."
    if ev == "played" and info["card"].spec["type"] == "Unit" and not info["card"].token and info["card"].loc == b.idx:
        key = ("star", b.idx, info["pid"], g.turn_no)
        if key in g.stats:
            return
        g.stats[key] = 1
        played = info["card"]

        def choose(g_, it):
            # comparer par uid : la table rejoue la partie depuis une copie profonde (choix humain), « is » y échoue
            us = [u for u in g_.units(it.ctrl, b.idx) if u.uid != played.uid]
            if not us:
                return False
            u = g_.ask(it.ctrl, "star_spring", us)
            g_.add_target(it, u, lambda g2, it2, o: o.loc == b.idx)
            return True

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.move([u], "base", it.ctrl)
        g.queue_trigger(info["pid"], "Star Spring", res, may=True, choose=choose)


card("Star Spring", bf_event=_star_spring)


def _targon(g, b, ev, info):
    # "When you conquer here, ready 2 runes at the end of this turn."
    if ev == "conquer" and info["bf"] == b.idx:
        pid = info["pid"]

        def res(g_, it):
            def at_end(g2, eff, inf):
                g2.effects.remove(eff)

                def ready2(g3, it3):
                    ex = [r for r in g3.p[it3.ctrl].runes if r.exhausted]
                    if full_choices(g3, it3.ctrl) and len(ex) > 2 and len({r.domain for r in ex}) > 1:
                        ex = []                   # a human chooses which runes (by domain)
                        for _ in range(2):
                            r = g3.ask(it3.ctrl, "ready_rune", [r for r in g3.p[it3.ctrl].runes if r.exhausted])
                            r.exhausted = False
                    for r in ex[:2]:
                        r.exhausted = False
                g2.queue_trigger(pid, "Targon's Peak (end of turn)", ready2)
            if g_.stage in ("expire",):
                return
            g_.effects.append(dict(on="end_turn", fn=at_end, dur="turn", pid=pid))
        g.queue_trigger(pid, "Targon's Peak", res)


card("Targon's Peak", bf_event=_targon)


def _threshold(g, b, ev, info):
    # "When combat starts here, the attacker and defender each [Add] 1 energy." (Add resolves immediately)
    if ev == "combat_start" and info["bf"] == b.idx:
        sd = info["sd"]
        g.p[sd.attacker].pool_e += 1
        g.p[sd.defender].pool_e += 1
        g.log("  Threshold of the Gray: both players add 1 energy")


card("Threshold of the Gray", bf_event=_threshold)


# ====================================================================== tokens
card("Mech")
card("Reflection")
card("Gold")                                            # Add ability used while paying (Game.pay)


# ====================================================================== Akali deck
def _dw_event(g, o, ev, info):
    # Akali, Deadly Weapon — "When I move, you may deal 1 to a unit at a battlefield I moved to or from.
    # If I'm [Empowered], deal 2 instead."
    if ev == "move" and info["obj"] is o:
        locs = [l for l in (info["frm"], info["to"]) if l in (0, 1)]
        if not locs:
            return
        dmg = 2 if o.empowered else 1

        def opts(g_, it):
            return [u for u in all_units(g_, it.ctrl) if u.loc in locs]

        def res(g_, it):
            u = g_.legal(it, 0)
            if u is not None:
                g_.deal(u, it.data["dmg"], "ability", it.ctrl)
        g.queue_trigger(o.ctrl, "Akali, Deadly Weapon", res, dict(dmg=dmg), src=o.uid, may=True,
                        choose=trig_target(opts, lambda g_, it, u: u.loc in locs))


card("Akali, Deadly Weapon", on_event=_dw_event,
     might_mod=lambda g, o: 1 if o.empowered else 0,
     abilities=[dict(name="Empower", timing="main", can=lambda g, pid, o: not o.empowered,
                     cost=lambda g, pid, o, ch: (2, [frozenset({"Fury"})]),
                     resolve=lambda g, it: g.obj(it.src) is not None and g.empower(g.obj(it.src)))])


def _silent_event(g, o, ev, info):
    # Akali, Silent — "When I move to a battlefield, give me +2 might this turn."
    if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
        g.queue_trigger(o.ctrl, "Akali, Silent", lambda g_, it: g_.obj(it.src) is not None and g_.mod(g_.obj(it.src), 2),
                        src=o.uid)


card("Akali, Silent", on_event=_silent_event,
     untargetable=lambda g, o, by: by != o.ctrl and not g.in_combat(o))


def _adaptatron(g, o, ev, info):
    # "When I conquer, you may kill a gear. If you do, buff me."
    if ev == "conquer" and info["pid"] == o.ctrl and o in info["units"]:
        def res(g_, it):
            gg = g_.legal(it, 0)
            if gg is not None and g_.kill([gg], it.ctrl):
                me = g_.obj(it.src)
                if me is not None:
                    g_.buff(me)
        g.queue_trigger(o.ctrl, "Adaptatron", res, src=o.uid, may=True,
                        choose=trig_target(lambda g_, it: sorted(g_.gear(), key=lambda x: x.ctrl == it.ctrl), P_gear))


card("Adaptatron", on_event=_adaptatron)


# Against the Odds — "[Reaction] Give a friendly unit at a battlefield +2 might this turn for each enemy unit there."
def _ato(g, it):
    u = g.legal(it, 0)
    if u is not None:
        n = len([e for e in g.units(1 - it.ctrl, u.loc)])
        g.mod(u, 2 * n)


# The AI only considers units facing an enemy; a human may choose any friendly unit at a battlefield, even with no
# enemy there (+0, still a legal play: valid choices for its targets are all it needs, rules 355.8, 355.9.b).
card("Against the Odds", timing="reaction", preds=[P_friend_bf], resolve=_ato,
     choices=lambda g, pid, ctx: tg_choices([u for u in friends(g, pid, True, ctx["hidden_bf"])
                                             if g.units(1 - pid, u.loc)]),
     all_choices=lambda g, pid, ctx: tg_choices(sorted(friends(g, pid, True, ctx["hidden_bf"]),
                                                       key=lambda u: not g.units(1 - pid, u.loc))))


def _heron_event(g, o, ev, info):
    # Astral Heron — "When you play your first card each turn, if I'm at a battlefield, your next card costs
    # [2] and [A][A] less."
    if ev == "played" and info["pid"] == o.ctrl and info["n"] == 1 and o.loc in (0, 1) and not info["card"].token:
        pid = o.ctrl
        g.queue_trigger(pid, "Astral Heron", lambda g_, it: g_.effects.append(dict(kind="heron", pid=pid)), src=o.uid)


card("Astral Heron", on_event=_heron_event)


# Back Off — "[Hidden] [Action] [Stun] a unit. If you played this from your hand, draw 1."
def _back_off(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.stun(u, it.ctrl)
    if it.data.get("from") == "hand":
        g.draw(it.ctrl, 1)


def _back_off_choices(g, pid, ctx):
    # "[Stun] a unit" : n'importe quelle unité, amie ou ennemie, même déjà étourdie ; depuis Hidden, une unité de ce
    # battlefield (811.1.d). L'IA garde sa liste courte : ennemis pas encore étourdis.
    hb = ctx["hidden_bf"]
    if full_choices(g, pid):
        return tg_choices(all_units(g, pid, False, hb))
    return tg_choices([u for u in enemies(g, pid, False, hb) if not u.stunned])


card("Back Off", timing="action", hidden=True, preds=[P_unit], resolve=_back_off, choices=_back_off_choices)


# Blitzcrank, Impassive — "[Tank] When you play me to a battlefield, you may move an enemy unit to here.
# When I hold, return me to my owner's hand."
def _blitz_play(g, o, ctx):
    if o.loc not in (0, 1):
        return
    here = o.loc

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.move([u], here, it.ctrl)
    g.queue_trigger(o.ctrl, "Blitzcrank", res, src=o.uid, may=True,
                    choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc != here],
                                       lambda g_, it, u: u.ctrl != it.ctrl))


def _blitz_event(g, o, ev, info):
    if ev == "hold" and info["pid"] == o.ctrl and o in info["units"]:
        g.queue_trigger(o.ctrl, "Blitzcrank (hold)", lambda g_, it: g_.obj(it.src) is not None and
                        g_.to_zone(g_.obj(it.src), "hand"), src=o.uid)


card("Blitzcrank, Impassive", kw={"Tank": 1}, on_play=_blitz_play, on_event=_blitz_event)


# Block — "[Hidden] [Action] Give a unit [Shield 3] and [Tank] this turn."
def _block(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.grant(u, "Shield", 3)
        g.grant(u, "Tank", 1)


card("Block", timing="action", hidden=True, preds=[P_unit], resolve=_block,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid, False, ctx["hidden_bf"])))


# Brittle Steel — "Kill a gear. [Flow] [4][Fury]"
def _kill_gear(g, it):
    gg = g.legal(it, 0)
    if gg is not None:
        g.kill([gg], it.ctrl)


card("Brittle Steel", preds=[P_gear], resolve=_kill_gear, flow=(4, 1, ("Fury",)),
     choices=lambda g, pid, ctx: tg_choices(sorted(g.gear(), key=lambda x: (x.ctrl == pid, -x.spec["e"]))))


# Charm — "Move an enemy unit."
def _charm(g, it):
    u = g.legal(it, 0)
    if u is not None and u.loc != it.data["dest"]:
        g.move([u], it.data["dest"], it.ctrl)


def _charm_choices(g, pid, ctx):
    out = []
    for u in enemies(g, pid):
        for d in ("base", 0, 1):
            if d != u.loc:
                out.append(dict(tg=(u.uid,), dest=d))
    out.sort(key=lambda c: (c["dest"] != "base",))
    return out


card("Charm", preds=[P_enemy], resolve=_charm, choices=_charm_choices, max_choices=10)


# Crumbling Sands — "[Reaction] Counter a spell if an opponent has played another spell this turn."
def _crumbling(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is None:
        return
    opp = 1 - it.ctrl
    n = g.spells_played[opp] - (1 if tgt.ctrl == opp else 0)
    if n >= 1:
        g.counter(tgt)


card("Crumbling Sands", timing="reaction", resolve=_crumbling,
     choices=lambda g, pid, ctx: [dict(item=i.id) for i in g.chain if i.kind == "spell" and i.ctrl != pid])


# Darius, Trifarian — "When you play your second card in a turn, give me +2 might this turn and ready me."
def _darius(g, o, ev, info):
    if ev == "played" and info["pid"] == o.ctrl and info["n"] == 2:
        def res(g_, it):
            me = g_.obj(it.src)
            if me is not None:
                g_.mod(me, 2)
                g_.ready_obj(me)
        g.queue_trigger(o.ctrl, "Darius", res, src=o.uid)


card("Darius, Trifarian", on_event=_darius)


# Decree of Focus — "[Reaction] Choose a friendly unit that's in combat with an enemy Fury unit or that's being
# chosen by an enemy Fury spell. Give it +4 might this turn."
def _dof_ok(g, pid, u):
    if g.in_combat(u) and any("Fury" in e.spec["domains"] for e in g.units(1 - pid, u.loc) if e.desig):
        return True
    return any(i.kind == "spell" and i.ctrl != pid and "Fury" in i.card.spec["domains"] and u.uid in i.chosen for i in g.chain)


card("Decree of Focus", timing="reaction", preds=[P_friend],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), 4),
     choices=lambda g, pid, ctx: tg_choices([u for u in friends(g, pid) if _dof_ok(g, pid, u)]))


# Decree of Insight — "[Reaction] Ignore [Deflect] while paying this spell's cost. Give an enemy Body unit -5
# might this turn."
card("Decree of Insight", timing="reaction",
     preds=[lambda g, it, o: P_enemy(g, it, o) and "Body" in o.spec["domains"]],
     resolve=lambda g, it: g.legal(it, 0) is not None and g.mod(g.legal(it, 0), -5),
     choices=lambda g, pid, ctx: tg_choices([u for u in enemies(g, pid) if "Body" in u.spec["domains"]]))


# Decree of Rage — "[Action] This can't be countered. Deal 4 to an enemy Calm unit."
card("Decree of Rage", timing="action", uncounterable=True,
     preds=[lambda g, it, o: P_enemy(g, it, o) and "Calm" in o.spec["domains"]],
     resolve=lambda g, it: g.deal(g.legal(it, 0), 4, "spell", it.ctrl),
     choices=lambda g, pid, ctx: tg_choices([u for u in enemies(g, pid) if "Calm" in u.spec["domains"]]))


# Decree of Unity — "Kill an enemy Chaos unit or gear."
card("Decree of Unity",
     preds=[lambda g, it, o: o.ctrl != it.ctrl and "Chaos" in o.spec["domains"]],
     resolve=lambda g, it: g.kill([g.legal(it, 0)], it.ctrl),
     choices=lambda g, pid, ctx: tg_choices([o for o in g.board if o.ctrl != pid and "Chaos" in o.spec["domains"]]))


# Defy — "[Reaction] Counter a spell that costs no more than 4 energy and no more than 1 rune of any type."
def _defy(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is not None and tgt.kind == "spell" and tgt.card.spec["e"] <= 4 and tgt.card.spec["p"] <= 1:
        g.counter(tgt)


card("Defy", timing="reaction", resolve=_defy,
     choices=lambda g, pid, ctx: [dict(item=i.id) for i in reversed(g.chain) if i.kind == "spell" and i.ctrl != pid
                                  and i.card.spec["e"] <= 4 and i.card.spec["p"] <= 1 and g.counterable(i)])


# Disarming Rake — "When you play me, you may kill a gear."
def _rake(g, o, ctx):
    g.queue_trigger(o.ctrl, "Disarming Rake", _kill_gear, src=o.uid, may=True,
                    choose=trig_target(lambda g_, it: sorted(g_.gear(), key=lambda x: x.ctrl == it.ctrl), P_gear))


card("Disarming Rake", on_play=_rake)


# Discipline — "[Reaction] Give a unit +2 might this turn. Draw 1."
def _discipline(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.mod(u, 2)
    g.draw(it.ctrl, 1)


card("Discipline", timing="reaction", preds=[P_unit], resolve=_discipline,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid)))


# En Garde — "[Reaction] Give a friendly unit +1 might this turn, then an additional +1 might this turn if it is
# the only unit you control there."
def _en_garde(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.mod(u, 1)
        if g.alone(u):
            g.mod(u, 1)


card("En Garde", timing="reaction", preds=[P_friend], resolve=_en_garde,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid)))


# Falling Star — "Deal 3 to a unit. Deal 3 to a unit."
def _falling_star(g, it):
    for i in range(2):
        u = g.legal(it, i)
        if u is not None:
            g.deal(u, 3, "spell", it.ctrl)


def _fs_choices(g, pid, ctx):
    es = cap(g, enemies(g, pid), 4, pid)
    out = []
    for i, a in enumerate(es):
        for b in es[i:]:
            out.append(dict(tg=(a.uid, b.uid)))
    def score(c):
        a, b = g.obj(c["tg"][0]), g.obj(c["tg"][1])
        if a is b:
            return -(value(g, a) if g.might(a) - a.damage > 3 else value(g, a) - 3)
        return -(value(g, a) * (g.might(a) - a.damage <= 3) + value(g, b) * (g.might(b) - b.damage <= 3))
    out.sort(key=score)
    return out


card("Falling Star", preds=[P_unit, P_unit], resolve=_falling_star, choices=_fs_choices)


# Ferrous Forerunner — "[Deathknell] Play two 3 might Mech unit tokens to your base."
def _forerunner_dk(g, it):
    for _ in range(2):
        make_token(g, "Mech", it.ctrl, "base")


card("Ferrous Forerunner", deathknell=_forerunner_dk)


# Irelia, Fervent — "[Deflect] When you choose or ready me, give me +1 might this turn."
def _irelia(g, o, ev, info):
    if (ev == "chosen" and info["obj"] is o and info["item"].ctrl == o.ctrl) or (ev == "ready" and info["obj"] is o):
        g.queue_trigger(o.ctrl, "Irelia", lambda g_, it: g_.obj(it.src) is not None and g_.mod(g_.obj(it.src), 1),
                        src=o.uid)


card("Irelia, Fervent", kw={"Deflect": 1}, on_event=_irelia)


# Kai'Sa, Survivor — "[Accelerate] When I conquer, draw 1."
def _kaisa(g, o, ev, info):
    if ev == "conquer" and info["pid"] == o.ctrl and o in info["units"]:
        g.queue_trigger(o.ctrl, "Kai'Sa", lambda g_, it: g_.draw(it.ctrl, 1), src=o.uid)


card("Kai'Sa, Survivor", accelerate=True, on_event=_kaisa)


# Lonely Poro — "[Deathknell] If I died alone, draw 1."
card("Lonely Poro", deathknell=lambda g, it: it.data["info"]["alone"] and g.draw(it.ctrl, 1))


# Long Sword / Sterak's Gage — "[Quick-Draw] [Equip] [Fury]/[Calm]" ; Pendulum Blade — "[Equip] [Fury]"
def equip_ability(domain):
    """Equip ability with a cost of one Power of domain (see equip_ability_cost for any cost)."""
    return dict(name="Equip", timing="main",
                choices=lambda g, pid, o: [dict(tg=(u.uid,)) for u in friends(g, pid) if u.uid != o.attached_to],
                cost=lambda g, pid, o, ch: (0, [frozenset({domain})]),
                preds=[P_friend],
                resolve=lambda g, it: (g.legal(it, 0) is not None and g.obj(it.src) is not None
                                       and attach(g, g.obj(it.src), g.legal(it, 0))))


card("Long Sword", timing="reaction", quickdraw=True, abilities=[equip_ability("Fury")])
card("Sterak's Gage", timing="reaction", quickdraw=True, abilities=[equip_ability("Calm")])


def _pendulum_effect(g, gear, unit, ev, info):
    # Effect text (attached): the equipped unit gets +2 might this turn when it moves to a battlefield.
    if ev == "move" and info["obj"] is unit and info["to"] in (0, 1):
        g.queue_trigger(unit.ctrl, "Pendulum Blade", lambda g_, it: g_.obj(it.src) is not None and
                        g_.mod(g_.obj(it.src), 2), src=unit.uid)


card("Pendulum Blade", abilities=[equip_ability("Fury")], effect_event=_pendulum_effect)


# Mischievous Marai — "[Hidden] When you play me to a battlefield, deal 2 to an enemy unit here."
def _marai(g, o, ctx):
    if o.loc not in (0, 1):
        return
    here = o.loc

    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.deal(u, 2, "ability", it.ctrl)
    g.queue_trigger(o.ctrl, "Mischievous Marai", res, src=o.uid,
                    choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                                       lambda g_, it, u: u.loc == here and u.ctrl != it.ctrl))


card("Mischievous Marai", hidden=True, on_play=_marai)


# Mournful Witness — "When a combat that I was in ends, empower me. [Empowered] I have +2 might."
def _witness(g, o, ev, info):
    if ev == "combat_end" and o in info["members"]:
        g.queue_trigger(o.ctrl, "Mournful Witness", lambda g_, it: g_.obj(it.src) is not None and
                        g_.empower(g_.obj(it.src)), src=o.uid)


card("Mournful Witness", on_event=_witness, might_mod=lambda g, o: 2 if o.empowered else 0)


# Not So Fast — "[Reaction] Counter an enemy spell or ability that chooses a friendly unit or gear."
def _nsf_items(g, pid):
    mine = set(o.uid for o in g.board if o.ctrl == pid)
    return [i for i in reversed(g.chain) if i.ctrl != pid and (i.chosen & mine) and g.counterable(i)]


def _nsf(g, it):
    tgt = item_by_id(g, it.data.get("item"))
    if tgt is not None:
        g.counter(tgt)


card("Not So Fast", timing="reaction", resolve=_nsf,
     choices=lambda g, pid, ctx: [dict(item=i.id) for i in _nsf_items(g, pid)])


# Noxus Hopeful — "[Legion] I cost 2 energy less."
card("Noxus Hopeful", cost_mod=lambda g, pid, c, ch: (2, 0) if g.finalized[pid] else (0, 0))


# Scuttle Crab — "When you play me, draw 1. [Deathknell] Choose an opponent. They reveal their hand. You can look
# at their facedown cards this turn. Gain 1 XP."
def _crab_dk(g, it):
    g.reveal_hand(1 - it.ctrl, it.ctrl)
    g.p[1 - it.ctrl].revealed_turn = g.turn_no     # cartes face cachée visibles ce tour
    g.gain_xp(it.ctrl, 1)


card("Scuttle Crab", on_play=lambda g, o, ctx: g.queue_trigger(o.ctrl, "Scuttle Crab", lambda g_, it: g_.draw(it.ctrl, 1),
                                                                src=o.uid),
     deathknell=_crab_dk)


# Shuriken Flip — "Deal 2 to up to one enemy unit at a battlefield, then move a friendly unit.
# [Flow] [3][A]"
def _flip(g, it):
    if it.data.get("tg") and it.data["tg"][0] is not None:
        u = g.legal(it, 0)
        if u is not None:
            g.deal(u, 2, "spell", it.ctrl)
    m = g.obj(it.data.get("mover")) if it.data.get("mover") else None
    if m is not None and m.ctrl == it.ctrl and m.oid == it.data.get("mover_oid") and m.loc != it.data["dest"]:
        if it.data.get("hidden_bf") is None or True:
            g.move([m], it.data["dest"], it.ctrl)


def _flip_choices(g, pid, ctx):
    hb = ctx["hidden_bf"]
    es = cap(g, enemies(g, pid, True, hb), 3, pid)
    tgs = [(e.uid,) for e in es] + [()]
    fr = friends(g, pid)
    if hb is not None:
        fr = [u for u in fr if u.loc == hb] or fr
    moves = []
    for u in fr:
        for d in ("base", 0, 1):
            if d != u.loc:
                moves.append((u.uid, u.oid, d))
    out = []
    for t in tgs:
        for (m, oid, d) in moves:
            out.append(dict(tg=t, mover=m, mover_oid=oid, dest=d))
        if not moves:
            out.append(dict(tg=t, mover=None, dest=None))

    def score(c):
        s = 0
        if c["tg"]:
            e = g.obj(c["tg"][0])
            s -= value(g, e) * (g.might(e) - e.damage <= 2) + 1
        if c.get("mover"):
            m = g.obj(c["mover"])
            if c["dest"] in (0, 1) and g.bfs[c["dest"]].ctrl != pid:
                s -= 2 + g.might(m) / 2
            if m.cname in ("Akali, Deadly Weapon", "Stellacorn Herder"):
                s -= 1
        return s
    out.sort(key=score)
    return out


card("Shuriken Flip", flow=(3, 1, tuple(sorted(("Fury", "Calm", "Mind", "Body", "Chaos", "Order")))),
     preds=[P_enemy_bf], resolve=_flip, choices=_flip_choices, max_choices=12)


# Sky Splitter — "[Action] This spell's Energy cost is reduced by the highest Might among units you control.
# Deal 5 to a unit at a battlefield."
card("Sky Splitter", timing="action", preds=[P_unit_bf],
     cost_mod=lambda g, pid, c, ch: (max([g.might(u) for u in g.units(pid)] + [0]), 0),
     resolve=lambda g, it: g.deal(g.legal(it, 0), 5, "spell", it.ctrl),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# Stellacorn Herder — "When I move, draw 1."
def _herder(g, o, ev, info):
    if ev == "move" and info["obj"] is o:
        g.queue_trigger(o.ctrl, "Stellacorn Herder", lambda g_, it: g_.draw(it.ctrl, 1), src=o.uid)


card("Stellacorn Herder", on_event=_herder)


# Tomb-Raider Barbara — "When you play me, if you control 7 or more runes, choose an enemy gear. If it's
# [Empowered], disempower it. Otherwise, kill it."
def _barbara(g, o, ctx):
    if len(g.p[o.ctrl].runes) < 7:
        return

    def res(g_, it):
        gg = g_.legal(it, 0)
        if gg is None:
            return
        if gg.empowered:
            gg.empowered = False
        else:
            g_.kill([gg], it.ctrl)
    g.queue_trigger(o.ctrl, "Tomb-Raider Barbara", res, src=o.uid,
                    choose=trig_target(lambda g_, it: [x for x in g_.gear() if x.ctrl != it.ctrl],
                                       lambda g_, it, x: x.ctrl != it.ctrl))


card("Tomb-Raider Barbara", on_play=_barbara)


# Zhonya's Hourglass — "[Hidden] If a friendly unit would die, kill this instead. Heal that unit, exhaust it,
# and recall it." (errata; replacement effect applied by Game.kill, rules 370-373). An Hourglass dying in the same
# event doesn't save a unit (as in the original engine).
def _zhonya_rep(g, h, u):
    if u.ctrl != h.ctrl or u.spec["type"] != "Unit" or h in g._dying:
        return None

    def apply(g_, x):
        g_.stats[f"zhonya_save_P{h.ctrl}"] += 1
        g_.kill([h], h.ctrl)
        g_.save_unit(x)
    return dict(name="Zhonya's Hourglass", apply=apply)


card("Zhonya's Hourglass", hidden=True, death_rep=_zhonya_rep)


# ====================================================================== LeBlanc deck
# Baited Hook — "[1][Order], [E]: Kill a friendly unit. Look at the top 5 cards of your Main Deck. You may banish
# a unit from among them that has Might up to 1 more than the killed unit and play it, ignoring its cost.
# Then recycle the rest."
def _hook(g, it):
    pid = it.ctrl
    u = g.legal(it, 0)
    killed_might = None
    if u is not None:
        killed_might = g.might(u)
        g.kill([u], pid)
    pl = g.p[pid]
    top = g.look(pid, pl.deck[:5])
    if killed_might is not None:
        cands = [c for c in top if c.spec["type"] == "Unit" and (c.spec["might"] or 0) <= killed_might + 1]
    else:
        cands = []
    pick = g.ask(pid, "hook_pick", [None] + sorted(cands, key=lambda c: -(c.spec["might"] or 0) - c.spec["e"] / 10))
    rest = [c for c in top if c is not pick]
    if pick is not None:
        g.log(f"  Baited Hook finds {pick.cname}")
        g.stats["hook_hit"] += 1
        pl.deck.remove(pick)
        pick.zone = "banish"
        pl.banish.append(pick)
        play_unit_free(g, pid, pick, "banish")
    for c in rest:
        if c in pl.deck:
            pl.deck.remove(c)
    g.recycle_cards(pid, rest)


card("Baited Hook", abilities=[dict(
    name="Hook", timing="main", exhaust=True, preds=[P_friend],
    choices=lambda g, pid, o: [dict(tg=(u.uid,)) for u in sorted(g.units(pid), key=lambda u: value(g, u))],
    cost=lambda g, pid, o, ch: (1, [frozenset({"Order"})]), resolve=_hook)])


# Bellows Breath — "[Action] [Repeat] [1][Mind] Deal 1 to up to three units at the same location."
def _bellows(g, it):
    groups = [[g.legal(it, i) for i in range(len(it.targets))]]
    if it.data.get("t2"):
        groups.append([g.obj(u) if g.obj(u) is not None and g.obj(u).oid == oid else None for u, oid in it.data["t2"]])
    for grp in groups:
        grp = [u for u in grp if u is not None]
        if not grp:
            continue
        loc = max(set(u.loc for u in grp), key=lambda l: sum(1 for u in grp if u.loc == l))
        for u in grp:
            if u.loc == loc:
                g.deal(u, 1, "spell", it.ctrl)


def _bellows_choices(g, pid, ctx):
    out = []
    locs = set(u.loc for u in enemies(g, pid, False, ctx["hidden_bf"]))
    for l in sorted(locs, key=str):
        es = [u for u in enemies(g, pid) if u.loc == l][:3]
        out.append(dict(tg=tuple(u.uid for u in es)))
    out.sort(key=lambda c: -sum(1 for u in c["tg"] if g.might(g.obj(u)) - g.obj(u).damage <= 1))
    if full_choices(g, pid):                       # a human: any one to three units at one location
        from itertools import combinations
        hb = ctx["hidden_bf"]
        for l in sorted(set(u.loc for u in g.units()), key=str):
            if hb is not None and l != hb:
                continue
            us = sorted([u for u in g.units() if u.loc == l and g.targetable(u, pid)], key=lambda u: u.uid)
            for k in (1, 2, 3):
                for grp in combinations(us, k):
                    ch = dict(tg=tuple(u.uid for u in grp))
                    if ch not in out:
                        out.append(ch)
    return out + [dict(tg=())]                     # "up to three" includes zero (rule 355.13)


card("Bellows Breath", timing="action", repeat=(1, ("Mind",), 1), preds=[P_unit], resolve=_bellows,
     choices=_bellows_choices)


# Black Rose Dignitary — "[Assault] [Deathknell] Channel 1 rune exhausted."
card("Black Rose Dignitary", kw={"Assault": 1}, deathknell=lambda g, it: g.channel(it.ctrl, 1, exhausted=True))


# Chakram Dancer — "[Ambush] When you play me, give your other units here [Shield] this turn."
def _chakram(g, o, ctx):
    here = o.loc

    def res(g_, it):
        for u in g_.units(it.ctrl, here):
            if u.uid != it.src:
                g_.grant(u, "Shield", 1)
    g.queue_trigger(o.ctrl, "Chakram Dancer", res, src=o.uid)


card("Chakram Dancer", ambush=True, on_play=_chakram)


# Cull the Weak — "Each player kills one of their units."
def _cull(g, it):
    victims = []
    for pid in (g.tp, 1 - g.tp):
        us = g.units(pid)
        if us:
            victims.append(g.ask(pid, "sacrifice", sorted(us, key=lambda u: value(g, u))))
    g.kill(victims)


card("Cull the Weak", resolve=_cull)


# Deathgrip — "[Reaction] Kill a friendly unit to give +might equal to its Might to another friendly unit this
# turn. Draw 1."  Deux cibles obligatoires (355.7, 355.8) : tg[0] = l'unité alliée tuée, tg[1] = une AUTRE unité
# alliée. Sans deux unités alliées, le sort ne peut pas être joué (décision de l'utilisateur, 2026-10-08 : suivre les
# règles au plus près). À la résolution, une cible devenue illégale ne fait rien, le reste s'applique (359.3.e).
def _deathgrip(g, it):
    victim, tgt = g.legal(it, 0), g.legal(it, 1)
    if victim is not None:
        m = g.might(victim)                            # sa Might au moment de mourir
        killed = g.kill([victim], it.ctrl)
        if killed and tgt is not None and tgt is not victim and tgt in g.board:
            g.mod(tgt, max(0, m))
    g.draw(it.ctrl, 1)


def _deathgrip_choices(g, pid, ctx):
    fr = friends(g, pid, False, ctx["hidden_bf"])
    victims = cap(g, sorted(fr, key=lambda u: value(g, u)), 3, pid)          # l'IA sacrifie d'abord le moins utile
    gets = cap(g, sorted(fr, key=lambda u: -g.might(u)), 3, pid)
    return [dict(tg=(v.uid, t.uid)) for v in victims for t in gets if t is not v]


card("Deathgrip", timing="reaction", preds=[P_friend, P_friend], resolve=_deathgrip, choices=_deathgrip_choices)


# Glasc Mixologist — "[Deathknell] You may play a unit with cost no more than 3 energy and no more than 1 rune of
# any type from your trash, ignoring its cost."
def _mixo(g, it):
    pid = it.ctrl
    cands = [c for c in g.p[pid].trash if c.spec["type"] == "Unit" and c.spec["e"] <= 3 and c.spec["p"] <= 1]
    pick = g.ask(pid, "mixologist_pick", [None] + sorted(cands, key=lambda c: -(c.spec["might"] or 0) - c.spec["e"]))
    if pick is not None:
        play_unit_free(g, pid, pick, "trash")


card("Glasc Mixologist", deathknell=_mixo)


# Harnessed Dragon — "When you play me, kill an enemy unit."
def _dragon(g, o, ctx):
    hb = ctx.get("hidden_bf")
    g.queue_trigger(o.ctrl, "Harnessed Dragon", lambda g_, it: g_.kill([g_.legal(it, 0)], it.ctrl), src=o.uid,
                    choose=trig_target(lambda g_, it: enemies(g_, it.ctrl, False, hb), P_enemy))


card("Harnessed Dragon", on_play=_dragon)


# Hidden Blade — "[Hidden] [Action] Kill a unit at a battlefield. Its controller draws 2."
def _blade(g, it):
    u = g.legal(it, 0)
    if u is None:
        return                                      # linked instruction ignored too (rule 359.3.e.14.a)
    c = u.ctrl
    g.kill([u], it.ctrl)
    g.draw(c, 2)


card("Hidden Blade", timing="action", hidden=True, preds=[P_unit_bf], resolve=_blade,
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid, True, ctx["hidden_bf"])))


# Honest Broker — "[Deathknell] Play a Gold gear token exhausted."
def _broker(g, it):
    make_token(g, "Gold", it.ctrl, "base", ready=False)


card("Honest Broker", deathknell=_broker)


# Karthus, Eternal — "Your [Deathknell] effects trigger an additional time." (in Game.kill)
card("Karthus, Eternal")


# Kennen, Keeper of Balance — "[Hidden] When you play me or I attack, you may pay [2] to [Stun] a unit.
# While there's a stunned enemy unit here, I have +2 might."
def _kennen_trigger(g, o, hb=None):
    def res(g_, it):
        u = g_.legal(it, 0)
        if u is not None:
            g_.stun(u, it.ctrl)
    g.queue_trigger(o.ctrl, "Kennen", res, src=o.uid, may=True,
                    choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl, False, hb) if not u.stunned]
                                       + [u for u in friends(g_, it.ctrl) if hb is None or u.loc == hb][:0], P_unit),
                    cost=may_pay(2))


card("Kennen, Keeper of Balance", hidden=True,
     on_play=lambda g, o, ctx: _kennen_trigger(g, o, ctx.get("hidden_bf")),
     on_event=lambda g, o, ev, info: ev == "attack" and info["obj"] is o and _kennen_trigger(g, o),
     might_mod=lambda g, o: 2 if any(u.stunned for u in g.units(1 - o.ctrl, o.loc)) else 0)


# Ki Barrier — "[Reaction] Choose a unit. Prevent the next 7 damage that would be dealt to it this turn."
def _ki(g, it):
    u = g.legal(it, 0)
    if u is not None:
        u.prevent += 7


card("Ki Barrier", timing="reaction", preds=[P_unit], resolve=_ki,
     choices=lambda g, pid, ctx: tg_choices(friends(g, pid)))


# LeBlanc, Everywhere At Once — "[Backline] Your [Temporary] effects at my battlefield don't trigger."
card("LeBlanc, Everywhere At Once", kw={"Backline": 1})


# LeBlanc, Fragmented — "[Assault] [Deathknell] Draw 1. If it's your Beginning Phase, draw 2 instead."
card("LeBlanc, Fragmented", kw={"Assault": 1},
     deathknell=lambda g, it: g.draw(it.ctrl, 2 if (g.tp == it.ctrl and g.stage in ("beginning", "scoring")) else 1))


# Mirror Image — "Choose a unit. Play a ready Reflection unit token to your base. It becomes a copy of that unit.
# Give it [Temporary]."
def _mirror(g, it):
    u = g.legal(it, 0)
    _reflection(g, it.ctrl, "base", u)


card("Mirror Image", preds=[P_unit], resolve=_mirror,
     choices=lambda g, pid, ctx: tg_choices(sorted([u for u in g.units() if g.targetable(u, pid)],
                                                   key=lambda u: -value(g, u))))


# Rift Herald — "When I move to a battlefield, look at the top 3 cards of your Main Deck. You may reveal a unit
# from among them and draw it. Recycle the rest. [Deathknell] Play a unit from your hand to your base, ignoring
# its Energy cost. (You must still pay its Power cost.)"
def _herald_move(g, o, ev, info):
    if ev == "move" and info["obj"] is o and info["to"] in (0, 1):
        def res(g_, it):
            pl = g_.p[it.ctrl]
            top = g_.look(it.ctrl, pl.deck[:3])
            us = [c for c in top if c.spec["type"] == "Unit"]
            pick = g_.ask(it.ctrl, "herald_pick", [None] + us)
            rest = [c for c in top if c is not pick]
            if pick is not None:
                g_.draw_card(it.ctrl, pick)
            for c in rest:
                pl.deck.remove(c)
            g_.recycle_cards(it.ctrl, rest)
        g.queue_trigger(o.ctrl, "Rift Herald", res, src=o.uid)


def _herald_dk(g, it):
    pid = it.ctrl
    cands = [c for c in g.p[pid].hand if c.spec["type"] == "Unit"
             and g.can_pay(pid, 0, g.power_reqs(c.spec["domains"], c.spec["p"]))]
    pick = g.ask(pid, "herald_dk_pick", sorted(cands, key=lambda c: -c.spec["e"]))
    if pick is not None:
        play_unit_free(g, pid, pick, "hand", loc_choices=["base"], ignore_energy=True)


card("Rift Herald", on_event=_herald_move, deathknell=_herald_dk)


# Ruined Rex — "[Deathknell] Deal 4 to an enemy unit."
def _rex_dk(g, it):
    u = g.legal(it, 0)
    if u is not None:
        g.deal(u, 4, "ability", it.ctrl)


IMPL_REX = card("Ruined Rex", deathknell=_rex_dk,
                dk_choose=trig_target(lambda g_, it: enemies(g_, it.ctrl), P_enemy))

# Sacrifice — "[Reaction] As an additional cost to play this, kill a friendly [Mighty] unit. Draw 2 and channel 1
# rune exhausted."
card("Sacrifice", timing="reaction",
     choices=lambda g, pid, ctx: [dict(kill=u.uid) for u in sorted(g.units(pid), key=lambda u: value(g, u)) if g.mighty(u)],
     resolve=lambda g, it: (g.draw(it.ctrl, 2), g.channel(it.ctrl, 1, exhausted=True)))


# Safety Inspector — "You may spend 3 XP as an additional cost to play me. When you play me, each player must kill
# one of their units. If you paid my additional cost, you don't kill a unit this way."
def _inspector(g, o, ctx):
    paid = ctx.get("xp")

    def res(g_, it):
        victims = []
        for pid in (g_.tp, 1 - g_.tp):
            if pid == it.ctrl and paid:
                continue
            us = g_.units(pid)
            if us:
                victims.append(g_.ask(pid, "sacrifice", sorted(us, key=lambda u: value(g_, u))))
        g_.kill(victims)
    g.queue_trigger(o.ctrl, "Safety Inspector", res, src=o.uid)


card("Safety Inspector", on_play=_inspector,
     as_played=lambda g, pid, c: [dict()] + ([dict(xp=True)] if g.p[pid].xp >= 3 else []))


# Salvage — "[Action] You may kill a gear. Draw 1."
def _salvage(g, it):
    if it.targets:
        gg = g.legal(it, 0)
        if gg is not None:
            g.kill([gg], it.ctrl)
    g.draw(it.ctrl, 1)


card("Salvage", timing="action", preds=[P_gear], resolve=_salvage,
     choices=lambda g, pid, ctx: tg_choices([x for x in g.gear() if x.ctrl != pid]) + [dict(tg=())])


# Soaring Scout — "[Deathknell] Channel 1 rune exhausted."
card("Soaring Scout", deathknell=lambda g, it: g.channel(it.ctrl, 1, exhausted=True))

# Stupefy — "[Reaction] Give a unit -1 might this turn, to a minimum of 1 might. Draw 1."
card("Stupefy", timing="reaction", preds=[P_unit],
     resolve=lambda g, it: (g.legal(it, 0) is not None and g.mod(g.legal(it, 0), -1, minimum=1), g.draw(it.ctrl, 1)),
     choices=lambda g, pid, ctx: tg_choices(enemies(g, pid)))


# Thousand-Tailed Watcher — "[Accelerate] When you play me, give enemy units -3 might this turn, to a minimum of 1."
def _watcher(g, o, ctx):
    def res(g_, it):
        for u in g_.units(1 - it.ctrl):
            g_.mod(u, -3, minimum=1)
    g.queue_trigger(o.ctrl, "Thousand-Tailed Watcher", res, src=o.uid)


card("Thousand-Tailed Watcher", accelerate=True, on_play=_watcher)


# Time Warp — "Take a turn after this one. Banish this."
card("Time Warp", banish_after=True, resolve=lambda g, it: g.extra_turns.insert(0, it.ctrl))


# Turn to Dust — "Give a gear [Temporary]."
card("Turn to Dust", preds=[P_gear], resolve=lambda g, it: g.legal(it, 0) is not None and g.grant(g.legal(it, 0), "Temporary", 1, None),
     choices=lambda g, pid, ctx: tg_choices([x for x in g.gear() if x.ctrl != pid]))


# Vi, Peacekeeper — "[Ambush] When I attack, [Stun] an enemy unit here."
def _vi(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc
        g.queue_trigger(o.ctrl, "Vi", lambda g_, it: g_.stun(g_.legal(it, 0), it.ctrl), src=o.uid,
                        choose=trig_target(lambda g_, it: [u for u in enemies(g_, it.ctrl) if u.loc == here],
                                           lambda g_, it, u: u.loc == here and u.ctrl != it.ctrl))


card("Vi, Peacekeeper", ambush=True, on_event=_vi)

# Watchful Sentry — "[Deathknell] Draw 1."
card("Watchful Sentry", deathknell=lambda g, it: g.draw(it.ctrl, 1))


# Ashe, Focused — "When you play me, choose an opponent. They reveal their hand. Choose a card revealed this way
# and banish it. When they hold, return it to their hand (even if I'm no longer on the board)."
def _ashe(g, o, ctx):
    def res(g_, it):
        opp = 1 - it.ctrl
        g_.reveal_hand(opp, it.ctrl)
        hand = g_.p[opp].hand
        if not hand:
            return
        c = g_.ask(it.ctrl, "ashe_pick", sorted(hand, key=lambda x: -(x.spec["e"] + 2 * x.spec["p"])))
        g_.to_zone(c, "banish")

        def back(g2, eff, info):
            # relire la carte par uid/oid : la table (choix humain, conseil) et l'IA (clone) travaillent sur une copie
            # profonde de la partie, où « c » capturé n'est plus l'objet de l'exil
            x = next((y for y in g2.p[opp].banish if y.uid == eff["uid"] and y.oid == eff["oid"]), None)
            if info["pid"] == opp and x is not None:
                g2.effects.remove(eff)
                g2.p[opp].banish.remove(x)
                x.zone = "hand"; g2.p[opp].hand.append(x)
        g_.effects.append(dict(on="hold", fn=back, uid=c.uid, oid=c.oid))
    g.queue_trigger(o.ctrl, "Ashe", res, src=o.uid)


card("Ashe, Focused", on_play=_ashe)


# Atakhan — "You may kill a friendly unit as an additional cost to play me. If you do, I cost [1] less for each
# Energy it costs and [Order] less for each Power it costs. [Ganking] When I attack, the defender must kill one of
# their units here."
def _atakhan_cost(g, pid, c, ch):
    k = g.obj(ch["kill"]) if ch.get("kill") else None
    if k is None:
        return (0, 0)
    return (k.spec["e"] if not k.token or k.copy else 0, k.spec["p"] if not k.token or k.copy else 0)


def _atakhan_event(g, o, ev, info):
    if ev == "attack" and info["obj"] is o:
        here = o.loc

        def res(g_, it):
            d = 1 - it.ctrl
            us = g_.units(d, here)
            if us:
                g_.kill([g_.ask(d, "sacrifice", sorted(us, key=lambda u: value(g_, u)))], d)
        g.queue_trigger(o.ctrl, "Atakhan", res, src=o.uid)


card("Atakhan", kw={"Ganking": 1}, cost_mod=_atakhan_cost, on_event=_atakhan_event,
     as_played=lambda g, pid, c: [dict()] + [dict(kill=u.uid) for u in cap(g, sorted(g.units(pid),
                                                                                      key=lambda u: value(g, u)), 3, pid)])


# ====================================================================== generic keyword helpers
_COST_PART = re.compile(r"(\d+)\s+(energy|runes? of any type|(fury|calm|mind|body|chaos|order) runes?)", re.I)


def parse_cost(text):
    """'2 energy and 1 fury rune' -> (2, [frozenset({'Fury'})]); '1 rune of any type' -> (0, [ANY]).
    Accepts '', None, an (energy, reqs) tuple, or text as printed in cards_unique.csv."""
    if not text:
        return 0, []
    if isinstance(text, tuple):
        return text[0], list(text[1])
    e, reqs = 0, []
    for m in _COST_PART.finditer(text):
        n, what = int(m.group(1)), m.group(2).lower()
        if what == "energy":
            e += n
        elif "any type" in what:
            reqs += [ANY] * n
        else:
            reqs += [frozenset({m.group(3).capitalize()})] * n
    return e, reqs


def power_list(text):
    """'1 energy and 1 rune of any type' -> (1, ['A']); '1 fury rune' -> (0, ['Fury']) (for Impl.add)."""
    e, reqs = parse_cost(text)
    return e, ["A" if r == ANY else sorted(r)[0] for r in reqs]


def repeat_cost(text):
    """Impl.repeat tuple (energy, domains, power) from '[Repeat] 2 energy and 1 fury rune' text."""
    e, reqs = parse_cost(text)
    doms = sorted(set().union(*reqs)) if reqs else []
    return (e, tuple(doms) if doms else DOMAINS, len(reqs))


def flow_cost(text):
    """Impl.flow tuple (energy, power, domains) from '[Flow] 4 energy and 1 fury rune' text."""
    e, reqs = parse_cost(text)
    doms = sorted(set().union(*reqs)) if reqs else []
    return (e, len(reqs), tuple(doms)) if doms else (e, len(reqs), DOMAINS)


def add_ability(text, exhaust=True, kill=False, can=None):
    """An [Add] ability usable while paying (Impl.add entry): add_ability('1 fury rune'),
    add_ability('1 energy', can=lambda g, pid, o, ctx: ctx is not None and ctx['kind'] == 'spell')."""
    e, p = power_list(text)
    return dict(e=e, p=p, exhaust=exhaust, kill=kill, can=can)


def ability(name, cost=None, timing="main", exhaust=False, xp=0, kill_self=False, resolve=None, choices=None,
            preds=None, can=None, **extra):
    """Activated ability dict for Impl.abilities. cost: text ('1 energy and 1 calm rune'), (e, reqs) or
    fn(g, pid, obj, choice) -> (e, reqs). xp: XP to spend as a cost. kill_self: 'Kill this' cost.
    timing: 'main' | 'action' | 'reaction'. resolve(g, item): item.src is the uid of the source."""
    if callable(cost):
        cost_fn = cost
    else:
        e, reqs = parse_cost(cost)
        cost_fn = lambda g, pid, o, ch: (e, list(reqs))
    ab = dict(name=name, timing=timing, exhaust=exhaust, cost=cost_fn, resolve=resolve or (lambda g, it: None))
    if choices is not None:
        ab["choices"] = choices
    if preds is not None:
        ab["preds"] = preds
    conds = [c for c in (can, (lambda g, pid, o: g.p[pid].xp >= xp) if xp else None) if c is not None]
    if conds:
        ab["can"] = lambda g, pid, o: all(c(g, pid, o) for c in conds)
    extra_fn = extra.pop("extra_cost", None)
    if xp or kill_self or extra_fn:
        def pay_extra(g, pid, o, ch):
            if xp:
                g.spend_xp(pid, xp)
            if extra_fn:
                extra_fn(g, pid, o, ch)
            if kill_self and o in g.board:
                g.kill([o], pid, cost=True)
        ab["extra_cost"] = pay_extra
    ab.update(extra)
    return ab


def equip_ability_cost(cost, timing="main"):
    """[Equip] cost: attach this gear to a unit you control (rule 818). cost as in ability()."""
    if callable(cost):
        cost_fn = cost
    else:
        e, reqs = parse_cost(cost)
        cost_fn = lambda g, pid, o, ch: (e, list(reqs))
    return dict(name="Equip", timing=timing,
                choices=lambda g, pid, o: [dict(tg=(u.uid,)) for u in friends(g, pid) if u.uid != o.attached_to],
                cost=cost_fn, preds=[P_friend],
                resolve=lambda g, it: (g.legal(it, 0) is not None and g.obj(it.src) is not None
                                       and attach(g, g.obj(it.src), g.legal(it, 0))))


def empower_ability(cost, timing="main", exhaust=False):
    """[Empower] cost: "Empower this. Use only if not Empowered" (rule 827). Works on permanents and legends."""
    def resolve(g, it):
        o = g.obj(it.src)
        leg = g.p[it.ctrl].legend
        if o is None and leg.uid == it.src:
            o = leg                      # legend abilities: item.src is the legend's uid
        if o is not None:
            g.empower(o)
    return ability("Empower", cost, timing=timing, exhaust=exhaust, resolve=resolve,
                   can=lambda g, pid, o: not o.empowered)


# dependent keyword conditions for Impl.kw_if / might_if: cond(g, o)
def when_level(n):
    return lambda g, o: g.p[o.ctrl].xp >= n


def when_empowered(g, o):
    return o.empowered


def when_mighty(g, o):
    return g.mighty(o)


def when_legion(g, o):
    return g.legion(o.ctrl, o)


def when_at_bf(g, o):
    return o.loc in (0, 1)


def repeatable(fn):
    """Wrap a single-execution spell resolver fn(g, item) for [Repeat] (rule 820): when the Repeat cost was paid,
    fn runs again with the targets chosen for the repetition (choice 'tg2', checked with the same preds)."""
    def resolve(g, it):
        fn(g, it)
        if it.data.get("rep"):
            it2 = Item("spell", it.ctrl, it.name, card=it.card, src=it.src, data=dict(it.data, rep=False))
            preds = g.impl(it.card).preds or []
            for i, (uid, oid) in enumerate(it.data.get("t2", [])):
                pred = preds[min(i, len(preds) - 1)] if preds else None
                it2.targets.append((uid, oid, pred))
            fn(g, it2)
    return resolve


def token_locations(g, pid, name, locs=None):
    """Locations where pid may play a unit token of this name (default: base or a battlefield they control, rules
    184.2, 355.2), without those where it can't be played (actions.loc_allowed: Rockfall Path, Mageseeker Warden)."""
    from actions import loc_allowed
    probe = Obj.__new__(Obj)                     # a stand-in token (no uid taken from the object counter)
    probe.uid, probe.name, probe.owner, probe.token, probe.zone, probe.oid = 0, name, pid, True, None, 0
    probe.reset()
    if locs is None:
        locs = ["base"] + [b.idx for b in g.bfs if b.ctrl == pid]
    return [l for l in locs if loc_allowed(g, pid, probe, l)]


def make_token(g, name, pid, loc="base", ready=False, _copy=True):
    """Play a unit/gear token (rules 185.2.a, 187) to loc — the single path for every token played by an effect:
    the location must allow it (a unit token can't be played where units can't be played: the instruction is
    skipped, rule 358.3.a, and None is returned), it enters ready if the effect says so or if "your units/tokens
    enter ready" applies, its keyword play triggers ([Vision], [Weaponmaster]) trigger and 'played' is emitted
    (n=0, token=True: tokens are not cards). Replacement "play that token and an additional copy of it instead"
    (Impl.token_rep: Zilean, Time Mage) is offered here. Returns the token (the first one)."""
    from actions import loc_allowed, others_enter_ready, keyword_play_triggers
    t = g.new_token(name, pid)
    if SPEC[name]["type"] == "Unit" and not loc_allowed(g, pid, t, loc):
        g.log(f"  a {name} token can't be played at {loc}")
        return None
    extra = 0
    if _copy and SPEC[name]["type"] == "Unit":
        for src, im in g.providers("token_rep"):
            extra += 1 if im.token_rep(g, src, pid, name) else 0
    if not ready and others_enter_ready(g, pid, t):          # "Your tokens enter ready" (rule 369.3)
        ready = True
    g.enter_board(t, pid, loc, ready=ready)
    g.log(f"  P{pid} plays a {name} token at {loc}{' ready' if ready else ''}")
    keyword_play_triggers(g, pid, t)
    g.played_event(pid, t)
    for _ in range(extra):
        make_token(g, name, pid, loc, ready, _copy=False)
    return t


def pay_deflect(g, it, o):
    """Pay the Deflect cost of o for a triggered ability item it (rule 809). Returns False if unpayable."""
    reqs = deflect_reqs(g, it.ctrl, dict(tg=(o.uid,)))
    if not reqs:
        return True
    return g.pay(it.ctrl, 0, reqs, dict(kind="ability"))


def deck_problems(deck):
    """Deck construction checks for a deck dict (legend, champion, main, runes, battlefield[s]) or a list of names:
    cards not modelled (not in IMPL) and [Unique] cards with more than one copy (rule 825)."""
    from collections import Counter
    if isinstance(deck, dict):
        cards_ = [n for n in [deck.get("champion")] + list(deck.get("main", [])) if n]
        others = [deck.get("legend")] + list(deck.get("battlefields") or [deck.get("battlefield")])
        others += [r if r.endswith(" Rune") else r + " Rune" for r in deck.get("runes", [])]
        names = cards_ + [n for n in others if n]
    else:
        cards_ = names = list(deck)
    out = []
    for n in sorted(set(names)):
        if n not in IMPL:
            out.append(f"{n}: not modelled")
        elif is_token(n):
            out.append(f"{n}: a token can't be put in a deck (rule 187)")
    cnt = Counter(cards_)
    for n in sorted(cnt):
        if n in SPEC and "Unique" in keywords_of(n) and cnt[n] > 1:
            out.append(f"{n}: [Unique], {cnt[n]} copies")
    return out


def keywords_of(name):
    """Keywords printed on a card (bracketed in its text, reminder text removed)."""
    t = re.sub(r"\([^)]*\)", "", SPEC[name]["text"])
    return set(m.group(1) for m in re.finditer(r"\[([A-Z][A-Za-z\-]+)", t))


# ---------------------------------------------------------------------- auto-modelling of keyword-only cards
_KW_SIMPLE = ("Action", "Reaction", "Accelerate", "Hidden", "Tank", "Ganking", "Backline", "Temporary", "Ambush",
              "Unique", "Vision", "Weaponmaster", "Quick-Draw")
_KW_VALUE = ("Assault", "Shield", "Deflect", "Hunt")
_KW_TOKEN = re.compile(r"\[(%s)\]|\[(%s)(?: (\d+))?\]" % ("|".join(_KW_SIMPLE), "|".join(_KW_VALUE)))


def parse_keywords(name):
    """If the text of a card consists only of keywords and reminder text, return the Impl fields that model it,
    else None. Reminder text (parentheses) is ignored, so 'Reaction' inside Ambush reminder text doesn't count."""
    sp = SPEC.get(name)
    if sp is None:
        return None
    t = re.sub(r"\([^)]*\)", " ", sp["text"])
    fields = dict(kw={})
    rest = _KW_TOKEN.sub(lambda m: _collect(m, fields), t)
    if rest.strip(" .\n\t"):
        return None
    if not fields["kw"]:
        fields.pop("kw")
    return fields


def _collect(m, fields):
    simple, valued, n = m.group(1), m.group(2), m.group(3)
    if simple in ("Action", "Reaction"):
        fields["timing"] = simple.lower()
    elif simple == "Accelerate":
        fields["accelerate"] = True
    elif simple == "Hidden":
        fields["hidden"] = True
    elif simple == "Ambush":
        fields["ambush"] = True
    elif simple == "Quick-Draw":
        fields["quickdraw"] = True
    elif simple:
        fields["kw"][simple] = 1
    else:
        fields["kw"][valued] = fields["kw"].get(valued, 0) + int(n or 1)
    return " "


def auto_keywords(name, register=True):
    """Build (and register) the Impl of a card whose text is only keywords and reminder text
    (e.g. 'Sunlit Guardian': [Shield] [Tank]). Returns the Impl, or None if the text has other abilities."""
    fields = parse_keywords(name)
    if fields is None:
        return None
    sp = SPEC[name]
    if sp["type"] in ("Rune", "Battlefield", "Legend"):
        return None
    if sp["type"] == "Gear" and "Equipment" in sp["tags"]:
        return None                     # Might Bonus is not in the card database (EQUIP_BONUS)
    if register:
        im = card(name, **fields)
    else:
        im = Impl(name, **fields)
    im.auto = True
    return im


# Printed in the card database but not playable cards: the "Buff" reminder card and the XP Tracker.
NOT_CARDS = {"Buff", "XP Tracker"}
# Token printings without the Token supertype in the database
TOKEN_NAMES = {"Bird", "Sprite", "Gold", "Reflection", "Recruit (NX)", "Mech", "Brush"}


def is_token(name):
    """Tokens (rule 187) can't be put in a deck."""
    sp = SPEC.get(name)
    return sp is not None and (sp["super"] == "Token" or name in TOKEN_NAMES)


def register_keyword_only_cards():
    out = []
    for name in sorted(SPEC):
        if name in IMPL or name in NOT_CARDS:
            continue
        if auto_keywords(name) is not None:
            out.append(name)
    return out


def check():
    """Every card name of the pool must have an implementation."""
    import re
    names = [l.split(" | ")[0][3:] for l in open(__file__.replace("cards.py", "pool.txt"), encoding="utf-8") if l.startswith("## ")]
    return [n for n in names if n not in IMPL]


# ====================================================================== other runes and token printings
card("Body Rune"); card("Chaos Rune")
card("Gold // Buff")                                    # double-faced printing of the Gold token (Game.golds)
reflection_token = _reflection

# ====================================================================== card modules
# Names a card module gets with `from cards import *`.
__all__ = [
    "Impl", "IMPL", "card", "SPEC", "ANY", "DOMAINS", "EQUIP_BONUS", "Item", "Obj", "Opt", "play_card", "attach",
    "equip_cost", "deflect_reqs", "loc_allowed", "open_bf", "card_kw", "flow_of", "abilities_of",
    "others_enter_ready", "keyword_play_triggers", "full_choices", "cap", "total_cost", "card_choices", "pay_ctx", "ASK_TEXT", "ask_text",
    # choice / target helpers
    "value", "enemies", "friends", "all_units", "hb_ok", "tg_choices", "item_by_id", "trig_target", "may_pay",
    "play_unit_free", "make_token", "token_locations", "unit_play_choices", "reflection_token", "pay_deflect", "extra_label", "remake_choices",
    "gain_control_item",
    "P_unit", "P_unit_bf", "P_enemy", "P_enemy_bf", "P_friend", "P_friend_bf", "P_gear",
    # keyword helpers
    "parse_cost", "power_list", "repeat_cost", "flow_cost", "ability", "add_ability", "equip_ability",
    "equip_ability_cost", "empower_ability", "repeatable", "when_level", "when_empowered", "when_mighty",
    "when_legion", "when_at_bf", "keywords_of", "parse_keywords", "auto_keywords", "deck_problems", "is_token",
]


import os


def load_cardsets(package="cardsets"):
    """Import every module listed in <package>/__init__.py MODULES (explicit list: the browser loads files by
    name). Each module registers its cards with card(); registering an existing name raises ValueError.
    A missing package (older deployments) only means fewer modelled cards."""
    import importlib
    try:
        pkg = importlib.import_module(package)
    except ModuleNotFoundError as e:
        if e.name != package:
            raise
        return []
    only = os.environ.get("RB_CARDSETS")             # "fury,body": charge seulement ces modules (travail en parallèle)
    mods = [m for m in pkg.MODULES if only is None or m in only.split(",")]
    for m in mods:
        _LOADING[0] = f"{package}.{m}"
        try:
            importlib.import_module(f"{package}.{m}")
        finally:
            _LOADING[0] = None
    return mods


CARDSETS = load_cardsets()
_LOADING[0] = "auto_keywords"
AUTO = register_keyword_only_cards()                     # keyword-only cards not modelled by a module
_LOADING[0] = None
