"""Version du moteur, pour étiqueter chaque résultat de simulation (règle : ne jamais agréger deux versions).

engine_md5() est exactement le hachage de manager/run_session.py (MD5 des fichiers .py du moteur et de cardsets/,
chemin relatif + contenu, 10 caractères) : les sessions déjà enregistrées restent comparables. Il change dès qu'un
fichier .py change, commentaires compris : c'est volontairement prudent (manager/eqtest.py vérifie une équivalence)."""
import hashlib
from pathlib import Path

HERE = Path(__file__).resolve().parent


def engine_md5(root=HERE):
    root = Path(root)
    h = hashlib.md5()
    for f in sorted(root.glob("*.py")) + sorted((root / "cardsets").glob("*.py")):
        h.update(f.relative_to(root).as_posix().encode() + f.read_text(encoding="utf-8").encode())
    return h.hexdigest()[:10]


def scores(entry):
    """{graine: score} d'un résultat (per_seed des expériences et du manager, per_pair d'exp_general)."""
    rows = entry.get("per_seed") or entry.get("per_pair") or []
    return {int(r[0]): float(r[1]) for r in rows}


def stamp(entry, root=HERE):
    """Ajoute la version du moteur et le résumé des graines à un résultat (à appeler avant json.dump)."""
    entry["engine_md5"] = engine_md5(root)
    s = sorted(scores(entry))
    if s:
        entry["seeds"] = dict(first=s[0], last=s[-1], n=len(s))
    return entry
