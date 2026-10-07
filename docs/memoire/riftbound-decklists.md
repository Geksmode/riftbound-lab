---
name: riftbound-decklists
description: Where parsed Riftbound tournament decklists live, how they were collected (deck sites blocked), and how to add more
metadata:
  type: project
  modified: 2026-10-02T14:41:49.699Z
---
Since 2026-10-02: /mnt/project-files/riftbound/decks/ holds raw/*.txt decklists (one per deck, header + [legend]/[champion]/[main]/[runes]/[battlefields]/[sideboard]), parse_decks.py (joins to cards_unique.csv, outputs decks.json, deck_cards.csv, card_frequency.csv, validation.txt) and matchups/*.md analyses. See decks/README.md.

Network: riftdecks.com, riftbound.gg, riftools.app, mobalytics.gg, piltoverarchive.com, riftmana.com, magicalmeta.ink, hextechanalytics.com are blocked (curl and WebFetch, proxy 403/EGRESS_BLOCKED) as of 2026-10-02. Only WebSearch works; its result summaries often contain full decklists when the query names the riftdecks deck (player + legend + "decklist"). Summaries can be wrong on quantities, so the parser checks 40/12/3. User can allow hosts in Project settings > environment network.

Sample on 2026-10-02: 6 Akali, Rogue Assassin + 6 LeBlanc, Deceiver lists. Related: [[riftbound-sources]], [[akali-vs-leblanc]].
