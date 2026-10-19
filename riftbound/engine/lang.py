"""Langue des textes montrés au joueur par la table d'entraînement : « fr » (défaut) ou « en » (version anglaise).

Seuls les libellés changent (questions, choix, journal, coups décrits, erreurs de deck) : le moteur ne lit jamais ces
textes pour décider, donc la langue ne change pas la partie (même graine = même partie). La page appelle
train.set_lang("en") ; tout le reste du dépôt (tests, simulations, replays) reste en français.

Chaque nouvelle question g.ask d'un module (cards.ask_text) doit avoir son titre anglais dans ASK_EN ci-dessous :
test_lang.py le vérifie."""
LANG = "fr"


def set_lang(x):
    global LANG
    LANG = "en" if x == "en" else "fr"
    return LANG


def en():
    return LANG == "en"


def tr(fr, en_):
    """Le texte dans la langue de la table : tr(f"{n} énergie", f"{n} energy")."""
    return en_ if LANG == "en" else fr


# Titres anglais des questions (train.ASKS et cards.ASK_TEXT, même clé).
ASK_EN = dict(
    # train.ASKS
    mulligan="Mulligan: choose up to 2 cards to put back", may="Use this optional effect?",
    play_location="Where do you play the unit?", equip_target="Equip which unit?", discard="Discard which card?",
    copy_target="Copy which unit?", sacrifice="Sacrifice which unit?", recycle_rune="Recycle which rune?",
    pay_gold="Pay the power: with a Gold or by recycling a rune?",
    hook_pick="Baited Hook: which unit do you take?", zhonya_save="Zhonya: save which unit?",
    star_spring="Star Spring: which unit?", dusk_kill="Kill which unit?",
    herald_pick="Which unit?", herald_dk_pick="Which card?",
    mixologist_pick="Which unit?", ashe_pick="Which card?", target="Target:",
    damage_order="Damage order:",
    damage_pick="Combat: which unit receives its lethal damage first?",
    # cards.ASK_TEXT
    add_kill="Which unit do you kill to add the resource?",
    akshan_gear="Akshan, Mischievous: which enemy gear do you move to your base?",
    altar_card="Which card from your hand do you place?", altar_where="On top of or at the bottom of your deck?",
    aphelios_mode="Which mode?", apothecary_pick="Which unit do you heal?",
    armory_discard="Unlicensed Armory: which card do you discard?", ava_play="Which card do you play?",
    azir_attach="Which gear do you attach?", azir_move="Move this unit?",
    banish_from_trash="Which card from the trash do you banish?", bard_move="Which units do you move, and where?",
    brambleback_buff="Red Brambleback: which friendly unit do you buff?",
    buff_target="Which unit do you buff?", buhru_mode="Draw 1 or buff yourself?",
    bullet_time_x="How many runes do you pay (X)?", burn_player="Which player is affected?",
    call_to_battle="Which unit do you move to this battlefield?", choose="Choose a target:",
    choose_player="Which player discards?", clone_banish="Which unit do you banish?",
    constellation_kill="Which unit do you kill to pay?",
    corrupted_dragon="Which enemy units do you return to their base?",
    dais_return="Which unit do you return to hand?", dame_unit="Which unit here?",
    death_replace="Replace the unit's death?", decree_strength="Which revealed card do you take?",
    detach="Which gear do you detach?", discord_subset="Which enemy units do you return to hand?",
    double_pick="Which unit do you take from the cards seen?", draw_pick="Which card do you draw?",
    duel_keep="Which unit do you keep?",
    elder_dragon="Elder Dragon: which enemy unit here (up to one)?", empower_target="Which unit here?",
    entourage="Which legend, and do you ready or exhaust it?", excited_discard="Discard which card?",
    fae_dragon_buff="Fae Dragon: which friendly units do you buff (up to four)?",
    faefolk_target="Which enemy unit do you move here?", fallen_feline_name="Fallen Feline: name a spell",
    fate_pick="Which spell do you play?", fizz_pick="Which spell do you cast?",
    forge_recycle="Which card do you recycle?", fortified_shield="Which unit do you protect?",
    foxfire_subset="Which units do you target?", friend_target="Which unit do you choose?",
    friendly_target="Which unit do you target?", gear_target="Which gear do you target?",
    guerilla_return="Guerilla Warfare: which [Hidden] card do you take back?",
    harrowing_pick="Which unit from your trash do you play?",
    hatchling_recycle="Recycle the top card of the deck?", here_to_help="Which unit do you play?",
    investigator_pick="Which card from their hand do you choose?",
    ivern_pick="Which unit do you take from the cards seen?", ivern_tag="Ivern, Friend to All: which tag does it gain?",
    jayce_kill_gear="Jayce: which friendly gear do you kill?", jayce_mode="Which keyword do you gain?",
    judgment_gear="Which gear do you keep?", judgment_hand="Which cards in your hand do you keep?",
    judgment_rune="Which runes do you keep?", judgment_unit="Which units do you keep?",
    kaisa_spell="Kai'Sa: which spell do you play from your trash?", kato_target="Which friendly unit do you target?",
    kennen_flow="Kennen, Storm of Shuriken: which spell in your trash gains [Flow]?",
    kharox_pick="Which unit from their trash do you play?",
    maduli_dest="Maduli the Gatekeeper: to which battlefield?", mask_mother="Mask Mother: which friendly unit gets +2?",
    matriarch_pick="Which unit do you play to your base?", mf_ready="What do you ready: a unit or a rune?",
    minah_mode="Draw or discard?", mindsplitter_pick="Which card from the opponent's hand is discarded?",
    morgana_target="Which unit do you target?", move_enemy="Which enemy unit do you move, and where?",
    name_tag="Which champion name do you choose?", nasus_ready_rune="Nasus: which rune do you ready?",
    new_choices="Make new choices for this spell?", nocturne_play="Nocturne, Horrifying: play it for 1 rune?",
    ocean_drake="Ocean Drake: which non-Dragon unit do you return to its owner's hand?",
    ornn_pick="Which gear do you take from the cards seen?", overt_spend="Spend this unit's buff?",
    party_favors="What do you choose: cards or runes?", pay_or_countered="Pay 2 so it isn't countered?",
    play_choice="How do you play this card?", play_extra="Pay an additional cost?",
    predict_recycle="Recycle the cards looked at?", predict_top="Which card do you put back on top?",
    profiteer="Which unit do you move, and to which one?", promising_pick="Which card do you play?",
    promising_play="How do you play this card?", pursuit_attach="Which gear do you attach?",
    qiyana_mode="Draw 1 or channel 1 rune?", ready_gear="Which gear do you ready?",
    ready_pick="What do you ready?", ready_rune="Which rune do you ready?", ready_target="Which unit do you ready?",
    reaver_move="Which unit do you move?",
    rebuttal_pay="Rebuttal: pay 1 rune to take control of the spell (otherwise it is countered)?",
    recycle_from_trash="Which card from the trash do you recycle?",
    recycle_pick="Which card from the trash do you recycle?", recycle_trash="Which card from the trash do you recycle?",
    reinforce_pick="Which unit do you play?", reksai_choice="Where do you play this card?",
    reksai_pick="Which revealed card do you play?", reksai_play="Which revealed card do you play?",
    rell_pick="Which gear from your hand do you play and attach?", rengar_target="Which unit do you target?",
    resurrect_pick="Which unit from your trash do you bring back?", return_gear="Which gear do you return to hand?",
    return_pick="Which card do you return to hand?", rites_pick="Which unit from your trash do you play?",
    rocket_discard="Super Mega Death Rocket!: which card do you discard to take it back?",
    rumble_pick="Which cards do you choose (Mech to recycle, Mech to take back)?",
    rush_pick="Which card do you put into your hand?", sabotage="Which revealed card do you recycle?",
    sacrifice_gear="Which gear do you sacrifice?", shakedown_let_draw="Let the opponent draw?",
    skewer_pick="Which unit do you play on this battlefield?", solari_shrine="Solari Shrine: exhaust to draw 1?",
    soulgorger_pick="Which card from your trash do you play?",
    spectacles_copy="Shady Spectacles: which friendly unit does the equipped unit become a copy of?",
    spend_buff="Which unit spends its buff?", split_damage="How do you split the damage?",
    split_keep="Between which units do you split the damage?", stacked_pick="Which card do you put into your hand?",
    starhound_pick="Which card from your trash do you return to hand?", token_location="Where do you create the token?",
    trash_gear="Which gear do you put into the trash?", trash_spell="Which spell from your trash do you take back?",
    trash_unit="Which unit from your trash do you take back?", tricksy_subset="Which units do you choose?",
    vanguard_helm="Vanguard Helm: which other friendly unit do you buff?", veiled_detach="Detach this gear?",
    verdict_place="On top of or at the bottom of the deck?", void_rush_pick="Which card do you play?",
    weaponmaster_pick="Which gear do you attach?", whirlwind_pick="Which unit do you return to its owner's hand?",
    wild_claw="Which unit do you play?", zed_discard="Zed: which card do you discard?",
    zilean_copy="Zilean: play an extra copy of the token?",
    # Radiance (moteur : Deploy, Disarm, jetons)
    deploy_location="On which battlefield do you play this gear?", disarm_target="Disarm: which enemy unit do you weaken?",
    token_swap="Play a Mech token instead?",
    # Radiance (cardsets/radiance.py)
    bomb_location="On which battlefield do you play the Bomb token?", bomb_target="Bomb: which enemy unit does it hit?",
    tracker_targets="Which units (up to 2) take 1 damage?", amp_kill="Which other friendly card do you kill to draw 1?",
    morde_kill="Which friendly unit do you kill to save Mordekaiser?", riven_mode="Riven: which effect?",
    gear_pick="Which gear do you reveal and draw?", scope_guess="Guess the type of the top card:",
    innovator_mode="Reckless Innovator: which effect?", stranger_bf="Which battlefield do you choose?",
    stranger_return="Which enemy unit do you return to its owner's hand?",
    wyrm_move="Which unit do you move to Angler Wyrm's battlefield?",
    transpo_pick="Which unit from your hand do you play for free?", spirit_kill="Which unit or gear do you kill?",
    captain_pick="Which card do you reveal and draw?", remember_pick="Which card from your trash do you play?",
    chrono_pick="Which unit do you banish?", chrono_order="Which unit do you play back now?",
    orianna_gear="Which gear becomes a unit?", ball_mode="The Ball: which effect?",
)


def ask_title(kind, fr):
    """Titre d'une question dans la langue de la table (le français fr si la clé n'a pas d'anglais)."""
    return ASK_EN.get(kind, fr) if LANG == "en" else fr
