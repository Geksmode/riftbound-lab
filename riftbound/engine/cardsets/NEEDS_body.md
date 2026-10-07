# Batch body: cards left out of IMPL

All 128 cards of `batches/body.txt` are registered in `body.py` (tests in `test_body.py`). The 18 cards that needed
engine hooks (Arachnoid Horror, Dauntless Vanguard, Deadbloom Predator, Rengar, Trophy Hunter, Ambessa, The Wolf,
Unyielding Spirit, Elder Dragon, Determined Sentry, Jagged Cutlass, Fae Dragon, Pirate's Haven, Renekton, Brute,
Spoils of War, Wily Newtfish, Herald of Scales, Ancient Henge, Akshan, Mischievous, Gangplank, Naval) use the hooks
described in `GUIDE.md`.

## Rules decisions taken for the registered cards

- Damage that units deal to each other through a spell or ability (Challenge, Clash of Giants, Gentlemen's Duel,
  Marching Orders, Rampage, Carnivorous Snapvine, Strike Down) is dealt by the units (rule 417.6.b.3): kind
  `"unit"`, so no Void Gate bonus damage; both Mights are read before damage is dealt (simultaneous, 417.1.d).
- Volibear, Imposing: "When an opponent moves to a battlefield other than mine" triggers once per move performed by
  an opponent (units moved together count once); moves of their units performed by you don't count.
- Yone, Blademaster: "conquer an open battlefield" = a conquer that does not follow a combat (in 1v1 an empty
  battlefield loses its controller at the next cleanup, so such a battlefield was unoccupied and uncontrolled).
- Sivir, Ambitious: excess damage = damage the attacker assigned to an enemy unit beyond the lethal damage it needed
  at assignment (Might minus marked damage, at least 1, plus its prevention), summed over the enemy units.
- Riposte: if the chosen spell can't be countered or has already left the chain, the unit still gets +might equal
  to its printed Energy cost (do as much as you can, rule 055; printed cost, rule 206).
- Miss Fortune, Captain: "something else that's exhausted" includes units, gear, runes and legends (all on the
  board, rule 107); an enemy unit chosen this way must be paid for if it has [Deflect].
- Confront: "Units you play this turn enter ready" is a replacement of the way they enter (rule 369.3, `g.effects`
  kind `enter_ready`), unit tokens included.
- Mistfall: "When you buff a friendly unit" uses the player who buffs (`by` of the `buff` event).
- Rengar, Trophy Hunter: "I can be played to a battlefield where there are enemy units" extends Ambush's permissions
  (rule 822.1.d example), so these plays have Ambush's Reaction timing.
- Arachnoid Horror: "an enemy unit is alone there" = exactly one unit of the opponent at that battlefield.
- Elder Dragon: its play trigger chooses at each location (both bases and the battlefields) up to one enemy unit;
  "any amount of your damage" makes 1 combat damage lethal in assignment too (rule 465.2.c).
- Pirate's Haven: the Awaken readying is by the turn player (rule 415.3.a), so it gives +1 to each friendly unit
  readied at the start of your turn.
- Akshan, Mischievous: the gear (detached from the enemy unit) goes to your base under your control until Akshan
  leaves the board (then its previous controller gets it back, unattached, at its base); if Akshan has already left
  the board when the trigger resolves, nothing happens (the duration is over).
- Gangplank, Naval: the "+3 might instead" is permanent, except when it replaces a -might with a duration ("this
  turn"), whose duration it inherits (rule 375).
- Trinity Force: "score 1 point" gains a point that is not from Conquer, so the Final Point restriction doesn't
  apply (rule 471.1.a.1).
- Gemhand Hunter: the trailing "ambush" in the card database is not on the card (checked on the image).
- Equipment Might Bonus read on the images: Boneshiver +2, Doran's Blade +2, Hexdrinker +1 (Effect Text [Deflect]),
  Hunter's Machete +2 ([Hunt]), Trinity Force +2, Warmog's Armor +1 ("When I conquer, buff me."), Jagged Cutlass +2.
