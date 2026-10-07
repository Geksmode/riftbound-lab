---
name: riftbound-sources
description: Where Riftbound rules/card data live in /mnt/project-files/riftbound and which official URLs/APIs to use
metadata:
  type: reference
---
Files (collected 2026-10-02): /mnt/project-files/riftbound/README.md (index + gaps), rules/core-rules-resume.md, rules/tournament-rules-resume.md (incl. ban list as of 2026-09-18), cards/cards_unique.csv (938 cards OGN/SFD/UNL/VEN/OGS, with power_cost + supertype), cards/cards_all_printings.{csv,json} (1231), cards/build_cards.py (rebuild from rifthunt bulk).

Official rules (both updated 2026-07-16): https://playriftbound.com/en-us/news/rules-and-releases/gameplay-guide-core-rules/ and https://playriftbound.com/en-us/news/organizedplay/riftbound-tournament-rules/ ; per-rule HTML at https://app.riftjudge.com/rules/core and /rules/tournament. Penalties are section 700 of Tournament Rules (no separate IPG).

Card APIs (no auth): https://api.rifthunt.com/bulk/cards (bulk, reachable since 2026-10-02 network change; snapshot dated 2026-09-11) and Scryfall-like /sets, /cards/search?q=set:ven. api.riftcodex.com and api.dotgg.gg still blocked (proxy 403). Card DB is complete as of 2026-10-02. See [[riftbound-lab-goal]].

Retex source (2026-10-02): riftbound.gg is now reachable; its WordPress REST API (https://riftbound.gg/wp-json/wp/v2/posts?search=...) gives article text. Decklists/matchup winrates are embedded from api.dotgg.gg (blocked). Tool: riftbound/tools/retex_fetch.py; proposed skill "retex".
