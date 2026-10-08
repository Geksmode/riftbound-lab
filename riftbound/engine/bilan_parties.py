#!/usr/bin/env python3
"""Bilan des parties de l'utilisateur, à partir de train/analyses/choices.jsonl (écrit par train_games.py --frozen --coach)
et des parties elles-mêmes (train/games/games/*.json). Écrit train/analyses/BILAN.md (et l'imprime).

  python3 bilan_parties.py [dossier des parties] [choices.jsonl]

Faits seulement : résultats, parties inachevées et leur score au moment de l'arrêt, accord avec le coach (l'IA Akali)
par type de décision, rythme (premier Akali, Deadly Weapon, première entrée sur un battlefield, score aux tours 7 et 11).
Le coach n'est pas la vérité : une décision différente du coach n'est pas une erreur."""
import collections, glob, json, math, os, sys

HERE = os.path.dirname(os.path.abspath(__file__))
GAMES = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "..", "train", "games", "games")
CHOICES = sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, "..", "train", "analyses", "choices.jsonl")
OUT = os.path.join(os.path.dirname(CHOICES), "BILAN.md")


def kind_of(label):
    l = label or ""
    if l.startswith("joue"):
        return "jouer une carte"
    if l.startswith("déplace"):
        return "déplacer"
    if l.startswith(("termine", "passe")):
        return "passer / finir"
    if l.startswith("cache"):
        return "cacher"
    return "légende / capacité"


def main():
    meta = {}
    for f in sorted(glob.glob(os.path.join(GAMES, "*.json"))):
        d = json.load(open(f))
        for g in (d if isinstance(d, list) else [d]):
            g = g.get("data", g)
            if isinstance(g, dict) and "moves" in g:
                meta[g["id"]] = g
    rows = [json.loads(l) for l in open(CHOICES)] if os.path.exists(CHOICES) else []
    by = collections.defaultdict(list)
    for r in rows:
        by[r["game"]].append(r)
    L = ["# Bilan de tes parties", "", f"{len(meta)} parties ; rejeu : `train_games.py --frozen --coach` ; décisions : {len(rows)}.", ""]
    L += ["| Partie | Toi contre | Résultat | Rejeu fidèle | Moteur | Même coup que le coach |", "|---|---|---|---|---|---|"]
    wins = losses = 0
    stopped = []
    for gid in sorted(meta):
        g, rs = meta[gid], by.get(gid, [])
        n = g.get("names") or ["Akali", "LeBlanc"]
        res = g.get("result")
        if res:
            wins += res.get("winner") == 0
            losses += res.get("winner") == 1
            rtxt = ("victoire " if res.get("winner") == 0 else "défaite " if res.get("winner") == 1 else "nulle ") + "-".join(map(str, res.get("pts", [])))
        else:
            pts = rs[-1]["state"]["pts"] if rs else g.get("pts")
            stopped.append(pts)
            rtxt = f"non finie ({'-'.join(map(str, pts)) if pts else '?'})"
        co = [r for r in rs if r.get("coach")]
        same = sum(1 for r in co if r["coach"] == r["label"])
        fid = "oui" if rs and all(r.get("faithful") for r in rs) else "NON" if rs else "?"
        L.append(f"| {gid} | {n[1] if len(n) > 1 else '?'} (toi : {n[0]}) | {rtxt} | {fid} | {rs[0].get('engine', '?') if rs else '?'} | {same}/{len(co)} |")
    fin = wins + losses
    L += ["", "## Résultats", f"- Parties finies : {fin} ({wins} victoires, {losses} défaites)."]
    if fin:
        p, z = wins / fin, 1.96                      # intervalle de Wilson à 95 % (juste aussi à 0 % ou 100 %)
        c = (p + z * z / (2 * fin)) / (1 + z * z / fin)
        h = z * math.sqrt(p * (1 - p) / fin + z * z / (4 * fin * fin)) / (1 + z * z / fin)
        L.append(f"- Taux de victoire sur les parties finies : {100 * p:.0f} %, intervalle de Wilson à 95 % : "
                 f"[{100 * max(0, c - h):.0f} %, {100 * min(1, c + h):.0f} %].")
    if stopped:
        behind = sum(1 for s in stopped if s and s[0] < s[1])
        L.append(f"- Parties inachevées : {len(stopped)}, dont {behind} arrêtées en étant mené au score : les compter comme des défaites "
                 f"donnerait au plus {wins}/{fin + len(stopped)} victoires. **Biais de sélection possible** : ne pas lire le taux ci-dessus comme ton niveau réel.")
    M = collections.Counter()
    for r in rows:
        if r.get("coach") is not None and r["k"] == "act":
            M[(kind_of(r["label"]), kind_of(r["coach"]))] += 1
    if M:
        you, coach = collections.Counter(), collections.Counter()
        for (a, b), n in M.items():
            you[a] += n
            coach[b] += n
        L += ["", "## Tes décisions comparées au coach (IA Akali)", "| Type | Toi | Coach |", "|---|---|---|"]
        for k in sorted(set(you) | set(coach), key=lambda k: -you[k]):
            L.append(f"| {k} | {you[k]} | {coach[k]} |")
        L += ["", "Les écarts les plus fréquents (toi → coach) :"]
        for (a, b), n in sorted(M.items(), key=lambda kv: -kv[1]):
            if a != b and n >= 5:
                L.append(f"- {a} → {b} : {n} fois")
    L += ["", "## Rythme", "| Partie | 1er Akali, Deadly Weapon (tour) | 1re entrée sur un battlefield (tour) | cartes cachées | score au tour 7 | au tour 11 |", "|---|---|---|---|---|---|"]
    for gid in sorted(meta):
        rs = by.get(gid, [])
        dw = next((r["state"]["t"] for r in rs if r["label"].startswith("joue Akali, Deadly Weapon")), "—")
        mv = next((r["state"]["t"] for r in rs if r["label"].startswith("déplace") and "base" not in r["label"].split("vers")[-1]), "—")
        hide = sum(1 for r in rs if r["label"].startswith("cache"))
        sc = {t: next(("-".join(map(str, r["state"]["pts"])) for r in rs if r["state"]["t"] >= t), "—") for t in (7, 11)}
        L.append(f"| {gid} | {dw} | {mv} | {hide} | {sc[7]} | {sc[11]} |")
    txt = "\n".join(L) + "\n"
    open(OUT, "w").write(txt)
    print(txt)


if __name__ == "__main__":
    main()
