# Audit du texte imprimé : cartes à choix facultatif

Règle de référence : **355.13** des Core Rules (2026-07-16) — « any number » / « up to N » : on peut choisir zéro
cible, et le sort ou la capacité se joue alors sans cible. Autres règles utiles : 355.10 (ce qui est une cible),
355.11 (groupes de cibles), 355.14 (dégâts répartis), 359.3.e (un effet s'applique autant que possible).

Colonnes : **corrigé** = écart trouvé (test écrit d'abord, il échouait), puis correctif ; **ok** = conforme, test
ajouté ; **à vérifier** = limite connue, détaillée dans la dernière colonne. Les tests sont dans
`test_audit_texte.py` (sorts), `test_audit_unites.py` (unités, capacités) et `test_audit_then.py` (« then »).

## 1. « up to » / « any number » (32 cartes du CSV `cards_unique.csv`)

| Carte | Formulation | Comportement attendu | Test | Statut |
|---|---|---|---|---|
| Singularity | Deal 6 to each of up to two units | zéro cible jouable, max 2, n'importe quelles unités (amies aussi pour un humain), une cible disparue n'empêche pas l'autre | `singularity_zero_target`, `singularity_max_two_and_any_unit`, `singularity_one_target_gone_other_still_hit` | **corrigé** (pas de choix à zéro cible ; un humain ne pouvait viser que des ennemis) |
| Fox-Fire | Kill any number of units at a battlefield with total Might 4 or less | zéro cible, total ≤ 4, un seul champ de bataille, unités amies permises, 355.11.b au résultat | `foxfire_zero_target`, `foxfire_total_might_and_any_unit` (+ `fox_fire_subset_when_total_grows` existant) | **corrigé** (zéro cible ; humain : unités amies) |
| Bellows Breath | [Repeat] Deal 1 to up to three units at the same location | zéro cible, max 3, même lieu | `bellows_zero_target`, `bellows_max_three_same_location` | **corrigé** (zéro cible) |
| Tricksy Tentacles | Move any number of enemy units … total Might 8 or less to a single location | zéro cible, total ≤ 8 | `tricksy_zero_target`, `tricksy_total_might_max_8` | **corrigé** (zéro cible) |
| Disposal Order | Choose one — choose up to 3 cards from opponents' trashes, recycle / Draw 1 | le mode « recycler » avec zéro carte est un choix, max 3 | `disposal_order_recycle_zero_cards`, `disposal_order_max_three` | **corrigé** (zéro carte absent des choix) |
| Volibear, Furious | deal 5 damage split among any number of enemy units here | zéro cible permis, max 5 cibles, ≥ 1 dégât chacune (355.14) | `volibear_zero_targets_is_an_option`, `volibear_at_most_five_targets_each_at_least_one` | **corrigé** (zéro cible ; libellé « aucune cible » dans `train.py`) |
| Fae Dragon | buff up to four friendly units | cibles choisies à la mise sur la chaîne (355.10, événement `chosen`), zéro permis, max 4 | `fae_dragon_zero`, `fae_dragon_max_four`, `fae_dragon_buffed_units_are_chosen_as_targets` | **corrigé** (les unités étaient choisies à la résolution, sans événement `chosen` : Spirit Wheel, Irelia… ne réagissaient pas) |
| Shuriken Flip | Deal 2 to up to one enemy unit at a battlefield, then move a friendly unit | jouable sans cible, le déplacement s'applique quand même | `flip_no_target_still_moves` | ok |
| Piercing Light | Deal 2 to a unit at a battlefield, then deal 2 to up to one other unit | 2e cible facultative, la 2e partie s'applique si la 1re cible a disparu, injouable sans 1re cible | `piercing_light_second_target_optional`, `piercing_light_no_first_target_not_playable`, `piercing_light_second_part_applies_when_first_target_is_gone` | ok |
| Emperor's Divide | Move any number of friendly units at a battlefield to their base | zéro cible | `divide_zero_target` | ok |
| Flash | Move up to 2 friendly units to base | zéro cible, max 2 (IA et humain) | `flash_zero_target_and_max_two` | ok |
| Decree of Discord | Return any number of enemy Order units with total Might 5 or less | zéro cible | `decree_zero_target` | ok |
| Shadows of the Past | Return up to 2 units from trashes to their owners' hands | zéro carte, max 2 | `shadows_zero_target`, `shadows_max_two` | ok |
| Moonfall | You may move up to one enemy unit to that battlefield. Then give enemy units there −2 might | zéro cible, le −2 s'applique quand même, injouable sans unité à soi sur un champ de bataille | `moonfall_zero_target_still_shrinks`, `moonfall_needs_own_units` | ok |
| Guerilla Warfare | Return up to two cards with [Hidden] from your trash | zéro carte, max 2, « hide gratuit » appliqué quand même | `guerilla_zero_cards_still_hides_free`, `guerilla_returns_at_most_two` | ok |
| Acceleration Gate | Ready up to 4 units, gear, and/or runes | zéro cible jouable, max 4 | `gate_zero_target_and_max_four` | ok pour l'IA ; **à vérifier** pour un humain : seules 3 combinaisons (4, 2 ou 0 objets) sont proposées, il ne peut pas choisir lesquels |
| Kinkou Monk | When you play me, buff up to two other friendly units | zéro, max 2, jamais lui-même, jouable seul | `monk_zero_other_units`, `monk_alone_still_playable`, `monk_max_two_others_not_self` | ok |
| Elder Dragon | choose up to one enemy unit at each location. Deal 1 to them | zéro, une seule par lieu | `elder_dragon_one_per_location`, `elder_dragon_zero` | ok |
| Corrupted Dragon | you may move any number of enemy units here each with 5 might or less to their base | zéro, plusieurs, pas de 6+ might | `corrupted_dragon_zero`, `corrupted_dragon_several_small_not_big` | ok |
| Azir, Sovereign | you may move any number of your token units to this battlefield | zéro, tous les jetons, jamais les autres | `azir_zero_tokens_moved`, `azir_moves_all_tokens_only_tokens` | ok |
| Bard, Mercurial | move any number of your units to an open battlefield | zéro unité déplacée | `bard_zero_units_moved` | ok |
| Albus Ferros | spend any number of buffs; for each, channel 1 rune exhausted | zéro, n renforts → n runes | `albus_zero_buffs`, `albus_n_buffs_n_runes` | ok |
| Kraken Hunter | spend any number of buffs as additional cost; −1 body rune each | 0, 1, 2 renforts offerts, réduction exacte | `kraken_hunter_spend_zero_one_two` | ok |
| Commander Ledros | kill any number of friendly units as additional cost; −1 order rune each | 0 à n unités sacrifiées, réduction exacte | `ledros_kill_zero_and_n` | ok |
| Forge of the Future | Kill this: Recycle up to 4 cards from trashes | zéro, max 4, les deux défausses | `forge_recycle_zero`, `forge_recycle_max_four` | ok |
| Gentle Gemdragon | ready up to 2 runes | au plus 2 | `gemdragon_ready_at_most_two_runes` | ok, avec une nuance : l'effet prépare toujours le maximum de runes (jamais moins) et les runes ne sont pas « choisies » comme cibles (355.10) ; aucune carte ne réagit au choix d'une rune, et préparer plus ne peut pas nuire → équivalent, laissé tel quel |
| Hwei, Brooding Painter | Gear — Ready up to 2 runes | au plus 2 | `hwei_discard_spell_draws_gear_readies` (existant, test_mind) | ok (même nuance que Gemdragon) |
| Nasus, Curator of the Sands | you may exhaust me to ready up to 2 runes | zéro rune permis | `nasus_may_ready_zero_runes` | ok |
| Baited Hook | Might up to 1 more than the killed unit | borne ≤ tuée + 1 | `baited_hook_might_limit` | ok |
| Lee Sin, Ascetic | I can have any number of buffs | plusieurs renforts | `lee_sin_ascetic_stacks_buffs` | ok |
| Kayle, Justified | I can be [Empowered] up to three times | 3 fois max, Deflect 3 + Ganking à 3 | `kayle_empower_max_three` | ok |
| Spiderling | Your deck can have any number of cards named Spiderling | le deck en accepte 12 | `spiderling_any_number_in_deck` | ok pour le moteur (`deck_problems`) ; l'éditeur de deck de `train/` n'a pas été vérifié |

## 2. « then » dont la première partie peut ne rien faire (14 tests, 359.3.e)

| Carte | Formulation | Comportement attendu | Test | Statut |
|---|---|---|---|---|
| Lunar Boon, Evershade Stalker, Traveling Merchant | Discard 1, then draw | main vide : on pioche quand même | `lunar_boon_*`, `evershade_stalker_*`, `traveling_merchant_*` | ok |
| Invert Timelines | Each player discards their hand, then draws 4 | mains vides : 4 cartes chacun | `invert_timelines_empty_hands_draw_four_each` | ok |
| Janna, Savior | heal your units here, then move an enemy unit from here | soin sans ennemi à déplacer | `janna_heals_without_enemy_to_move` | ok |
| Showstopper | Buff a friendly unit in your base, then move it | unité déjà renforcée : elle bouge quand même | `showstopper_buffed_unit_still_moves` | ok |
| Void Assault | Move a friendly unit, then move an enemy unit | 1re cible disparue : la 2e partie s'applique | `void_assault_second_part_without_first_target` | ok |
| Arise! | Play a Sand Soldier for each Equipment. Then ready two of them | 0, 1 ou 3 équipements | `arise_*` (3 tests) | ok |
| Overt Operation | you may spend its buff to ready it. Then buff all | refuser est permis, puis tout est renforcé | `overt_operation_*` | ok |
| Lacerate | disempower it. Then kill it if it has 3 might or less | le seuil se mesure après la désactivation | `lacerate_disempower_then_kill_check_uses_new_might` | ok |
| Right of Conquest | Draw 1, then draw 1 per battlefield | 0 champ de bataille : 1 carte | `right_of_conquest_zero_battlefields` | ok |

Lues sans test dédié (code conforme à la lecture) : Scrapyard Champion, Undercover Agent, Zaun Warrens, Isolate,
Peak Guardian, Portal Rescue, Arcane Shift, Gentlemen's Duel, En Garde.

## 3. Reste à faire

- « then » restantes (≈ 35) : Blind Fury, Reinforce, Kai'Sa Evolutionary, Promising Future, Teemo, Unchecked Power,
  Twisted Fate, Stormbringer, Dragon's Rage, Call to Battle, Strike Down, Alpha Strike, Ornn Blacksmith, Ezreal
  Prodigy, Altar of Memories, Rek'Sai (×2), Ivern, Diana Lunari, Scryer's Bloom, Jhin Virtuoso, Thrill of the Hunt,
  Death from Below, Temporal Breach, Dame, Wild Claw, Kharox, Zed Master of Shadows, Endless Riches, Void Hatchling.
- « you may » : 247 cartes non auditées.
- Acceleration Gate côté humain (choix des objets à préparer) ; éditeur de deck (Spiderling).
- Pour l'IA : un choix « zéro cible » existe désormais pour Singularity, Fox-Fire, Bellows Breath, Tricksy Tentacles,
  Disposal Order et Volibear. Il est placé en dernier ; l'effet sur l'IA de recherche (jouer une carte sans cible) n'a pas été mesuré.
